use crate::config::Config;
use crate::extension::context::{ErrorContext, RecordingContext};
use crate::extension::{hooks, store};
use crate::platform::{PipelineOutcome, PlatformConfig};
use crate::stream::api::run_pipeline;
use crate::stream::messages::send_program_error_webhook;
use crate::stream::postprocess::post_process_session;
use crate::stream::recording::record_segment;
use crate::types::DurationValue;
use std::time::Duration;
use tokio::time::{Instant, sleep};

pub use crate::stream::types::StreamInfo;

/// Monitors the stream for a specific user.
/// Runs a polling loop: records the stream when live; waits when offline.
/// If `stream_reconnect_delay` is configured, waits after each
/// recording to detect a continuation and merges all segments into one session.
pub async fn monitor_stream(username: &str, platform: &PlatformConfig, token: &str) {
    loop {
        let fetch_interval = Config::get().get_fetch_interval();
        match run_pipeline(username, platform, token).await {
            Ok(PipelineOutcome::Live(vars)) => {
                match StreamInfo::from_pipeline(username, platform, &vars) {
                    Some(stream_info) => record_session(stream_info, token).await,
                    None => {
                        let message = "Pipeline returned Live but 'playback_url' was not extracted";
                        eprintln!("{message}");
                        let config = Config::get();
                        send_program_error_webhook(
                            config.get_discord_webhook_url().as_deref(),
                            "Live stream metadata was incomplete",
                            &format!(
                                "Platform `{}` reported `{}` as live, but the pipeline did not extract `playback_url`.",
                                platform.id, username
                            ),
                        )
                        .await;
                        hooks::on_error(None, None, ErrorContext::new("pipeline", message), None)
                            .await;
                    }
                }
            }
            Ok(PipelineOutcome::Offline) => {}
            Err(error) => {
                eprintln!("Error running pipeline for {}: {}", username, error);
                let config = Config::get();
                send_program_error_webhook(
                    config.get_discord_webhook_url().as_deref(),
                    "Pipeline error",
                    &format!(
                        "Running the pipeline for `{}` on platform `{}` failed.\n\n{}",
                        username, platform.id, error
                    ),
                )
                .await;
                hooks::on_error(
                    None,
                    None,
                    ErrorContext::new(
                        "pipeline",
                        format!(
                            "Running the pipeline for `{}` on platform `{}` failed",
                            username, platform.id
                        ),
                    ),
                    None,
                )
                .await;
            }
        }

        sleep(fetch_interval).await;
    }
}

/// Records one or more stream segments then spawns post-processing on the full session.
async fn record_session(stream_info: StreamInfo, token: &str) {
    let username = stream_info.username.clone();
    let platform = stream_info.platform.clone();
    let mut session_files = Vec::new();
    let mut session_info = stream_info.clone();
    let mut current_info = stream_info;
    let mut is_continuation = false;
    let session_id = uuid::Uuid::new_v4().to_string();
    store::ensure_session(&session_id);

    loop {
        match record_segment(&mut current_info, token, is_continuation, &session_id).await {
            Ok(Some(path)) => {
                session_files.push(path);
                session_info = current_info.clone();
            }
            Ok(None) => {
                hooks::cleanup_session(&session_id);
                return;
            }
            Err(error) => {
                let label = if is_continuation {
                    "continuation"
                } else {
                    "stream"
                };
                eprintln!("Error recording {} for {}: {}", label, username, error);
                let config = Config::get();
                send_program_error_webhook(
                    config.get_discord_webhook_url().as_deref(),
                    "Recording failed",
                    &format!(
                        "Recording the {} for `{}` on platform `{}` failed.\n\n{}",
                        label, username, platform.id, error
                    ),
                )
                .await;
                hooks::on_error(
                    Some(&session_id),
                    Some(&current_info),
                    ErrorContext::new(
                        "recording",
                        format!("Recording the {} failed: {}", label, error),
                    ),
                    Some(RecordingContext {
                        is_continuation: Some(is_continuation),
                        ..RecordingContext::default()
                    }),
                )
                .await;
                break;
            }
        }

        let Some(delay) = Config::get().get_stream_reconnect_delay() else {
            break;
        };

        let segment_label = if is_continuation {
            "Continuation stream"
        } else {
            "Stream"
        };
        println!(
            "{} ended for {}. Waiting up to {} for a continuation...",
            segment_label,
            username,
            DurationValue::from(delay)
        );

        let fetch_interval = Config::get().get_fetch_interval();
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

    let error_info = session_info.clone();
    let error_session_id = session_id.clone();
    tokio::spawn(async move {
        if let Err(error) = post_process_session(session_info, session_files, session_id).await {
            eprintln!("Error post-processing session: {}", error);
            let config = Config::get();
            send_program_error_webhook(
                config.get_discord_webhook_url().as_deref(),
                "Post-processing failed",
                &format!(
                    "Post-processing failed for `{}` on platform `{}`.\n\n{}",
                    username, platform.id, error
                ),
            )
            .await;
            hooks::on_error(
                Some(&error_session_id),
                Some(&error_info),
                ErrorContext::new("postprocess", format!("Post-processing failed: {}", error)),
                None,
            )
            .await;
        }
    });
}

/// Polls the platform pipeline until the stream goes live again or the deadline passes.
async fn next_live_stream(
    username: &str,
    platform: &PlatformConfig,
    token: &str,
    fetch_interval: Duration,
    reconnect_window: Duration,
) -> Option<StreamInfo> {
    let deadline = Instant::now() + reconnect_window;

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
                let config = Config::get();
                send_program_error_webhook(
                    config.get_discord_webhook_url().as_deref(),
                    "Continuation stream metadata was incomplete",
                    &format!(
                        "Platform `{}` reported `{}` as live during the continuation window, but the pipeline did not extract `playback_url`.",
                        platform.id, username
                    ),
                )
                .await;
                hooks::on_error(
                    None,
                    None,
                    ErrorContext::new(
                        "pipeline",
                        format!(
                            "Pipeline returned Live for `{}` on `{}` without playback_url during the continuation window",
                            username, platform.id
                        ),
                    ),
                    None,
                )
                .await;
            }
            Ok(PipelineOutcome::Offline) => {}
            Err(error) => {
                eprintln!(
                    "Error running pipeline for {} during continuation window: {}",
                    username, error
                );
                let config = Config::get();
                send_program_error_webhook(
                    config.get_discord_webhook_url().as_deref(),
                    "Pipeline error during continuation window",
                    &format!(
                        "Running the pipeline for `{}` on platform `{}` failed during the continuation window.\n\n{}",
                        username, platform.id, error
                    ),
                )
                .await;
                hooks::on_error(
                    None,
                    None,
                    ErrorContext::new(
                        "pipeline",
                        format!(
                            "Pipeline failed during the continuation window for platform `{}`",
                            platform.id
                        ),
                    ),
                    None,
                )
                .await;
            }
        }
    }
}
