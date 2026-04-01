use crate::cli::upload::try_upload;
use crate::config::Config;
use crate::platform::{PipelineOutcome, PlatformConfig};
use crate::print::table::{Cell, Table};
use crate::stream::api::run_pipeline;
use crate::stream::encoding::{VideoEncoding, build_ffmpeg_args, detect_best_hw_encoder};
use crate::stream::messages::{
    send_minimum_duration_webhook, send_recording_complete_webhook, send_recording_start_webhook,
    send_template_webhook,
};
use crate::template::{TemplateValue, get_template_string, render_template};
use crate::thumb::create_video_thumbnail_grid;
use crate::uploaders::build_uploaders;
use crate::utils::slugify;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

use chrono::{DateTime, Utc};
use fs2::available_space;
use std::io::Write;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use walkdir::WalkDir;

/// Context struct holding initial information about a recording session.
#[derive(Clone)]
pub struct StreamInfo {
    pub username: String,
    pub user_id: String,
    pub stream_title: String,
    pub playback_url: String,
    pub avatar_url: Option<String>,
    pub platform: PlatformConfig,
}

/// Struct to manage the ffmpeg process, ensuring it's killed when dropped.
struct StreamRecorder {
    child: tokio::process::Child,
}

impl StreamRecorder {
    async fn new(
        cmd: &mut tokio::process::Command,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // capture stderr so short-lived ffmpeg failures are visible in logs
        cmd.stderr(Stdio::piped());
        let mut child = cmd.spawn()?;

        // forward ffmpeg stderr lines to our stderr asynchronously
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr);
                let mut line = String::new();
                loop {
                    match reader.read_line(&mut line).await {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            eprintln!("ffmpeg: {}", line.trim_end());
                            line.clear();
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        Ok(Self { child })
    }

    async fn wait(mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.child.wait().await?;
        Ok(())
    }
}

impl Drop for StreamRecorder {
    fn drop(&mut self) {
        // Kill the child process when the struct is dropped
        let _ = self.child.start_kill();
    }
}

/// Core recording logic: starts ffmpeg, waits for the stream to end, and returns
/// the stream info together with the output file path.  Does NOT spawn any
/// post-processing — callers are responsible for that.
async fn record_stream_raw(
    stream_info: StreamInfo,
) -> Result<(StreamInfo, String), Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "Starting recording for stream: {} (user_id: {}, title: {})",
        stream_info.username, stream_info.user_id, stream_info.stream_title
    );

    let config = Config::get();
    let output_dir = config.get_output_directory();
    std::fs::create_dir_all(&output_dir)?;

    let slugified_username = slugify(&stream_info.username);
    let user_dir = format!("{}/{}", output_dir, slugified_username);
    std::fs::create_dir_all(&user_dir)?;

    let timestamp: DateTime<Utc> = Utc::now();
    let timestamp_str = timestamp.format("%Y-%m-%d_%H-%M-%S").to_string();
    let output_path = format!("{}/{}_{}.mp4", user_dir, slugified_username, timestamp_str);

    // Send Discord webhook for recording start
    let webhook_url = config.get_discord_webhook_url();
    if let Err(e) = send_recording_start_webhook(webhook_url, &stream_info).await {
        eprintln!("Error sending start webhook: {}", e);
    }

    // Detect hardware encoder and build ffmpeg arguments
    let video_quality = config.get_video_quality();
    let video_bitrate = config.get_video_bitrate();
    let max_bitrate = config.get_max_bitrate();
    let encoding = match video_bitrate {
        Some(bitrate) => VideoEncoding::ConstantBitrate(bitrate.to_string()),
        None => VideoEncoding::Quality(video_quality),
    };
    let hw_encoder = detect_best_hw_encoder(&encoding).await;
    let ffmpeg_args = build_ffmpeg_args(
        &stream_info.playback_url,
        &output_path,
        &encoding,
        hw_encoder,
        max_bitrate,
    );

    let mut cmd = tokio::process::Command::new("ffmpeg");
    cmd.args(&ffmpeg_args);

    // Start recording and wait for completion
    let recorder = StreamRecorder::new(&mut cmd).await?;
    recorder.wait().await?;

    Ok((stream_info, output_path))
}

