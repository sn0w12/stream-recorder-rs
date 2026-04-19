pub mod ffmpeg;
pub mod storage;
pub mod thumb;

use super::{StreamResult, types::StreamInfo};
use crate::cli::upload::try_upload;
use crate::config::Config;
use crate::stream::messages::{
    send_minimum_duration_webhook, send_recording_complete_webhook, send_template_webhook,
};
use crate::template::{TemplateValue, get_template_string, render_template};
use crate::types::{DurationValue, FileSize};
use crate::uploaders::build_uploaders;
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thumb::create_video_thumbnail_grid;
use tiny_table::{Cell, Column, ColumnWidth, Table};

#[derive(Clone)]
struct RecordingFile {
    path: PathBuf,
    modified: SystemTime,
    user_key: String,
}

/// Post-processes a complete recording session that may consist of multiple files.
pub async fn post_process_session(
    stream_info: StreamInfo,
    session_files: Vec<String>,
) -> StreamResult<()> {
    if session_files.is_empty() {
        return Ok(());
    }

    if session_files.len() == 1 {
        let file = session_files
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("expected a single session file"))?;
        return post_process_stream(stream_info, file).await;
    }

    println!(
        "Combining {} stream segments for {}...",
        session_files.len(),
        stream_info.username
    );

    match concat_video_files(&session_files).await {
        Ok(combined_path) => {
            println!(
                "Successfully combined stream segments into: {}",
                combined_path
            );

            for file in &session_files {
                if let Err(error) = tokio::fs::remove_file(file).await {
                    eprintln!("Failed to delete segment {}: {}", file, error);
                }
            }

            post_process_stream(stream_info, combined_path).await
        }
        Err(error) => {
            eprintln!(
                "Failed to combine stream segments ({}), processing files individually...",
                error
            );

            for file in session_files {
                if let Err(post_process_error) =
                    post_process_stream(stream_info.clone(), file).await
                {
                    eprintln!("Error post-processing segment: {}", post_process_error);
                }
            }

            Ok(())
        }
    }
}

async fn post_process_stream(stream_info: StreamInfo, output_path: String) -> StreamResult<()> {
    println!("Post-processing recorded stream: {}", output_path);

    storage::manage_disk_space().await?;

    let (file_size, duration) = get_video_metadata(&output_path).await?;
    let config = Config::get();

    if let Some(min_duration) = config.get_min_stream_duration()
        && handle_minimum_duration(
            &output_path,
            duration,
            min_duration,
            config.get_discord_webhook_url(),
            stream_info.clone(),
        )
        .await?
    {
        return Ok(());
    }

    let duration_str = format_duration(duration);
    let config = Config::get();
    let webhook_url = config.get_discord_webhook_url();
    if let Err(error) =
        send_recording_complete_webhook(webhook_url, &stream_info, &duration_str, &file_size).await
    {
        eprintln!("Error sending recorded webhook: {}", error);
    }

    let thumbnail_path = generate_thumbnail(&output_path).await;
    let upload_results = upload_recording(&stream_info, &output_path).await;
    print_upload_results(&upload_results);

    let template_info = TemplateInfo {
        output_path: output_path.clone(),
        thumbnail_path: thumbnail_path.clone(),
        upload_urls: upload_results,
        duration: duration_str,
        file_size,
    };
    send_template_notification(&stream_info, &template_info).await;

    Ok(())
}

async fn concat_video_files(files: &[String]) -> StreamResult<String> {
    if files.is_empty() {
        return Err("cannot concatenate an empty segment list".into());
    }

    let concat_manifest = build_ffconcat_manifest(files)?;
    let first_path = Path::new(&files[0]);
    let parent_dir = first_path
        .parent()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    let file_stem = first_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "combined".to_string());
    let combined_path = format!("{}/{}_combined.mp4", parent_dir, file_stem);

    let mut manifest_file = tempfile::Builder::new()
        .prefix("stream-recorder-")
        .suffix(".ffconcat")
        .tempfile_in(&parent_dir)?;
    manifest_file.write_all(concat_manifest.as_bytes())?;
    manifest_file.flush()?;

    let manifest_path = manifest_file.path().to_string_lossy().into_owned();

    let args = vec![
        "-loglevel".to_string(),
        "error".to_string(),
        "-f".to_string(),
        "concat".to_string(),
        "-safe".to_string(),
        "0".to_string(),
        "-i".to_string(),
        manifest_path,
        "-c".to_string(),
        "copy".to_string(),
        "-y".to_string(),
        combined_path.clone(),
    ];

    let output = ffmpeg::run_ffmpeg_output(&args).await?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "ffmpeg concat failed for {} segment(s): {}",
            files.len(),
            error
        )
        .into());
    }

    Ok(combined_path)
}

