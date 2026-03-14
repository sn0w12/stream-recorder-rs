use crate::cli::upload::try_upload;
use crate::config::Config;
use crate::platform::{PipelineOutcome, PlatformConfig};
use crate::stream::api::run_pipeline;
use crate::template::{get_template_string, render_template, TemplateValue};
use crate::thumb::create_video_thumbnail_grid;
use crate::uploaders::build_uploaders;
use crate::utils::slugify;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

use chrono::{DateTime, Utc};
use fs2::available_space;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use walkdir::WalkDir;

#[cfg(feature = "discord")]
use discord_webhook2::message::Message;
#[cfg(feature = "discord")]
use discord_webhook2::webhook::DiscordWebhook;
#[cfg(feature = "discord")]
use iso8601_timestamp::Timestamp;

// Run a short ffmpeg probe to verify that `encoder` actually works at runtime.
// Many builds list encoders at compile-time (`ffmpeg -encoders`) even when the
// hardware/driver isn't present; this runtime probe prevents selecting a broken
// encoder that immediately exits and produces 0-length files.
async fn verify_hw_encoder(encoder: &str) -> Result<(), String> {
    // Run a short ffmpeg probe and capture stderr for diagnostics.
    let probe = tokio::process::Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc=duration=1:size=640x360:rate=30",
            "-c:v",
            encoder,
            "-t",
            "1",
            "-f",
            "null",
            "-",
        ])
        .stderr(Stdio::piped())
        .output()
        .await;

    // Helper to extract a short reason from ffmpeg stderr
    fn short_reason(stderr: &str) -> String {
        let s = stderr.to_lowercase();
        if s.contains("cuda_error_no_device")
            || s.contains("cuinit(0) failed")
            || s.contains("no cuda")
        {
            return "no CUDA-capable device".into();
        }
        if s.contains("error creating a mfx session") || s.contains("mfx") {
            return "intel qsv: mfx session not available".into();
        }
        if s.contains("dll amfrt64.dll failed to open") || s.contains("amfrt64.dll") {
            return "amd amf runtime not found".into();
        }
        if s.contains("error while opening encoder") {
            return "encoder failed to open (bad params or missing runtime)".into();
        }
        if s.contains("nothing was written into output file") || s.contains("received no packets") {
            return "encoder produced no output packets".into();
        }
        // Fallback: return the first non-empty ffmpeg stderr line (trimmed)
        stderr
            .lines()
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim().to_string())
            .unwrap_or_else(|| "unknown error".into())
    }

    match probe {
        Ok(output) => {
            if output.status.success() {
                return Ok(());
            }
            let err = String::from_utf8_lossy(&output.stderr);
            Err(short_reason(&err))
        }
        Err(err) => Err(format!("failed to run ffmpeg: {}", err)),
    }
}

/// Detects the best available hardware encoder by querying ffmpeg at runtime and
/// verifying it works. Priority: NVENC → QSV → VAAPI → AMF → VideoToolbox → OMX.
/// Returns the encoder `-c:v` name plus recommended extra ffmpeg options, or
/// `None` if no working hardware encoder is found.
pub async fn detect_best_hw_encoder(bitrate: &str) -> Option<(String, Vec<String>)> {
    // First check the build-time availability to avoid unnecessarily probing
    // encoders that aren't compiled into ffmpeg.
    let encoders_out = match tokio::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
        .await
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_lowercase(),
        Err(_) => String::new(),
    };

    // Candidate list in preferred order. Each tuple is (encoder_name, extra_opts).
    // Note: VAAPI requires an input hwupload filter; include that in opts.
    let candidates: Vec<(&str, Vec<String>)> = vec![
        (
            "h264_nvenc",
            vec!["-preset".into(), "p4".into(), "-b:v".into(), bitrate.into()],
        ),
        (
            "hevc_nvenc",
            vec!["-preset".into(), "p4".into(), "-b:v".into(), bitrate.into()],
        ),
        ("h264_qsv", vec!["-b:v".into(), bitrate.into()]),
        ("hevc_qsv", vec!["-b:v".into(), bitrate.into()]),
        (
            "h264_vaapi",
            vec![
                "-vf".into(),
                "format=nv12,hwupload".into(),
                "-b:v".into(),
                bitrate.into(),
            ],
        ),
        (
            "hevc_vaapi",
            vec![
                "-vf".into(),
                "format=nv12,hwupload".into(),
                "-b:v".into(),
                bitrate.into(),
            ],
        ),
        ("h264_amf", vec!["-b:v".into(), bitrate.into()]),
        ("hevc_amf", vec!["-b:v".into(), bitrate.into()]),
        ("h264_videotoolbox", vec!["-b:v".into(), bitrate.into()]),
        ("h264_omx", vec!["-b:v".into(), bitrate.into()]),
    ];

    for (enc, opts) in candidates {
        if !encoders_out.contains(enc) {
            continue; // not present in this ffmpeg build
        }

        // runtime verification — some encoders are listed but not usable at runtime
        match verify_hw_encoder(enc).await {
            Ok(()) => return Some((enc.to_string(), opts)),
            Err(_reason) => continue,
        }
    }

    None
}