/// Records a stream using ffmpeg.
/// Runs the ffmpeg command with the provided playback URL and saves to the output path.
/// Waits for the command to finish, which indicates the stream has ended.
/// Then spawns a new task to post-process the recorded file.
pub async fn record_stream(
    stream_info: StreamInfo,
) -> Result<(StreamInfo, String), Box<dyn std::error::Error + Send + Sync>> {
    let (stream_info, output_path) = record_stream_raw(stream_info).await?;

    // Spawn post-processing on a new task
    let stream_info_clone = stream_info.clone();
    let output_path_clone = output_path.clone();
    tokio::spawn(async move {
        if let Err(e) = post_process_stream(stream_info_clone, output_path_clone).await {
            eprintln!("Error post-processing stream: {}", e);
        }
    });

    Ok((stream_info, output_path))
}

/// Concatenates multiple MP4 files into a single file using ffmpeg's concat demuxer.
/// The output file is placed in the same directory as the first segment.
/// Returns the path of the combined output file.
async fn concat_video_files(
    files: &[String],
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if files.is_empty() {
        return Err("cannot concatenate an empty segment list".into());
    }

    let concat_manifest = build_ffconcat_manifest(files)?;

    // Place the combined file next to the first segment, adding a _combined suffix.
    let first_path = std::path::Path::new(&files[0]);
    let parent_dir = first_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    let file_stem = first_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
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
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "ffmpeg concat failed for {} segment(s): {}",
            files.len(),
            err
        )
        .into());
    }

    Ok(combined_path)
}

fn build_ffconcat_manifest(
    files: &[String],
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut manifest = String::from("ffconcat version 1.0\n");

    for file in files {
        let canonical_path = std::fs::canonicalize(file)
            .map_err(|e| format!("segment file missing or inaccessible '{}': {}", file, e))?;
        let escaped_path = canonical_path
            .to_string_lossy()
            .replace('\\', "/")
            .replace('\'', r"'\''");
        manifest.push_str(&format!("file '{}'\n", escaped_path));
    }

    Ok(manifest)
}

/// Post-processes a complete recording session that may consist of multiple
/// segment files (when stream continuation is enabled).
///
/// * Single segment  → passed directly to `post_process_stream`.
/// * Multiple segments → combined with ffmpeg concat first; individual segments
///   are deleted after a successful merge.  On merge failure, each segment is
///   processed individually as a fallback.
async fn post_process_session(
    stream_info: StreamInfo,
    session_files: Vec<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
            // Remove the individual segment files now that they are merged.
            for file in &session_files {
                if let Err(e) = tokio::fs::remove_file(file).await {
                    eprintln!("Failed to delete segment {}: {}", file, e);
                }
            }
            post_process_stream(stream_info, combined_path).await
        }
        Err(e) => {
            eprintln!(
                "Failed to combine stream segments ({}), processing files individually...",
                e
            );
            // Fall back: process each segment on its own.
            for file in session_files {
                if let Err(e2) = post_process_stream(stream_info.clone(), file).await {
                    eprintln!("Error post-processing segment: {}", e2);
                }
            }
            Ok(())
        }
    }
}