fn build_ffconcat_manifest(files: &[String]) -> StreamResult<String> {
    let mut manifest = String::from("ffconcat version 1.0\n");

    for file in files {
        let canonical_path = std::fs::canonicalize(file).map_err(|error| {
            format!("segment file missing or inaccessible '{}': {}", file, error)
        })?;
        let escaped_path = canonical_path
            .to_string_lossy()
            .replace('\\', "/")
            .replace('\'', r"'\''");
        manifest.push_str(&format!("file '{}'\n", escaped_path));
    }

    Ok(manifest)
}

async fn generate_thumbnail(output_path: &str) -> String {
    let config = Config::get();
    let thumbnail_path = output_path.replace(".mp4", "_thumb.jpg");

    if let Err(error) = create_video_thumbnail_grid(
        Path::new(output_path),
        Path::new(&thumbnail_path),
        &config.get_thumbnail_size(),
        &config.get_thumbnail_grid(),
    )
    .await
    {
        eprintln!("Failed to generate thumbnail: {}", error);
    }

    thumbnail_path
}

async fn upload_recording(
    stream_info: &StreamInfo,
    output_path: &str,
) -> HashMap<String, Vec<String>> {
    let config = Config::get();
    let max_retries = config.get_max_upload_retries();
    let mut upload_results = HashMap::new();

    let uploaders = build_uploaders().await;
    for (uploader, uploader_config) in &uploaders {
        let mut uploader_settings = uploader_config.clone();
        match uploader.get_folder_id_by_name(&stream_info.username).await {
            Ok(Some(folder_id)) => uploader_settings.folder_id = Some(folder_id),
            Ok(None) => {}
            Err(_) => {}
        }

        try_upload(
            uploader.as_ref(),
            output_path,
            &uploader_settings,
            &mut upload_results,
            max_retries,
        )
        .await;
    }

    upload_results
}

fn print_upload_results(upload_results: &HashMap<String, Vec<String>>) {
    let mut upload_table = Table::with_columns(vec![
        Column::new("Uploader").max_width(0.2),
        Column::new("URLs").max_width(ColumnWidth::fill()),
    ]);

    for (uploader, urls) in upload_results {
        upload_table.add_row(vec![Cell::new(uploader), Cell::new(urls.join(", "))]);
    }

    upload_table.print();
}

struct TemplateInfo {
    output_path: String,
    thumbnail_path: String,
    upload_urls: HashMap<String, Vec<String>>,
    duration: String,
    file_size: FileSize,
}

async fn send_template_notification(stream_info: &StreamInfo, template_info: &TemplateInfo) {
    let Ok(Some(template)) = get_template_string() else {
        return;
    };

    let mut context: HashMap<String, TemplateValue> = HashMap::new();
    context.insert(
        "date".to_string(),
        TemplateValue::String(Utc::now().format("%Y-%m-%d").to_string()),
    );
    context.insert(
        "username".to_string(),
        TemplateValue::String(stream_info.username.clone()),
    );
    context.insert(
        "output_path".to_string(),
        TemplateValue::String(template_info.output_path.clone()),
    );
    context.insert(
        "thumbnail_path".to_string(),
        TemplateValue::String(template_info.thumbnail_path.clone()),
    );
    context.insert(
        "stream_title".to_string(),
        TemplateValue::String(
            stream_info
                .extracted
                .stream_title
                .clone()
                .unwrap_or_default(),
        ),
    );
    context.insert(
        "duration".to_string(),
        TemplateValue::String(template_info.duration.clone()),
    );
    context.insert(
        "file_size".to_string(),
        TemplateValue::String(format!("{}", template_info.file_size)),
    );

    for (uploader, urls) in &template_info.upload_urls {
        context.insert(
            format!("{}_urls", uploader),
            TemplateValue::Array(urls.clone()),
        );
    }

    let content = format!("```\n{}\n```", render_template(&template, &context));
    let config = Config::get();
    let webhook_url = config.get_discord_webhook_url();
    if let Err(error) = send_template_webhook(
        webhook_url,
        stream_info,
        &content,
        &template_info.thumbnail_path,
    )
    .await
    {
        eprintln!("Error sending template webhook: {}", error);
    }
}