/// Public helper to probe available hw encoders and print diagnostics.
/// This is intended for the CLI `encoders test` command.
pub async fn probe_hw_encoders() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // print ffmpeg encoder build-time list filtered for hardware encoders
    let encoders_out = match tokio::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .output()
        .await
    {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(e) => {
            eprintln!("failed to run 'ffmpeg -encoders': {}", e);
            String::new()
        }
    };

    println!("ffmpeg -encoders (hardware-related lines):");
    for line in encoders_out.lines() {
        let l = line.to_lowercase();
        if l.contains("nvenc")
            || l.contains("qsv")
            || l.contains("amf")
            || l.contains("vaapi")
            || l.contains("videotoolbox")
            || l.contains("omx")
            || l.contains("v4l2m2m")
        {
            println!("  {}", line.trim());
        }
    }

    // candidate encoders we probe (same order as detection)
    let candidates = vec![
        "h264_nvenc",
        "hevc_nvenc",
        "h264_qsv",
        "hevc_qsv",
        "h264_vaapi",
        "hevc_vaapi",
        "h264_amf",
        "hevc_amf",
        "h264_videotoolbox",
        "h264_omx",
    ];

    println!("\nRuntime probe for each candidate (1s test):");
    for enc in candidates {
        if !encoders_out.to_lowercase().contains(enc) {
            println!("  {:20} — not compiled into ffmpeg", enc);
            continue;
        }

        print!("  {:20} — probing... ", enc);
        match verify_hw_encoder(enc).await {
            Ok(()) => println!("OK"),
            Err(reason) => println!("FAIL ({})", reason),
        }
    }

    // final selection using detect_best_hw_encoder
    match detect_best_hw_encoder("4M").await {
        Some((enc, _opts)) => println!("\nSelected encoder: {}", enc),
        None => {
            println!("\nNo working hardware encoder detected; software (libx264) will be used.")
        }
    }

    Ok(())
}

