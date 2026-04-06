use super::{StreamResult, postprocess::post_process_stream, types::StreamInfo};
use crate::config::Config;
use crate::stream::encoding::{VideoEncoding, build_ffmpeg_args, detect_best_hw_encoder};
use crate::stream::messages::send_recording_start_webhook;
use crate::utils::slugify;
use chrono::{DateTime, Utc};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};

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
}

impl Drop for StreamRecorder {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// Core recording logic that starts ffmpeg and waits for the stream to end.
pub async fn record_stream_raw(
    stream_info: StreamInfo,
    continuation: bool,
) -> StreamResult<(StreamInfo, String)> {
    println!(
        "Starting recording for stream: {} (title: {})",
        stream_info.username,
        stream_info
            .extracted
            .stream_title
            .clone()
            .unwrap_or_default()
    );

    let output_path = build_output_path(&stream_info.username)?;

    if !continuation {
        let webhook_url = Config::get().get_discord_webhook_url();
        if let Err(error) = send_recording_start_webhook(webhook_url, &stream_info).await {
            eprintln!("Error sending start webhook: {}", error);
        }
    }

    let ffmpeg_args = build_recording_args(&stream_info, &output_path).await;
    let mut command = tokio::process::Command::new("ffmpeg");
    command.args(&ffmpeg_args);

    let recorder = StreamRecorder::new(&mut command).await?;
    recorder.wait().await?;

    Ok((stream_info, output_path))
}

/// Records a stream and schedules post-processing on a separate task.
pub async fn record_stream(stream_info: StreamInfo) -> StreamResult<(StreamInfo, String)> {
    let (stream_info, output_path) = record_stream_raw(stream_info, false).await?;

    let stream_info_clone = stream_info.clone();
    let output_path_clone = output_path.clone();
    tokio::spawn(async move {
        if let Err(error) = post_process_stream(stream_info_clone, output_path_clone).await {
            eprintln!("Error post-processing stream: {}", error);
        }
    });

    Ok((stream_info, output_path))
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
