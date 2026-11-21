use crate::{
    config::Config,
    stream::messages::send_program_error_webhook,
    uploaders::{UploadError, Uploader, UploaderKindFilter, build_uploaders},
};
use anyhow::Result;
use clap::Subcommand;
use std::{collections::HashMap, time::Duration};
use tiny_table::{Cell, Column, ColumnWidth, Table};
use tokio::time::sleep;
use upio::UploaderEndpointConfig;
use upio::preprocess::{
    PreprocessConfig, PreprocessRule, PreprocessStrategy, cleanup, preprocess_file,
};

/// Split a recording that exceeds the destination's limit into parts the
/// service accepts. `upio` skips the rule when ffmpeg is absent rather than
/// failing, so an oversized file is still attempted and the service rejects it.
fn split_config() -> PreprocessConfig {
    let mut rules = Vec::new();
    if let Some(strategy) = PreprocessStrategy::by_id("split_video") {
        rules.push(PreprocessRule::new(strategy));
    }

    PreprocessConfig {
        enabled: true,
        rules,
    }
}

#[derive(Subcommand)]
pub enum UploadAction {
    /// Upload a file
    File {
        /// Path to the file to upload
        file: String,
        /// Only upload to these services, see `list` command for available uploaders (can be specified multiple times or comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        uploader: Vec<String>,
    },
    /// List available uploaders
    #[clap(alias = "ls")]
    List,
}

fn guess_uploader_kind(file: &str) -> UploaderKindFilter {
    let ext = std::path::Path::new(file)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some(
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "tif" | "svg" | "ico"
            | "heic" | "heif" | "avif",
        ) => UploaderKindFilter::Image,
        Some(
            "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "m4v" | "3gp" | "mpg" | "mpeg"
            | "ts" | "mts" | "ogv",
        ) => UploaderKindFilter::Video,
        _ => UploaderKindFilter::All,
    }
}

pub async fn handle_upload_command(file: String, uploader: Vec<String>) -> Result<()> {
    if !std::path::Path::new(&file).is_file() {
        return Err(anyhow::anyhow!(
            "File not found or is not a regular file: {}",
            file
        ));
    }

    let max_retries = Config::get().get_max_upload_retries();
    let filter = if uploader.is_empty() {
        guess_uploader_kind(&file)
    } else {
        UploaderKindFilter::All
    };
    let uploaders = build_uploaders(filter);

    let mut matched = false;
    let mut upload_results: HashMap<String, Vec<String>> = HashMap::new();

    for (up, up_config) in &uploaders {
        if !uploader.is_empty() && !uploader.iter().any(|n| up.name().eq_ignore_ascii_case(n)) {
            continue;
        }
        matched = true;
        let _ = try_upload(
            up.as_ref(),
            &file,
            up_config,
            &mut upload_results,
            max_retries,
        )
        .await;
    }

    if !matched {
        if !uploader.is_empty() {
            return Err(anyhow::anyhow!(
                "No configured uploader matches: {}",
                uploader.join(", ")
            ));
        }
        return Err(anyhow::anyhow!("No uploaders are configured"));
    }

    let mut upload_table = Table::with_columns(vec![
        Column::new("Uploader").max_width(0.2),
        Column::new("URLs").max_width(ColumnWidth::fill()),
    ]);

    for (uploader, urls) in &upload_results {
        upload_table.add_row(vec![Cell::new(uploader), Cell::new(urls.join(", "))]);
    }

    upload_table.print();
    Ok(())
}

#[derive(Debug, Clone)]
pub struct UploadFailure {
    pub uploader: String,
    pub file: String,
    pub status_code: Option<u16>,
    pub retryable: bool,
}