async fn get_video_metadata(output_path: &str) -> StreamResult<(FileSize, DurationValue)> {
    let metadata = std::fs::metadata(output_path)?;
    let file_size_bytes = metadata.len();
    let file_size = FileSize::from_bytes(file_size_bytes);

    let duration = match tokio::process::Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            output_path,
        ])
        .output()
        .await
    {
        Ok(output) => {
            if let Ok(json) = serde_json::from_slice::<Value>(&output.stdout) {
                let seconds = json["format"]["duration"]
                    .as_str()
                    .unwrap_or("0")
                    .parse::<f64>()
                    .unwrap_or(0.0);
                DurationValue::from_secs_f64(seconds).unwrap_or(DurationValue::ZERO)
            } else {
                DurationValue::ZERO
            }
        }
        Err(_) => DurationValue::ZERO,
    };

    Ok((file_size, duration))
}

fn format_duration(duration: DurationValue) -> String {
    duration.to_string()
}

async fn handle_minimum_duration(
    output_path: &str,
    duration: DurationValue,
    min_duration: std::time::Duration,
    webhook_url: Option<&str>,
    stream_info: StreamInfo,
) -> StreamResult<bool> {
    if duration < min_duration {
        println!(
            "Stream duration ({}) is below minimum threshold ({}), removing files without processing",
            duration,
            DurationValue::from(min_duration)
        );
        let output_path_buf = Path::new(output_path);

        send_minimum_duration_webhook(webhook_url, &stream_info).await?;
        storage::delete_recording_assets(output_path_buf).await;
        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{build_ffconcat_manifest, concat_video_files};

    fn ffmpeg_is_available() -> bool {
        std::process::Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn create_test_segment(path: &std::path::Path) {
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=16x16:r=15:d=0.2",
                "-f",
                "lavfi",
                "-i",
                "anullsrc=r=44100:cl=mono",
                "-shortest",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-movflags",
                "+faststart",
                "-y",
            ])
            .arg(path)
            .status()
            .expect("failed to spawn ffmpeg for test segment generation");

        assert!(status.success(), "ffmpeg failed to generate test segment");
    }

    #[test]
    fn build_ffconcat_manifest_uses_canonical_absolute_paths() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let segment_path = temp_dir.path().join("segment one.mp4");
        std::fs::write(&segment_path, b"test").expect("failed to create segment file");

        let manifest = build_ffconcat_manifest(&[segment_path.to_string_lossy().to_string()])
            .expect("manifest generation should succeed");

        let canonical_path = std::fs::canonicalize(&segment_path)
            .expect("failed to canonicalize test segment path")
            .to_string_lossy()
            .replace('\\', "/");

        assert_eq!(
            manifest,
            format!("ffconcat version 1.0\nfile '{}'\n", canonical_path)
        );
    }

    #[test]
    fn build_ffconcat_manifest_escapes_single_quotes() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let quoted_dir = temp_dir.path().join("creator's archive");
        std::fs::create_dir_all(&quoted_dir).expect("failed to create quoted directory");
        let segment_path = quoted_dir.join("segment.mp4");
        std::fs::write(&segment_path, b"test").expect("failed to create segment file");

        let manifest = build_ffconcat_manifest(&[segment_path.to_string_lossy().to_string()])
            .expect("manifest generation should succeed");

        assert!(manifest.contains("creator'\\''s archive/segment.mp4"));
    }

    #[test]
    fn build_ffconcat_manifest_reports_missing_segment() {
        let err = build_ffconcat_manifest(&["missing-segment.mp4".to_string()])
            .expect_err("missing files should fail manifest generation");

        assert!(err.to_string().contains("missing-segment.mp4"));
    }

    #[tokio::test]
    async fn concat_video_files_combines_two_small_temp_files() {
        if !ffmpeg_is_available() {
            eprintln!("Skipping concat test because ffmpeg is not available");
            return;
        }

        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let first_segment = temp_dir.path().join("part1.mp4");
        let second_segment = temp_dir.path().join("part2.mp4");

        create_test_segment(&first_segment);
        create_test_segment(&second_segment);

        let combined_path = concat_video_files(&[
            first_segment.to_string_lossy().to_string(),
            second_segment.to_string_lossy().to_string(),
        ])
        .await
        .expect("concat should succeed for matching temp segments");

        let combined_metadata =
            std::fs::metadata(&combined_path).expect("combined output should be written to disk");

        assert!(
            combined_metadata.len() > 0,
            "combined output should not be empty"
        );
    }
}