/// Monitors the stream for a specific user.
/// Runs in a loop, executing the platform pipeline at each interval.
/// When the pipeline returns Live the stream is recorded; when Offline the
/// loop simply waits for the next interval.
///
/// When `stream_reconnect_delay_minutes` is configured the monitor enters a
/// **continuation window** after each recording ends.  During that window it
/// keeps polling; if a new stream starts the recording is resumed.  After the
/// window closes all accumulated segment files are merged and post-processed
/// together as one session.
pub async fn monitor_stream(
    username: &str,
    platform: &PlatformConfig,
    token: &str,
    fetch_interval: Duration,
) {
    loop {
        match run_pipeline(username, platform, token).await {
            Ok(PipelineOutcome::Live(vars)) => {
                let playback_url = vars.get("playback_url").cloned();
                if let Some(url) = playback_url {
                    let stream_title = vars
                        .get("stream_title")
                        .map(|s| platform.clean_title(s))
                        .unwrap_or_default();
                    let user_id = vars
                        .get("user_id")
                        .cloned()
                        .unwrap_or_else(|| username.to_string());
                    let avatar_url = vars.get("avatar_url").cloned();
                    let stream_info = StreamInfo {
                        username: username.to_string(),
                        user_id,
                        stream_title,
                        playback_url: url,
                        avatar_url,
                        platform: platform.clone(),
                    };

                    let reconnect_delay = Config::get().get_stream_reconnect_delay_minutes();

                    if let Some(delay_minutes) = reconnect_delay {
                        // ── Continuation mode ─────────────────────────────────────
                        // Record the first stream of the session.
                        let primary_stream_info = stream_info.clone();
                        match record_stream_raw(stream_info).await {
                            Ok((_, first_path)) => {
                                let mut session_files = vec![first_path];
                                let mut deadline = tokio::time::Instant::now()
                                    + Duration::from_secs_f64(delay_minutes * 60.0);

                                println!(
                                    "Stream ended for {}. Waiting up to {:.0} minute(s) for a continuation before post-processing...",
                                    username, delay_minutes
                                );

                                // Continuation polling loop.
                                loop {
                                    let now = tokio::time::Instant::now();
                                    if now >= deadline {
                                        break;
                                    }

                                    let remaining = deadline - now;
                                    let sleep_duration = fetch_interval.min(remaining);
                                    sleep(sleep_duration).await;

                                    match run_pipeline(username, platform, token).await {
                                        Ok(PipelineOutcome::Live(new_vars)) => {
                                            if let Some(new_url) =
                                                new_vars.get("playback_url").cloned()
                                            {
                                                let new_stream_info = StreamInfo {
                                                    username: username.to_string(),
                                                    user_id: new_vars
                                                        .get("user_id")
                                                        .cloned()
                                                        .unwrap_or_else(|| username.to_string()),
                                                    stream_title: new_vars
                                                        .get("stream_title")
                                                        .map(|s| platform.clean_title(s))
                                                        .unwrap_or_default(),
                                                    playback_url: new_url,
                                                    avatar_url: new_vars.get("avatar_url").cloned(),
                                                    platform: platform.clone(),
                                                };
                                                println!(
                                                    "Continuation stream detected for {}, recording...",
                                                    username
                                                );
                                                match record_stream_raw(new_stream_info).await {
                                                    Ok((_, new_path)) => {
                                                        session_files.push(new_path);
                                                        // Reset the deadline so we keep watching
                                                        // after this segment ends too.
                                                        deadline = tokio::time::Instant::now()
                                                            + Duration::from_secs_f64(
                                                                delay_minutes * 60.0,
                                                            );
                                                        println!(
                                                            "Continuation stream ended for {}. Waiting up to {:.0} minute(s) for another continuation...",
                                                            username, delay_minutes
                                                        );
                                                    }
                                                    Err(e) => eprintln!(
                                                        "Error recording continuation stream for {}: {}",
                                                        username, e
                                                    ),
                                                }
                                            }
                                        }
                                        Ok(PipelineOutcome::Offline) => {
                                            // Streamer still offline — keep waiting.
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Error running pipeline for {} during continuation window: {}",
                                                username, e
                                            );
                                        }
                                    }
                                }

                                // Continuation window closed — post-process the full session.
                                let info = primary_stream_info;
                                let files = session_files;
                                tokio::spawn(async move {
                                    if let Err(e) = post_process_session(info, files).await {
                                        eprintln!("Error post-processing session: {}", e);
                                    }
                                });

                                // We have already spent the delay window waiting; skip
                                // the standard fetch_interval sleep at the bottom.
                                continue;
                            }
                            Err(e) => eprintln!("Error recording stream for {}: {}", username, e),
                        }
                    } else {
                        // ── Standard mode (original behaviour) ────────────────────
                        if let Err(e) = record_stream(stream_info).await {
                            eprintln!("Error recording stream: {}", e);
                        }
                    }
                } else {
                    eprintln!("Pipeline returned Live but 'playback_url' was not extracted");
                }
            }
            Ok(PipelineOutcome::Offline) => {
                // Stream not live — just wait for the next poll.
            }
            Err(e) => {
                eprintln!("Error running pipeline for {}: {}", username, e);
            }
        }
        sleep(fetch_interval).await;
    }
}
async fn get_video_metadata(
    output_path: &str,
) -> Result<(f64, f64), Box<dyn std::error::Error + Send + Sync>> {
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

/// Formats file size into a human-readable string (MB or GB).
fn format_file_size(file_size_mb: f64) -> String {
    let file_size_gb = file_size_mb / 1024.0;
    if file_size_gb >= 1.0 {
        format!("{:.2} GB", file_size_gb)
    } else {
        format!("{:.2} MB", file_size_mb)
    }
}

/// Formats duration into a human-readable string (hours and minutes).
fn format_duration(duration_minutes: f64) -> String {
    let hours = (duration_minutes / 60.0).floor() as u32;
    let mins = (duration_minutes % 60.0).round() as u32;
    if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

/// Deletes a video file and its corresponding thumbnail if they exist.
async fn delete_video_and_thumbnail(
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Delete the video file
    if let Err(e) = tokio::fs::remove_file(output_path).await {
        eprintln!("Failed to delete video file {}: {}", output_path, e);
    } else {
        println!("Deleted video file: {}", output_path);
    }

    // Delete thumbnail if it exists
    let output_path_buf = std::path::Path::new(output_path);
    if let Some(file_stem) = output_path_buf.file_stem() {
        let thumbnail_path =
            output_path_buf.with_file_name(format!("{}_thumb.jpg", file_stem.to_string_lossy()));
        if thumbnail_path.exists()
            && let Err(e) = tokio::fs::remove_file(&thumbnail_path).await
        {
            eprintln!(
                "Failed to delete thumbnail {}: {}",
                thumbnail_path.display(),
                e
            );
        }
    }

    Ok(())
}

/// Checks if a stream is below minimum duration and deletes it if needed.
async fn handle_minimum_duration(
    output_path: &str,
    duration_minutes: f64,
    min_duration: f64,
    webhook_url: Option<&str>,
    stream_info: StreamInfo,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    if duration_minutes < min_duration {
        println!(
            "Stream duration ({:.1} minutes) is below minimum threshold ({:.1} minutes), removing files without processing",
            duration_minutes, min_duration
        );
        send_minimum_duration_webhook(webhook_url, &stream_info).await?;
        delete_video_and_thumbnail(output_path).await?;
        return Ok(true);
    }
    Ok(false)
}

/// Post-processes the recorded stream file.
/// This function runs on a separate task after recording is complete.
/// Sends a Discord webhook if configured.
async fn post_process_stream(
    stream_info: StreamInfo,
    output_path: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Post-processing recorded stream: {}", output_path);

    manage_disk_space().await?;

    let (file_size_mb, duration_minutes) = get_video_metadata(&output_path).await?;
    let config = Config::get();

    // Check if stream duration is below minimum threshold
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

    // Send Discord notification for completed recording
    let webhook_url = config.get_discord_webhook_url();
    if let Err(e) =
        send_recording_complete_webhook(webhook_url, &stream_info, &duration_str, &size_str).await
    {
        eprintln!("Error sending recorded webhook: {}", e);
    }

    // Generate thumbnail
    let thumbnail_path = output_path.replace(".mp4", "_thumb.jpg");
    match create_video_thumbnail_grid(
        std::path::Path::new(&output_path),
        std::path::Path::new(&thumbnail_path),
        &config.get_thumbnail_size(),
        &config.get_thumbnail_grid(),
    )
    .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("Failed to generate thumbnail: {}", e),
    }

    // Upload files to configured services
    let mut upload_results: HashMap<String, Vec<String>> = HashMap::new();
    let max_retries = config.get_max_upload_retries();

    let uploaders = build_uploaders().await;
    for (uploader, uploader_config) in &uploaders {
        let mut config = uploader_config.clone();
        match uploader.get_folder_id_by_name(&stream_info.username).await {
            Ok(Some(folder_id)) => config.folder_id = Some(folder_id),
            Ok(None) => {} // not supported or not found
            Err(_) => {}   // folder not found
        }
        try_upload(
            uploader.as_ref(),
            &output_path,
            &config,
            &mut upload_results,
            max_retries,
        )
        .await;
    }

    let mut upload_table = Table::new();
    upload_table.set_headers(vec![Cell::new("Uploader"), Cell::new("URLs")]);
    for (uploader, urls) in &upload_results {
        upload_table.add_row(vec![Cell::new(uploader), Cell::new(urls.join(", "))]);
    }
    upload_table.print();

    // Send template-based upload complete webhook
    let mut template_context = HashMap::new();
    let date = Utc::now().format("%Y-%m-%d").to_string();
    template_context.insert("date".to_string(), TemplateValue::String(date));
    template_context.insert(
        "username".to_string(),
        TemplateValue::String(stream_info.username.clone()),
    );
    template_context.insert(
        "user_id".to_string(),
        TemplateValue::String(stream_info.user_id.clone()),
    );
    template_context.insert(
        "output_path".to_string(),
        TemplateValue::String(output_path.clone()),
    );
    template_context.insert(
        "thumbnail_path".to_string(),
        TemplateValue::String(thumbnail_path.clone()),
    );
    template_context.insert(
        "stream_title".to_string(),
        TemplateValue::String(stream_info.stream_title.clone()),
    );
    for (uploader, urls) in &upload_results {
        template_context.insert(
            format!("{}_urls", uploader),
            TemplateValue::Array(urls.clone()),
        );
    }

    if let Some(template) = get_template_string()? {
        let content = format!(
            "```\n{}\n```",
            render_template(&template, &template_context)
        );
        {
            if let Err(e) =
                send_template_webhook(webhook_url, &stream_info, &content, thumbnail_path.clone())
                    .await
            {
                eprintln!("Error sending template webhook: {}", e);
            }
        }
    }

    Ok(())
}

/// Manages disk space by deleting oldest streams if free space is below the configured minimum.
/// Also deletes corresponding thumbnails.
async fn manage_disk_space() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::get();
    let output_dir = config.get_output_directory();
    let min_free_gb = config.get_min_free_space_gb();
    let min_free_bytes = (min_free_gb * 1_000_000_000.0) as u64;

    let free_bytes = available_space(&output_dir)?;
    if free_bytes >= min_free_bytes {
        return Ok(());
    }

    println!(
        "Free space {} GB is below minimum {} GB, cleaning up old streams...",
        free_bytes as f64 / 1_000_000_000.0,
        min_free_gb
    );

    // Collect all .mp4 files recursively
    let mut files: Vec<_> = WalkDir::new(&output_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("mp4"))
        .filter_map(|e| {
            let path = e.path().to_path_buf();
            let metadata = std::fs::metadata(&path).ok()?;
            let modified = metadata.modified().ok()?;
            Some((path, modified))
        })
        .collect();

    // Sort by modification time, oldest first
    files.sort_by_key(|(_, time)| *time);

    for (mp4_path, _) in files {
        if available_space(&output_dir)? >= min_free_bytes {
            break;
        }

        println!("Deleting old stream: {}", mp4_path.display());
        if let Err(e) = tokio::fs::remove_file(&mp4_path).await {
            eprintln!("Failed to delete {}: {}", mp4_path.display(), e);
            continue;
        }

        // Delete thumbnail
        let thumb_path = mp4_path.with_file_name(format!(
            "{}_thumb.jpg",
            mp4_path.file_stem().unwrap().to_string_lossy()
        ));
        if thumb_path.exists()
            && let Err(e) = tokio::fs::remove_file(&thumb_path).await
        {
            eprintln!("Failed to delete thumbnail {}: {}", thumb_path.display(), e);
        }
    }

    Ok(())
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
