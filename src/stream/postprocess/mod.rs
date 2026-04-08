pub mod storage;
pub mod thumb;

use super::{StreamResult, types::StreamInfo};
use crate::cli::upload::try_upload;
use crate::config::Config;
use crate::print::table::{Cell, Table};
use crate::stream::messages::{
    send_minimum_duration_webhook, send_recording_complete_webhook, send_template_webhook,
};
use crate::template::{TemplateValue, get_template_string, render_template};
use crate::uploaders::build_uploaders;
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thumb::create_video_thumbnail_grid;

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
        return post_process_stream(stream_info, session_files.into_iter().next().unwrap()).await;
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

    let (file_size_mb, duration_minutes) = get_video_metadata(&output_path).await?;
    let config = Config::get();

    if let Some(min_duration) = config.get_min_stream_duration()
        && handle_minimum_duration(
            &output_path,
            duration_minutes,
            min_duration,
            config.get_discord_webhook_url(),
            stream_info.clone(),
        )
        .await?
    {
        return Ok(());
    }

    let duration_str = format_duration(duration_minutes);
    let size_str = format_file_size(file_size_mb);
    let webhook_url = Config::get().get_discord_webhook_url();
    if let Err(error) =
        send_recording_complete_webhook(webhook_url, &stream_info, &duration_str, &size_str).await
    {
        eprintln!("Error sending recorded webhook: {}", error);
    }

    let thumbnail_path = generate_thumbnail(&output_path).await;
    let upload_results = upload_recording(&stream_info, &output_path).await;
    print_upload_results(&upload_results);
    send_template_notification(&stream_info, &output_path, &thumbnail_path, &upload_results).await;

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

    let output = tokio::process::Command::new("ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            &manifest_path,
            "-c",
            "copy",
            "-y",
            &combined_path,
        ])
        .stderr(std::process::Stdio::piped())
        .output()
        .await?;

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
    let mut upload_table = Table::new();
    upload_table.set_headers(vec![Cell::new("Uploader"), Cell::new("URLs")]);

    for (uploader, urls) in upload_results {
        upload_table.add_row(vec![Cell::new(uploader), Cell::new(urls.join(", "))]);
    }

    upload_table.print();
}

async fn send_template_notification(
    stream_info: &StreamInfo,
    output_path: &str,
    thumbnail_path: &str,
    upload_results: &HashMap<String, Vec<String>>,
) {
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
        TemplateValue::String(output_path.to_string()),
    );
    context.insert(
        "thumbnail_path".to_string(),
        TemplateValue::String(thumbnail_path.to_string()),
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
    for (uploader, urls) in upload_results {
        context.insert(
            format!("{}_urls", uploader),
            TemplateValue::Array(urls.clone()),
        );
    }

    let content = format!("```\n{}\n```", render_template(&template, &context));
    let webhook_url = Config::get().get_discord_webhook_url();
    if let Err(error) = send_template_webhook(
        webhook_url,
        stream_info,
        &content,
        thumbnail_path.to_string(),
    )
    .await
    {
        eprintln!("Error sending template webhook: {}", error);
    }
}

async fn get_video_metadata(output_path: &str) -> StreamResult<(f64, f64)> {
    let metadata = std::fs::metadata(output_path)?;
    let file_size_bytes = metadata.len();
    let file_size_mb = file_size_bytes as f64 / (1024.0 * 1024.0);

    let duration_minutes = match tokio::process::Command::new("ffprobe")
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
                json["format"]["duration"]
                    .as_str()
                    .unwrap_or("0")
                    .parse::<f64>()
                    .unwrap_or(0.0)
                    / 60.0
            } else {
                0.0
            }
        }
        Err(_) => 0.0,
    };

    Ok((file_size_mb, duration_minutes))
}

fn format_file_size(file_size_mb: f64) -> String {
    let file_size_gb = file_size_mb / 1024.0;
    if file_size_gb >= 1.0 {
        format!("{:.2} GB", file_size_gb)
    } else {
        format!("{:.2} MB", file_size_mb)
    }
}

fn format_duration(duration_minutes: f64) -> String {
    let hours = (duration_minutes / 60.0).floor() as u32;
    let mins = (duration_minutes % 60.0).round() as u32;
    if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

async fn handle_minimum_duration(
    output_path: &str,
    duration_minutes: f64,
    min_duration: f64,
    webhook_url: Option<&str>,
    stream_info: StreamInfo,
) -> StreamResult<bool> {
    if duration_minutes < min_duration {
        println!(
            "Stream duration ({:.1} minutes) is below minimum threshold ({:.1} minutes), removing files without processing",
            duration_minutes, min_duration
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
