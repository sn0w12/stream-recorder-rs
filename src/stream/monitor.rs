use crate::config::Config;
use crate::platform::{PipelineOutcome, PlatformConfig};
use crate::stream::api::run_pipeline;
use crate::stream::postprocess::post_process_session;
use crate::stream::recording::{record_stream, record_stream_raw};
use std::time::Duration;
use tokio::time::{Instant, sleep};

pub use crate::stream::types::StreamInfo;

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
        if matches!(
            monitor_once(username, platform, token, fetch_interval).await,
            MonitorCycle::WaitForNextPoll
        ) {
            sleep(fetch_interval).await;
        }
    }
}

enum MonitorCycle {
    WaitForNextPoll,
    SkipPollDelay,
}

async fn monitor_once(
    username: &str,
    platform: &PlatformConfig,
    token: &str,
    fetch_interval: Duration,
) -> MonitorCycle {
    match run_pipeline(username, platform, token).await {
        Ok(PipelineOutcome::Live(vars)) => {
            handle_live_stream(username, platform, token, fetch_interval, vars).await
        }
        Ok(PipelineOutcome::Offline) => MonitorCycle::WaitForNextPoll,
        Err(error) => {
            eprintln!("Error running pipeline for {}: {}", username, error);
            MonitorCycle::WaitForNextPoll
        }
    }
}

async fn handle_live_stream(
    username: &str,
    platform: &PlatformConfig,
    token: &str,
    fetch_interval: Duration,
    vars: std::collections::HashMap<String, String>,
) -> MonitorCycle {
    let Some(stream_info) = StreamInfo::from_pipeline(username, platform, &vars) else {
        eprintln!("Pipeline returned Live but 'playback_url' was not extracted");
        return MonitorCycle::WaitForNextPoll;
    };

    match Config::get().get_stream_reconnect_delay_minutes() {
        Some(delay_minutes) => {
            record_session_with_continuations(stream_info, token, fetch_interval, delay_minutes)
                .await
        }
        None => {
            if let Err(error) = record_stream(stream_info).await {
                eprintln!("Error recording stream: {}", error);
            }
            MonitorCycle::WaitForNextPoll
        }
    }
}

async fn record_session_with_continuations(
    stream_info: StreamInfo,
    token: &str,
    fetch_interval: Duration,
    delay_minutes: f64,
) -> MonitorCycle {
    let primary_stream_info = stream_info.clone();
    let username = primary_stream_info.username.clone();
    let platform = primary_stream_info.platform.clone();

    match record_stream_raw(stream_info, false).await {
        Ok((_, first_path)) => {
            let session_files = collect_session_files(
                &username,
                &platform,
                token,
                fetch_interval,
                delay_minutes,
                first_path,
            )
            .await;
            spawn_session_post_processing(primary_stream_info, session_files);
            MonitorCycle::SkipPollDelay
        }
        Err(error) => {
            eprintln!("Error recording stream for {}: {}", username, error);
            MonitorCycle::WaitForNextPoll
        }
    }
}

async fn collect_session_files(
    username: &str,
    platform: &PlatformConfig,
    token: &str,
    fetch_interval: Duration,
    delay_minutes: f64,
    first_path: String,
) -> Vec<String> {
    let mut session_files = vec![first_path];
    let mut deadline = continuation_deadline(delay_minutes);

    println!(
        "Stream ended for {}. Waiting up to {:.0} minute(s) for a continuation before post-processing...",
        username, delay_minutes
    );

    while let Some(continuation_info) =
        poll_for_continuation(username, platform, token, fetch_interval, deadline).await
    {
        println!(
            "Continuation stream detected for {}, recording...",
            username
        );

        match record_stream_raw(continuation_info, true).await {
            Ok((_, continuation_path)) => {
                session_files.push(continuation_path);
                deadline = continuation_deadline(delay_minutes);
                println!(
                    "Continuation stream ended for {}. Waiting up to {:.0} minute(s) for another continuation...",
                    username, delay_minutes
                );
            }
            Err(error) => {
                eprintln!(
                    "Error recording continuation stream for {}: {}",
                    username, error
                );
            }
        }
    }

    session_files
}

async fn poll_for_continuation(
    username: &str,
    platform: &PlatformConfig,
    token: &str,
    fetch_interval: Duration,
    deadline: Instant,
) -> Option<StreamInfo> {
    loop {
        let now = Instant::now();
        if now >= deadline {
            return None;
        }

        let sleep_duration = fetch_interval.min(deadline - now);
        sleep(sleep_duration).await;

        match run_pipeline(username, platform, token).await {
            Ok(PipelineOutcome::Live(vars)) => {
                if let Some(stream_info) = StreamInfo::from_pipeline(username, platform, &vars) {
                    return Some(stream_info);
                }

                eprintln!("Pipeline returned Live but 'playback_url' was not extracted");
            }
            Ok(PipelineOutcome::Offline) => {}
            Err(error) => {
                eprintln!(
                    "Error running pipeline for {} during continuation window: {}",
                    username, error
                );
            }
        }
    }
}

fn continuation_deadline(delay_minutes: f64) -> Instant {
    Instant::now() + Duration::from_secs_f64(delay_minutes.max(0.0) * 60.0)
}

fn spawn_session_post_processing(stream_info: StreamInfo, files: Vec<String>) {
    tokio::spawn(async move {
        if let Err(error) = post_process_session(stream_info, files).await {
            eprintln!("Error post-processing session: {}", error);
        }
    });
}
