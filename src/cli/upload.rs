use crate::uploaders::{build_uploaders, Uploader, UploaderConfig};
use anyhow::Result;
use clap::Subcommand;
use std::{collections::HashMap, time::Duration};
use tokio::time::sleep;

#[derive(Subcommand)]
pub enum UploadAction {
    /// Upload a file
    File {
        /// Path to the file to upload
        file: String,
        /// Only upload to this specific service (e.g. bunkr, gofile, fileditch, filester)
        #[arg(short, long)]
        uploader: Option<String>,
    },
    /// List available uploaders
    #[clap(alias = "ls")]
    List,
}

pub async fn handle_upload_command(file: String, uploader: Option<String>) -> Result<()> {
    if !std::path::Path::new(&file).is_file() {
        return Err(anyhow::anyhow!(
            "File not found or is not a regular file: {}",
            file
        ));
    }

    let config = crate::config::Config::load()?;
    let max_retries = config.get_max_upload_retries();
    let uploaders = build_uploaders().await;

    let mut matched = false;
    let mut upload_results: HashMap<String, Vec<String>> = HashMap::new();

    for (up, up_config) in &uploaders {
        if let Some(ref name) = uploader {
            if !up.name().eq_ignore_ascii_case(name) {
                continue;
            }
        }
        matched = true;
        try_upload(
            up.as_ref(),
            &file,
            up_config,
            &mut upload_results,
            max_retries,
        )
        .await;
    }

    if !matched {
        if let Some(name) = uploader {
            return Err(anyhow::anyhow!(
                "No uploader named '{}' is configured",
                name
            ));
        }
        return Err(anyhow::anyhow!("No uploaders are configured"));
    }

    for (name, urls) in &upload_results {
        for url in urls {
            println!("{}: {}", name, url);
        }
    }

    Ok(())
}

/// Helper function to attempt upload with automatic retry logic using the unified Uploader interface.
/// Retries automatically on server errors (5xx) with exponential backoff.
pub async fn try_upload(
    uploader: &dyn Uploader,
    file_path: &str,
    config: &UploaderConfig,
    upload_results: &mut HashMap<String, Vec<String>>,
    max_retries: u32,
) {
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
                return; // Success - exit retry loop
            }
            Err(e) => {
                if retry_count == 0 {
                    eprintln!("{} upload failed: {}", uploader.name(), e);
                } else {
                    eprintln!(
                        "{} upload failed on retry attempt {}: {}",
                        uploader.name(),
                        retry_count,
                        e
                    );
                }

                // Only retry on server errors and if we haven't exceeded max retries
                if is_server_error(e.status_code) && retry_count < max_retries {
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
                    }
                    return; // No more retries - exit
                }
            }
        }
    }
}

/// Handle the list uploaders command
pub async fn handle_list_command() -> Result<()> {
    let uploaders = build_uploaders().await;
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