/// Context struct holding initial information about a recording session.
#[derive(Clone)]
pub struct StreamInfo {
    pub username: String,
    pub user_id: String,
    pub stream_title: String,
    pub playback_url: String,
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

/// Helper function to send Discord webhook if the feature is enabled and webhook URL is configured.
/// Validates the webhook URL format.
#[cfg(feature = "discord")]
async fn send_discord_webhook(
    webhook_url: Option<&str>,
    message: Message,
    attachment_paths: Option<Vec<String>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(url) = webhook_url {
        if !url.starts_with("https://discord.com/api/webhooks/") {
            return Err("Invalid Discord webhook URL format".into());
        }
        let webhook = DiscordWebhook::new(url)?;
        if let Some(paths) = attachment_paths {
            if !paths.is_empty() {
                use std::collections::BTreeMap;
                let mut files = BTreeMap::new();
                for path in paths {
                    let filename = std::path::Path::new(&path)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    let data = tokio::fs::read(&path).await?;
                    files.insert(filename, data);
                }
                webhook.send_with_files(&message, files).await?;
            } else {
                webhook.send(&message).await?;
            }
        } else {
            webhook.send(&message).await?;
        }
    }
    Ok(())
}

#[cfg(not(feature = "discord"))]
async fn send_discord_webhook(
    _webhook_url: Option<&str>,
    _message: Message,
    _attachment_paths: Option<Vec<String>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // No-op when discord feature is disabled
    Ok(())
}

/// Builds ffmpeg arguments for recording with hardware acceleration when available.
fn build_ffmpeg_args(
    playback_url: &str,
    output_path: &str,
    hw_encoder: Option<(String, Vec<String>)>,
) -> Vec<String> {
    let mut ffmpeg_args: Vec<String> = vec!["-loglevel".into(), "quiet".into()];

    if let Some((codec, opts)) = hw_encoder {
        match codec.as_str() {
            // Intel Quick Sync: enable qsv hwaccel + init device so decoding is offloaded
            "h264_qsv" | "hevc_qsv" => {
                ffmpeg_args.extend(vec![
                    "-hwaccel".into(),
                    "qsv".into(),
                    "-init_hw_device".into(),
                    "qsv=hw".into(),
                    "-filter_hw_device".into(),
                    "hw".into(),
                ]);
                ffmpeg_args.push("-i".into());
                ffmpeg_args.push(playback_url.to_string());

                if !opts.is_empty() {
                    ffmpeg_args.extend(opts.clone());
                }

                ffmpeg_args.extend(vec!["-c:v".into(), codec]);
            }

            // NVIDIA NVENC: enable CUDA hwaccel so decode can use NVDEC and frames
            // can be passed to the encoder with minimal CPU overhead.
            "h264_nvenc" | "hevc_nvenc" => {
                ffmpeg_args.extend(vec![
                    "-hwaccel".into(),
                    "cuda".into(),
                    "-hwaccel_output_format".into(),
                    "cuda".into(),
                ]);
                ffmpeg_args.push("-i".into());
                ffmpeg_args.push(playback_url.to_string());

                ffmpeg_args.push("-c:v".into());
                ffmpeg_args.push(codec);
                if !opts.is_empty() {
                    ffmpeg_args.extend(opts);
                }
            }

            // VAAPI needs an explicit device and the hw upload filter (opts include that)
            "h264_vaapi" | "hevc_vaapi" => {
                ffmpeg_args.extend(vec!["-vaapi_device".into(), "/dev/dri/renderD128".into()]);
                ffmpeg_args.push("-i".into());
                ffmpeg_args.push(playback_url.to_string());

                if !opts.is_empty() {
                    ffmpeg_args.extend(opts.clone());
                }

                ffmpeg_args.extend(vec!["-c:v".into(), codec]);
            }

            // Default: no special input hwaccel, just select encoder
            _ => {
                ffmpeg_args.push("-i".into());
                ffmpeg_args.push(playback_url.to_string());
                ffmpeg_args.push("-c:v".into());
                ffmpeg_args.push(codec);
                if !opts.is_empty() {
                    ffmpeg_args.extend(opts);
                }
            }
        }
    } else {
        // software fallback
        println!("No hardware encoder available, using software encoding");
        ffmpeg_args.push("-i".into());
        ffmpeg_args.push(playback_url.to_string());
        ffmpeg_args.extend(vec![
            "-c:v".into(),
            "libx264".into(),
            "-preset".into(),
            "veryfast".into(),
            "-crf".into(),
            "26".into(),
        ]);
    }

    // Add audio encoding and output path
    ffmpeg_args.extend(vec![
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        "128k".into(),
        output_path.to_string(),
    ]);

    ffmpeg_args
}

/// Sends a Discord webhook notification for recording start.
#[cfg(feature = "discord")]
async fn send_recording_start_webhook(
    webhook_url: Option<&str>,
    username: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let message = Message::new(|message| {
        message.embed(|embed| {
            embed
                .title(format!("Stream Recording Started - {}", username))
                .color(0xFFFF00) // Yellow for starting
                .timestamp(Timestamp::now_utc())
        })
    });
    send_discord_webhook(webhook_url, message, None).await
}

#[cfg(not(feature = "discord"))]
async fn send_recording_start_webhook(
    _webhook_url: Option<&str>,
    _username: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
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

    let config = Config::load()?;
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
    if let Err(e) = send_recording_start_webhook(webhook_url, &stream_info.username).await {
        eprintln!("Error sending start webhook: {}", e);
    }

    // Detect hardware encoder and build ffmpeg arguments
    let bitrate = config.get_bitrate();
    let hw_encoder = detect_best_hw_encoder(&bitrate).await;
    let ffmpeg_args = build_ffmpeg_args(&stream_info.playback_url, &output_path, hw_encoder);

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
    use std::io::Write;

    // Build the ffmpeg concat file list in a temp file that stays alive for the
    // duration of the ffmpeg call.
    let mut concat_list = tempfile::NamedTempFile::new()?;
    {
        let mut writer = std::io::BufWriter::new(concat_list.as_file_mut());
        for file in files {
            // Escape single quotes so paths with apostrophes don't break the
            // concat file format.
            writeln!(writer, "file '{}'", file.replace('\'', r"'\''"))?;
        }
    }

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

    let output = tokio::process::Command::new("ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            concat_list
                .path()
                .to_str()
                .ok_or("invalid concat file path")?,
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
                    let stream_info = StreamInfo {
                        username: username.to_string(),
                        user_id,
                        stream_title,
                        playback_url: url,
                    };

                    let config = Config::load().unwrap_or_default();
                    let reconnect_delay = config.get_stream_reconnect_delay_minutes();

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
                                                            + Duration::from_secs_f64(delay_minutes * 60.0);
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
                                println!(
                                    "Continuation window expired for {}, post-processing {} segment(s)...",
                                    username,
                                    session_files.len()
                                );
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
        if thumbnail_path.exists() {
            if let Err(e) = tokio::fs::remove_file(&thumbnail_path).await {
                eprintln!(
                    "Failed to delete thumbnail {}: {}",
                    thumbnail_path.display(),
                    e
                );
            }
        }
    }

    Ok(())
}

/// Checks if a stream is below minimum duration and deletes it if needed.
async fn handle_minimum_duration(
    output_path: &str,
    duration_minutes: f64,
    min_duration: f64,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    if duration_minutes < min_duration {
        println!(
            "Stream duration ({:.1} minutes) is below minimum threshold ({:.1} minutes), removing files without processing",
            duration_minutes, min_duration
        );
        delete_video_and_thumbnail(output_path).await?;
        return Ok(true);
    }
    Ok(false)
}

/// Sends a Discord webhook notification for recorded stream completion.
#[cfg(feature = "discord")]
async fn send_recording_complete_webhook(
    webhook_url: Option<&str>,
    stream_info: &StreamInfo,
    duration_str: &str,
    size_str: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let message = Message::new(|message| {
        message.embed(|embed| {
            embed
                .title(format!("Stream Recorded - {}", stream_info.username))
                .field(|field| field.name("Stream Title").value(&stream_info.stream_title))
                .field(|field| field.name("Duration").value(duration_str))
                .field(|field| field.name("File Size").value(size_str))
                .color(0x00FF00) // Green for completion
                .timestamp(Timestamp::now_utc())
        })
    });
    send_discord_webhook(webhook_url, message, None).await
}

#[cfg(not(feature = "discord"))]
async fn send_recording_complete_webhook(
    _webhook_url: Option<&str>,
    _stream_info: &StreamInfo,
    _duration_str: &str,
    _size_str: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
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
    let config = Config::load()?;

    // Check if stream duration is below minimum threshold
    if let Some(min_duration) = config.get_min_stream_duration() {
        if handle_minimum_duration(&output_path, duration_minutes, min_duration).await? {
            return Ok(());
        }
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
        None,
        None,
    )
    .await
    {
        Ok(_) => println!("Generated thumbnail: {}", thumbnail_path),
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
            Err(e) => eprintln!("{} folder lookup failed: {}", uploader.name(), e),
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

    // Print upload results
    for (uploader, urls) in &upload_results {
        for url in urls {
            println!("{} uploaded to: {}", uploader, url);
        }
    }

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

    if let Some(template) = get_template_string() {
        let content = render_template(template, &template_context);
        #[cfg(feature = "discord")]
        {
            let mut attachments = vec![];
            if std::path::Path::new(&thumbnail_path).exists() {
                attachments.push(thumbnail_path.clone());
            }
            let message = Message::new(|message| message.content(format!("```\n{}\n```", content)));
            if let Err(e) = send_discord_webhook(webhook_url, message, Some(attachments)).await {
                eprintln!("Error sending upload complete webhook: {}", e);
            }
        }
    }

    Ok(())
}

/// Manages disk space by deleting oldest streams if free space is below the configured minimum.
/// Also deletes corresponding thumbnails.
async fn manage_disk_space() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load()?;
    let output_dir = config.get_output_directory();
    let min_free_gb = config.get_min_free_space_gb();
    let min_free_bytes = (min_free_gb * 1_000_000_000.0) as u64;

    let free_bytes = available_space(&output_dir)?;
    let free_gb = free_bytes as f64 / 1_000_000_000.0;
    println!(
        "Checking disk space: output_dir = {}, free_gb = {:.2}, min_free_gb = {:.2}",
        output_dir, free_gb, min_free_gb
    );
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
        if thumb_path.exists() {
            if let Err(e) = tokio::fs::remove_file(&thumb_path).await {
                eprintln!("Failed to delete thumbnail {}: {}", thumb_path.display(), e);
            }
        }
    }

    Ok(())
}