/// Uploads a file to one service, splitting it first when it exceeds that
/// service's size limit, and retrying on server errors (5xx) with exponential
/// backoff. Returns one failure per part that did not upload.
pub async fn try_upload(
    uploader: &dyn Uploader,
    file_path: &str,
    config: &UploaderEndpointConfig,
    upload_results: &mut HashMap<String, Vec<String>>,
    max_retries: u32,
) -> Vec<UploadFailure> {
    // Token verification and capability discovery happen once per service, not
    // once per part, so a split upload does not re-authenticate per segment.
    if let Err(e) = uploader.init().await {
        eprintln!("{} init failed: {}", uploader.name(), e);
        return vec![UploadFailure {
            uploader: uploader.name().to_string(),
            file: file_path.to_string(),
            status_code: None,
            retryable: false,
        }];
    }

    let prepared = match preprocess_file(
        file_path,
        uploader.max_file_size().as_bytes(),
        &split_config(),
    )
    .await
    {
        Ok(prepared) => prepared,
        Err(e) => {
            eprintln!("{} preprocessing failed: {}", uploader.name(), e);
            let config = Config::get();
            send_program_error_webhook(
                config.get_discord_webhook_url().as_deref(),
                "Upload preprocessing failed",
                &format!(
                    "Preprocessing for upload to `{}` failed for file `{}`.\n\n{}",
                    uploader.name(),
                    file_path,
                    e,
                ),
            )
            .await;
            return vec![UploadFailure {
                uploader: uploader.name().to_string(),
                file: file_path.to_string(),
                status_code: None,
                retryable: false,
            }];
        }
    };

    let mut failures = Vec::new();
    for part in &prepared.files {
        if let Some(failure) = try_upload_single(
            uploader,
            &part.to_string_lossy(),
            config,
            upload_results,
            max_retries,
        )
        .await
        {
            failures.push(failure);
        }
    }

    if let Err(e) = cleanup(&prepared) {
        eprintln!("{} cleanup of split parts failed: {}", uploader.name(), e);
    }

    failures
}

async fn try_upload_single(
    uploader: &dyn Uploader,
    file_path: &str,
    config: &UploaderEndpointConfig,
    upload_results: &mut HashMap<String, Vec<String>>,
    max_retries: u32,
) -> Option<UploadFailure> {
    let mut retry_count = 0;

    loop {
        match uploader.upload_file(file_path, config).await {
            Ok(result) => {
                if !result.urls.is_empty() {
                    upload_results
                        .entry(uploader.name().to_string())
                        .or_default()
                        .extend(result.urls);
                    if retry_count > 0 {
                        println!(
                            "{} upload succeeded on retry attempt {}",
                            uploader.name(),
                            retry_count
                        );
                    }
                } else {
                    eprintln!("{} upload succeeded but no URLs found", uploader.name());
                }
                return None; // Success - exit retry loop
            }
            Err(UploadError {
                status_code,
                message,
            }) => {
                if retry_count == 0 {
                    eprintln!("{} upload failed: {}", uploader.name(), message);
                } else {
                    eprintln!(
                        "{} upload failed on retry attempt {}: {}",
                        uploader.name(),
                        retry_count,
                        message
                    );
                }

                if is_server_error(status_code) && retry_count < max_retries {
                    let backoff = calculate_backoff(retry_count + 1);
                    println!(
                        "Retrying {} upload (attempt {}/{}) after {} seconds: {}",
                        uploader.name(),
                        retry_count + 1,
                        max_retries,
                        backoff.as_secs(),
                        file_path
                    );
                    sleep(backoff).await;
                    retry_count += 1;
                } else {
                    if retry_count >= max_retries {
                        eprintln!(
                            "Max retries ({}) exceeded for {} upload: {}",
                            max_retries,
                            uploader.name(),
                            file_path
                        );
                        let config = Config::get();
                        send_program_error_webhook(
                            config.get_discord_webhook_url().as_deref(),
                            "Upload failed after all retries",
                            &format!(
                                "Upload to `{}` for file `{}` failed after {} retries.\n\n{}",
                                uploader.name(),
                                file_path,
                                max_retries,
                                message,
                            ),
                        )
                        .await;
                    }
                    return Some(UploadFailure {
                        uploader: uploader.name().to_string(),
                        file: file_path.to_string(),
                        status_code,
                        retryable: is_server_error(status_code),
                    });
                }
            }
        }
    }
}

/// Handle the list uploaders command
pub async fn handle_list_command() -> Result<()> {
    let uploaders = build_uploaders(UploaderKindFilter::All);
    for (uploader, _) in uploaders {
        println!("{}", uploader.name());
    }
    Ok(())
}

/// Checks if an HTTP status code is a 5xx server error
fn is_server_error(status_code: Option<u16>) -> bool {
    status_code.is_some_and(|code| (500..600).contains(&code))
}

/// Calculates exponential backoff duration
/// Uses long backoff times for server errors (starting at 5 minutes)
fn calculate_backoff(attempt: u32) -> Duration {
    let base_seconds = 600;
    let backoff_seconds = base_seconds * 2_u64.pow(attempt - 1);
    Duration::from_secs(backoff_seconds.min(3600)) // Cap at 1 hour
}
