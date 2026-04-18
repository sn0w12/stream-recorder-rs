use crate::config::Config;
use crate::platform::{PipelineOutcome, PlatformConfig};
use crate::stream::api::run_pipeline;
use crate::stream::postprocess::post_process_session;
use crate::stream::recording::record_segment;
use std::time::Duration;
use tokio::time::{Instant, sleep};

pub use crate::stream::types::StreamInfo;

/// Monitors the stream for a specific user.
/// Runs a polling loop: records the stream when live; waits when offline.
/// If `stream_reconnect_delay_minutes` is configured, waits after each
/// recording to detect a continuation and merges all segments into one session.
pub async fn monitor_stream(
    username: &str,
    platform: &PlatformConfig,
    token: &str,
    fetch_interval: Duration,
) {
    loop {
        match run_pipeline(username, platform, token).await {
            Ok(PipelineOutcome::Live(vars)) => {
                match StreamInfo::from_pipeline(username, platform, &vars) {
                    Some(stream_info) => record_session(stream_info, token, fetch_interval).await,
                    None => {
                        eprintln!("Pipeline returned Live but 'playback_url' was not extracted")
                    }
                }
            }
            Ok(PipelineOutcome::Offline) => {}
            Err(error) => eprintln!("Error running pipeline for {}: {}", username, error),
        }

        sleep(fetch_interval).await;
    }
}

/// Records one or more stream segments then spawns post-processing on the full session.
async fn record_session(stream_info: StreamInfo, token: &str, fetch_interval: Duration) {
    let username = stream_info.username.clone();
    let platform = stream_info.platform.clone();
    let delay_minutes = Config::get().get_stream_reconnect_delay_minutes();
    let mut session_files = Vec::new();
    let mut session_info = stream_info.clone();
    let mut current_info = stream_info;
    let mut is_continuation = false;

    loop {
        match record_segment(&mut current_info, token, is_continuation).await {
            Ok(path) => {
                session_files.push(path);
                session_info = current_info.clone();
            }
            Err(error) => {
                let label = if is_continuation {
                    "continuation"
                } else {
                    "stream"
                };
                eprintln!("Error recording {} for {}: {}", label, username, error);
                break;
            }
        }

        let Some(delay) = delay_minutes else { break };

        let segment_label = if is_continuation {
            "Continuation stream"
        } else {
            "Stream"
        };
        println!(
            "{} ended for {}. Waiting up to {:.0} minute(s) for a continuation...",
            segment_label, username, delay
        );

        match next_live_stream(&username, &platform, token, fetch_interval, delay).await {
            Some(next_info) => {
                println!(
                    "Continuation stream detected for {}, recording...",
                    username
                );
                current_info = next_info;
                is_continuation = true;
            }
            None => break,
        }
    }

    tokio::spawn(async move {
        if let Err(error) = post_process_session(session_info, session_files).await {
            eprintln!("Error post-processing session: {}", error);
        }
    });
}

/// Polls the platform pipeline until the stream goes live again or the deadline passes.
async fn next_live_stream(
    username: &str,
    platform: &PlatformConfig,
    token: &str,
    fetch_interval: Duration,
    delay_minutes: f64,
) -> Option<StreamInfo> {
    let deadline = Instant::now() + Duration::from_secs_f64(delay_minutes.max(0.0) * 60.0);

    loop {
        let now = Instant::now();
        if now >= deadline {
            return None;
        }

        sleep(fetch_interval.min(deadline - now)).await;

        match run_pipeline(username, platform, token).await {
            Ok(PipelineOutcome::Live(vars)) => {
                if let Some(info) = StreamInfo::from_pipeline(username, platform, &vars) {
                    return Some(info);
                }
                eprintln!("Pipeline returned Live but 'playback_url' was not extracted");
            }
            Ok(PipelineOutcome::Offline) => {}
            Err(error) => eprintln!(
                "Error running pipeline for {} during continuation window: {}",
                username, error
            ),
        }
    }
}
