pub mod ffmpeg;
pub mod storage;
pub mod thumb;

use super::{StreamResult, types::StreamInfo};
use crate::cli::upload::{UploadFailure, try_upload};
use crate::config::Config;
use crate::extension::context::{ErrorContext, RecordingContext};
use crate::extension::{hooks, store};
use crate::stream::messages::{
    send_minimum_duration_webhook, send_program_error_webhook, send_recording_complete_webhook,
    send_template_webhook,
};
use crate::template::{TemplateValue, get_template_string, render_template_with_extensions};
use crate::types::{DurationValue, FileSize};
use crate::uploaders::{UploaderKindFilter, UploaderList, build_uploaders};
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::SystemTime;
use thumb::create_video_thumbnail_grid;
use tiny_table::{Cell, Column, ColumnWidth, Table};

/// Ensures only one post-processing session runs at a time.
static POSTPROCESS_LOCK: LazyLock<tokio::sync::Mutex<()>> =
    LazyLock::new(|| tokio::sync::Mutex::new(()));

#[derive(Clone)]
struct RecordingFile {
    path: PathBuf,
    modified: SystemTime,
    user_key: String,
}

struct ExtensionSessionGuard(String);

impl Drop for ExtensionSessionGuard {
    fn drop(&mut self) {
        hooks::cleanup_session(&self.0);
    }
}

/// Post-processes a complete recording session that may consist of multiple files.
pub async fn post_process_session(
    stream_info: StreamInfo,
    session_files: Vec<String>,
    session_id: String,
) -> StreamResult<()> {
    let _extension_guard = ExtensionSessionGuard(session_id.clone());

    if session_files.is_empty() {
        return Ok(());
    }

    let session_files: Vec<String> = session_files
        .into_iter()
        .filter(|f| {
            if !Path::new(f).exists() {
                eprintln!("Segment file missing, skipping: {}", f);
                false
            } else {
                true
            }
        })
        .collect();

    if session_files.is_empty() {
        return Ok(());
    }

    let mut pending_guard = storage::PendingGuard::new(&session_files);

    let _postprocess_guard = match POSTPROCESS_LOCK.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            println!(
                "Post-processing for {} is queuing behind the session already in progress...",
                stream_info.username
            );
            POSTPROCESS_LOCK.lock().await
        }
    };

    let result = if session_files.len() == 1 {
        let file = session_files
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("expected a single session file"))?;
        post_process_stream(stream_info, file, &session_id, 1).await
    } else {
        println!(
            "Combining {} stream segments for {}...",
            session_files.len(),
            stream_info.username
        );
        let segment_count = session_files.len();

        match concat_video_files(&session_files).await {
            Ok(combined_path) => {
                println!(
                    "Successfully combined stream segments into: {}",
                    combined_path
                );

                pending_guard.add(&combined_path);

                for file in &session_files {
                    if let Err(error) = tokio::fs::remove_file(file).await {
                        eprintln!("Failed to delete segment {}: {}", file, error);
                    }
                }

                post_process_stream(stream_info, combined_path, &session_id, segment_count).await
            }
            Err(error) => {
                eprintln!(
                    "Failed to combine stream segments ({}), processing files individually...",
                    error
                );

                let config = Config::get();
                send_program_error_webhook(
                    config.get_discord_webhook_url().as_deref(),
                    "Failed to combine stream segments",
                    &format!(
                        "Recording for `{}` on platform `{}` produced {} segments that could not be combined into a single file.\nSegments will be processed individually.\n\n{}",
                        stream_info.username,
                        stream_info.platform.id,
                        session_files.len(),
                        error,
                    ),
                )
                .await;

                for file in session_files {
                    if let Err(post_process_error) =
                        post_process_stream(stream_info.clone(), file, &session_id, 1).await
                    {
                        eprintln!("Error post-processing segment: {}", post_process_error);
                        hooks::on_error(
                            Some(&session_id),
                            Some(&stream_info),
                            ErrorContext::new(
                                "postprocess",
                                format!("Post-processing a segment failed: {}", post_process_error),
                            ),
                            None,
                        )
                        .await;
                    }
                }

                Ok(())
            }
        }
    };

    drop(pending_guard);
    if let Err(error) = storage::manage_disk_space().await {
        eprintln!("Error managing disk space: {}", error);
    }
    result
}

