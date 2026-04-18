use super::{StreamResult, types::StreamInfo};
use crate::config::Config;
use crate::platform::PipelineOutcome;
use crate::stream::api::run_pipeline;
use crate::stream::encoding::{VideoEncoding, build_ffmpeg_args, detect_best_hw_encoder};
use crate::stream::messages::send_recording_start_webhook;
use crate::utils::slugify;
use chrono::{DateTime, Utc};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::{Duration, sleep};

/// Manages the ffmpeg process, ensuring it is terminated when dropped.
struct StreamRecorder {
    child: tokio::process::Child,
}

impl StreamRecorder {
    async fn new(cmd: &mut tokio::process::Command) -> StreamResult<Self> {
        cmd.stderr(Stdio::piped());
        let mut child = cmd.spawn()?;

        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr);
                let mut line = String::new();

                loop {
                    match reader.read_line(&mut line).await {
                        Ok(0) => break,
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

    async fn wait(mut self) -> StreamResult<()> {
        self.child.wait().await?;
        Ok(())
    }

    async fn wait_with_metadata_refresh(
        mut self,
        stream_info: &mut StreamInfo,
        token: &str,
        refresh_interval: Duration,
    ) -> StreamResult<()> {
        let wait = self.child.wait();
        tokio::pin!(wait);

        loop {
            tokio::select! {
                result = &mut wait => {
                    result?;
                    return Ok(());
                }
                _ = sleep(refresh_interval) => {
                    refresh_stream_info(stream_info, token).await;
                }
            }
        }
    }
}

impl Drop for StreamRecorder {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// Starts ffmpeg for the given stream and waits until the recording ends.
/// Returns the output file path.
/// When `is_continuation` is true, the recording-start webhook is suppressed.
pub async fn record_segment(
    stream_info: &mut StreamInfo,
    token: &str,
    is_continuation: bool,
) -> StreamResult<String> {
    println!(
        "Starting recording for stream: {} (title: {})",
        stream_info.username,
        stream_info
            .extracted
            .stream_title
            .as_deref()
            .unwrap_or_default()
    );

    let output_path = build_output_path(&stream_info.username)?;

    if !is_continuation {
        let webhook_url = Config::get().get_discord_webhook_url();
        if let Err(error) = send_recording_start_webhook(webhook_url, stream_info).await {
            eprintln!("Error sending start webhook: {}", error);
        }
    }

    let ffmpeg_args = build_recording_args(stream_info, &output_path).await;
    let mut command = tokio::process::Command::new("ffmpeg");
    command.args(&ffmpeg_args);

    let recorder = StreamRecorder::new(&mut command).await?;
    if let Some(interval_seconds) = Config::get().get_stream_metadata_refresh_interval_seconds() {
        recorder
            .wait_with_metadata_refresh(
                stream_info,
                token,
                Duration::from_secs_f64(interval_seconds),
            )
            .await?;
    } else {
        recorder.wait().await?;
    }

    Ok(output_path)
}

fn build_output_path(username: &str) -> StreamResult<String> {
    let config = Config::get();
    let output_dir = config.get_output_directory();
    std::fs::create_dir_all(&output_dir)?;

    let slugified_username = slugify(username);
    let user_dir = format!("{}/{}", output_dir, slugified_username);
    std::fs::create_dir_all(&user_dir)?;

    let timestamp: DateTime<Utc> = Utc::now();
    let timestamp_str = timestamp.format("%Y-%m-%d_%H-%M-%S").to_string();
    Ok(format!(
        "{}/{}_{}.mp4",
        user_dir, slugified_username, timestamp_str
    ))
}

async fn build_recording_args(stream_info: &StreamInfo, output_path: &str) -> Vec<String> {
    let config = Config::get();
    let video_quality = config.get_video_quality();
    let video_bitrate = config.get_video_bitrate();
    let max_bitrate = config.get_max_bitrate();
    let max_fps = config.get_max_fps();
    let encoding = match video_bitrate {
        Some(bitrate) => VideoEncoding::ConstantBitrate(bitrate.to_string()),
        None => VideoEncoding::Quality(video_quality),
    };
    let hw_encoder = detect_best_hw_encoder(&encoding).await;

    build_ffmpeg_args(
        &stream_info.extracted.playback_url,
        output_path,
        &encoding,
        hw_encoder,
        max_bitrate,
        max_fps,
    )
}

async fn refresh_stream_info(stream_info: &mut StreamInfo, token: &str) {
    match run_pipeline(&stream_info.username, &stream_info.platform, token).await {
        Ok(PipelineOutcome::Live(vars)) => {
            let updated_fields = stream_info.refresh_from_pipeline(&vars);
            if !updated_fields.is_empty() {
                println!(
                    "Refreshed stream metadata for {}: {}",
                    stream_info.username,
                    updated_fields.join(", ")
                );
            }
        }
        Ok(PipelineOutcome::Offline) => {}
        Err(error) => eprintln!(
            "Error refreshing stream metadata for {}: {}",
            stream_info.username, error
        ),
    }
}