async fn post_process_stream(
    stream_info: StreamInfo,
    output_path: String,
    session_id: &str,
    segments: usize,
) -> StreamResult<()> {
    println!("Post-processing recorded stream: {}", output_path);

    let (file_size, duration) = get_video_metadata(&output_path).await?;
    let config = Config::get();

    if let Some(min_duration) = config.get_min_stream_duration()
        && handle_minimum_duration(
            &output_path,
            duration,
            min_duration,
            config.get_discord_webhook_url().as_deref(),
            stream_info.clone(),
        )
        .await?
    {
        return Ok(());
    }

    let config = Config::get();
    let webhook_url = config.get_discord_webhook_url();
    if let Err(error) =
        send_recording_complete_webhook(webhook_url.as_deref(), &stream_info, &duration, &file_size)
            .await
    {
        eprintln!("Error sending recorded webhook: {}", error);
    }

    let thumbnail_path = generate_thumbnail(&output_path).await;

    let recording_context = RecordingContext {
        file: Some(output_path.clone()),
        thumbnail: Some(thumbnail_path.clone()),
        duration_seconds: Some(duration.as_secs_f64()),
        size_bytes: Some(file_size.as_bytes()),
        segments: Some(segments),
        ..RecordingContext::default()
    };

    hooks::after_recording(
        session_id,
        &stream_info,
        &output_path,
        &thumbnail_path,
        duration.as_secs_f64(),
        file_size.as_bytes(),
        segments,
    )
    .await;

    let image_uploaders = build_uploaders(UploaderKindFilter::Image);
    let video_uploaders = build_uploaders(UploaderKindFilter::Video);
    hooks::before_upload(
        session_id,
        &stream_info,
        &recording_context,
        uploader_names(&video_uploaders),
        uploader_names(&image_uploaders),
    )
    .await;

    let (thumbnail_uploads, image_failures) =
        upload_with_stream(&stream_info, &thumbnail_path, &image_uploaders).await;
    let (upload_results, video_failures) =
        upload_with_stream(&stream_info, &output_path, &video_uploaders).await;
    print_upload_results(&upload_results);

    hooks::after_upload(
        session_id,
        &stream_info,
        &recording_context,
        &upload_results,
        &thumbnail_uploads,
    )
    .await;

    for failure in image_failures.into_iter().chain(video_failures) {
        let error = ErrorContext {
            stage: "upload".to_string(),
            message: format!("Upload to `{}` failed", failure.uploader),
            uploader: Some(failure.uploader),
            file: Some(failure.file),
            status_code: failure.status_code,
            retryable: Some(failure.retryable),
        };
        hooks::on_error(
            Some(session_id),
            Some(&stream_info),
            error,
            Some(recording_context.clone()),
        )
        .await;
    }

    let template_info = TemplateInfo {
        output_path: output_path.clone(),
        thumbnail_path: thumbnail_path.clone(),
        upload_urls: upload_results,
        thumbnail_upload_urls: thumbnail_uploads,
        duration,
        file_size,
    };
    send_template_notification(&stream_info, &template_info, session_id).await;

    Ok(())
}

fn uploader_names(uploaders: &UploaderList) -> Vec<String> {
    uploaders
        .iter()
        .map(|(uploader, _)| uploader.name().to_string())
        .collect()
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

async fn upload_with_stream(
    stream_info: &StreamInfo,
    output_path: &str,
    uploaders: &UploaderList,
) -> (HashMap<String, Vec<String>>, Vec<UploadFailure>) {
    let config = Config::get();
    let max_retries = config.get_max_upload_retries();
    let mut upload_results = HashMap::new();
    let mut failures = Vec::new();

    for (uploader, uploader_config) in uploaders {
        let mut uploader_settings = uploader_config.clone();
        match uploader
            .get_or_create_folder(&stream_info.username, uploader_config)
            .await
        {
            Ok(Some(folder_id)) => uploader_settings.folder_id = Some(folder_id),
            Ok(None) => {}
            Err(_) => {}
        }

        failures.extend(
            try_upload(
                uploader.as_ref(),
                output_path,
                &uploader_settings,
                &mut upload_results,
                max_retries,
            )
            .await,
        );
    }

    (upload_results, failures)
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
    thumbnail_upload_urls: HashMap<String, Vec<String>>,
    duration: DurationValue,
    file_size: FileSize,
}

async fn send_template_notification(
    stream_info: &StreamInfo,
    template_info: &TemplateInfo,
    session_id: &str,
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
        TemplateValue::String(template_info.duration.to_string()),
    );
    context.insert(
        "file_size".to_string(),
        TemplateValue::String(template_info.file_size.to_string()),
    );

    for (uploader, urls) in &template_info.thumbnail_upload_urls {
        context.insert(
            format!("{}_urls", uploader),
            TemplateValue::Array(urls.clone()),
        );
    }

    for (uploader, urls) in &template_info.upload_urls {
        context.insert(
            format!("{}_urls", uploader),
            TemplateValue::Array(urls.clone()),
        );
    }

    let extension_values = store::template_values(session_id);
    let content = format!(
        "```\n{}\n```",
        render_template_with_extensions(&template, &context, &extension_values)
    );
    let config = Config::get();
    let webhook_url = config.get_discord_webhook_url();
    if let Err(error) = send_template_webhook(
        webhook_url.as_deref(),
        stream_info,
        &content,
        &template_info.thumbnail_path,
    )
    .await
    {
        eprintln!("Error sending template webhook: {}", error);
        hooks::on_error(
            Some(session_id),
            Some(stream_info),
            ErrorContext::new("template", "Sending the template notification failed"),
            None,
        )
        .await;
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
    use super::{POSTPROCESS_LOCK, build_ffconcat_manifest, concat_video_files};

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

    #[tokio::test]
    async fn postprocess_lock_serializes_concurrent_holders() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };

        let current = Arc::new(AtomicUsize::new(0));
        let max_observed = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..8 {
            let current = Arc::clone(&current);
            let max_observed = Arc::clone(&max_observed);
            handles.push(tokio::spawn(async move {
                let _guard = POSTPROCESS_LOCK.lock().await;
                let now = current.fetch_add(1, Ordering::SeqCst) + 1;
                max_observed.fetch_max(now, Ordering::SeqCst);
                // Widen the race window: without mutual exclusion another task
                // would overlap here and push the observed count above one.
                tokio::task::yield_now().await;
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                current.fetch_sub(1, Ordering::SeqCst);
            }));
        }

        for handle in handles {
            handle.await.expect("postprocess lock task panicked");
        }

        assert_eq!(
            max_observed.load(Ordering::SeqCst),
            1,
            "post-processing lock must allow only one holder at a time"
        );
    }
}
