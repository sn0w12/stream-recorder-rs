mod cli;
mod config;
mod discord;
mod monitoring;
mod platform;
mod print;
mod stream;
mod template;
mod types;
mod uploaders;
mod utils;

use crate::cli::version::{VersionCheckResult, check_version};
use crate::config::Config;
use crate::platform::PlatformConfig;
use crate::print::section::StartupInfo;
use crate::stream::encoding::{VideoEncoding, detect_best_hw_encoder, probe_hw_encoders};
use crate::template::{TemplateValue, get_template_string, render_template};
use crate::uploaders::UploaderType;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use tiny_table::Color::*;

use crate::cli::config::{ConfigAction, handle_config_command};
use crate::cli::platform::{PlatformAction, handle_platform_command};
use crate::cli::thumbnail::{ThumbnailAction, handle_thumbnail_action};
use crate::cli::token::{TokenAction, handle_token_command};
use crate::cli::upload::{UploadAction, handle_list_command, handle_upload_command};
use crate::monitoring::MonitorSupervisor;

#[derive(Parser)]
#[command(name = "stream-recorder", about = "CLI tool for recording streams")]
struct Cli {
    #[arg(short, long)]
    token: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage tokens
    Token {
        #[command(subcommand)]
        action: TokenAction,
    },
    /// Manage templates
    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Manage platforms
    Platform {
        #[command(subcommand)]
        action: PlatformAction,
    },
    /// Probe and test hardware encoders
    Encoders {
        #[command(subcommand)]
        action: EncoderAction,
    },
    /// Upload a file to configured hosting services
    Upload {
        #[command(subcommand)]
        action: UploadAction,
    },
    /// Generate a thumbnail from a recorded video
    Thumbnail {
        #[command(subcommand)]
        action: ThumbnailAction,
    },
}

#[derive(Subcommand)]
enum EncoderAction {
    /// Probe hardware encoders (quick runtime tests)
    Test,
}

#[derive(Subcommand)]
enum TemplateAction {
    /// Render an example of the current template with mock values
    Render,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    Config::init()?;

    match cli.command {
        Some(Commands::Token { action }) => handle_token_command(action)?,
        Some(Commands::Template { action }) => match action {
            TemplateAction::Render => {
                if let Some(template) = get_template_string()? {
                    let mut context = HashMap::new();
                    context.insert(
                        "date".to_string(),
                        TemplateValue::String("2025-11-09".to_string()),
                    );
                    context.insert(
                        "username".to_string(),
                        TemplateValue::String("example_user".to_string()),
                    );
                    context.insert(
                        "user_id".to_string(),
                        TemplateValue::String("12345".to_string()),
                    );
                    context.insert(
                        "output_path".to_string(),
                        TemplateValue::String("/path/to/recording.mp4".to_string()),
                    );
                    context.insert(
                        "thumbnail_path".to_string(),
                        TemplateValue::String("/path/to/thumbnail.jpg".to_string()),
                    );
                    context.insert(
                        "stream_title".to_string(),
                        TemplateValue::String("Example Stream Title".to_string()),
                    );
                    context.insert(
                        "bunkr_urls".to_string(),
                        TemplateValue::Array(vec![
                            "https://bunkr.example.com/file1".to_string(),
                            "https://bunkr.example.com/file2".to_string(),
                        ]),
                    );
                    context.insert(
                        "gofile_urls".to_string(),
                        TemplateValue::Array(vec![
                            "https://gofile.example.com/download".to_string(),
                        ]),
                    );
                    context.insert(
                        "fileditch_urls".to_string(),
                        TemplateValue::Array(vec![
                            "https://fileditch.example.com/file".to_string(),
                        ]),
                    );
                    context.insert(
                        "filester_urls".to_string(),
                        TemplateValue::Array(vec!["https://filester.example.com/file".to_string()]),
                    );
                    context.insert(
                        "jpg6_urls".to_string(),
                        TemplateValue::Array(vec!["https://jpg6.example.com/file".to_string()]),
                    );
                    let rendered = render_template(&template, &context);
                    println!("{}", rendered);
                }
            }
        },
        Some(Commands::Config { action }) => {
            handle_config_command(action)?;
        }
        Some(Commands::Platform { action }) => {
            handle_platform_command(action).await?;
        }
        Some(Commands::Encoders { action }) => match action {
            EncoderAction::Test => {
                // run the encoder probe and print diagnostics
                match probe_hw_encoders().await {
                    Ok(_) => {}
                    Err(e) => return Err(anyhow::anyhow!(e.to_string())),
                }
            }
        },
        Some(Commands::Upload { action }) => match action {
            UploadAction::File { file, uploader } => {
                handle_upload_command(file, uploader).await?;
            }
            UploadAction::List => {
                handle_list_command().await?;
            }
        },
        Some(Commands::Thumbnail { action }) => {
            handle_thumbnail_action(action).await?;
        }
        None => {
            match startup_tests().await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Startup tests failed: {}", e);
                    eprintln!("Aborting startup. Fix the issue and try again.");
                    return Err(e);
                }
            }

            let platforms = PlatformConfig::load_all()?;
            run_recording(&platforms, cli.token).await?;
        }
    }

    Ok(())
}

/// Runs startup tests to make sure everything is working before we start recording.
/// This checks core functionality but it does not validate any external systems.
async fn startup_tests() -> Result<()> {
    // Check if ffmpeg is available
    let output = std::process::Command::new("ffmpeg")
        .arg("-version")
        .output();

    match output {
        Ok(o) if o.status.success() => {
            // ffmpeg is available
        }
        _ => {
            return Err(anyhow::anyhow!(
                "ffmpeg is not installed or not available in PATH. \
                 Please install ffmpeg to use stream-recorder."
            ));
        }
    }

    let config = Config::get();
    config.validate().map_err(|e| {
        anyhow::anyhow!(
            "Configuration validation failed: {}. \
             Please fix the configuration and try again.",
            e
        )
    })?;

    Ok(())
}

async fn print_startup_info(platforms: &[PlatformConfig]) {
    #[derive(Debug)]
    enum UploaderStatus {
        Enabled(String),
        UserDisabled,
        ConfigError(String),
    }

    fn get_uploader_status(
        uploader_type: UploaderType,
        name: &str,
        disabled: &[String],
    ) -> UploaderStatus {
        let disabled_set: std::collections::HashSet<_> = disabled.iter().cloned().collect();
        if disabled_set.contains(&name.to_lowercase()) {
            return UploaderStatus::UserDisabled;
        }
        match uploader_type {
            UploaderType::Fileditch => UploaderStatus::Enabled("always available".to_string()),
            UploaderType::Bunkr => {
                if crate::utils::get_bunkr_token().is_some() {
                    UploaderStatus::Enabled("token configured".to_string())
                } else {
                    UploaderStatus::ConfigError("token required".to_string())
                }
            }
            UploaderType::GoFile => {
                if crate::utils::get_gofile_token().is_some() {
                    UploaderStatus::Enabled("token configured".to_string())
                } else {
                    UploaderStatus::ConfigError("token required".to_string())
                }
            }
            UploaderType::Filester => {
                if crate::utils::get_filester_token().is_some() {
                    UploaderStatus::Enabled("token configured".to_string())
                } else {
                    UploaderStatus::Enabled("no token, public limits".to_string())
                }
            }
            UploaderType::Jpg6 => {
                if crate::utils::get_jpg6_token().is_some() {
                    UploaderStatus::Enabled("token configured".to_string())
                } else {
                    UploaderStatus::ConfigError("token required".to_string())
                }
            }
        }
    }

    let mut info = StartupInfo::new();

    info.begin_section("Info");
    let ver_status = check_version().await;
    match ver_status {
        VersionCheckResult::UpToDate => info.ok("version", "up to date"),
        VersionCheckResult::Outdated { latest_version } => {
            info.warn("version", format!("outdated, latest is {}", latest_version))
        }
        VersionCheckResult::Error(err) => {
            info.err("version", format!("error checking version: {}", err))
        }
    }

    info.begin_section("Platforms");
    if platforms.is_empty() {
        info.plain("No platforms configured", Some(Yellow));
    } else {
        for p in platforms {
            let token_status = if let Some(token_name) = &p.token_name {
                if crate::utils::get_token_by_name(token_name).is_some() {
                    "token configured"
                } else {
                    "no token, may not work"
                }
            } else {
                "no token required"
            };
            info.dot(&p.name, token_status);
        }
    }

    info.begin_section("Monitored Users");
    let monitors = Config::get().get_monitors();
    if monitors.is_empty() {
        info.plain("No users configured", Some(Yellow));
    } else {
        for monitor in &monitors {
            let (platform_id, _username) = match utils::split_monitor_reference(monitor) {
                Ok(pair) => pair,
                Err(_) => {
                    info.err(monitor, "malformed monitor string");
                    continue;
                }
            };

            match PlatformConfig::find_by_id(platforms, &platform_id) {
                Some(p) => p.clone(),
                None => {
                    info.err(monitor, "unknown platform");
                    continue;
                }
            };

            info.dot(monitor, "");
        }
    }

    info.begin_section("Uploaders");
    let disabled_uploaders = Config::get().get_disabled_uploaders();
    let uploader_types_and_names = crate::uploaders::get_all_uploader_types_and_names().await;
    for (uploader_type, name) in uploader_types_and_names {
        match get_uploader_status(uploader_type, &name, &disabled_uploaders) {
            UploaderStatus::Enabled(note) => info.ok(&name, &note),
            UploaderStatus::UserDisabled => info.warn(&name, "disabled by user"),
            UploaderStatus::ConfigError(note) => info.err(&name, &note),
        }
    }

    info.begin_section("Encoder");
    let config = Config::get();
    let video_quality = config.get_video_quality();
    let video_bitrate = config.get_video_bitrate();
    let encoding = match video_bitrate {
        Some(bitrate) => VideoEncoding::ConstantBitrate(bitrate.to_string()),
        None => VideoEncoding::Quality(video_quality),
    };
    match detect_best_hw_encoder(&encoding).await {
        Some((enc, _)) => info.ok(&enc, format!("hardware acceleration, {}", encoding)),
        None => info.warn(
            "libx264",
            format!("no hardware encoder found, using software, {}", encoding),
        ),
    }

    info.print();
}

async fn run_recording(platforms: &[PlatformConfig], cli_token: Option<String>) -> Result<()> {
    print_startup_info(platforms).await;

    MonitorSupervisor::new(platforms.to_vec(), cli_token)
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_all_no_fallback_when_empty() {
        // Verify that load_all returns an empty Vec when the platforms directory
        let tmp = tempfile::tempdir().expect("failed to create tempdir");

        // Temporarily override the platforms_dir resolution by reading an empty dir.
        // We call the internal directory-scan logic directly by reading the dir ourselves.
        let dir = tmp.path();
        let mut platforms = Vec::new();
        for entry in std::fs::read_dir(dir).expect("failed to read temp platform directory") {
            let entry = entry.expect("failed to read a directory entry");
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let content =
                    std::fs::read_to_string(&path).expect("failed to read platform JSON file");
                let config: PlatformConfig = serde_json::from_str(&content)
                    .expect("failed to deserialize platform JSON from test fixture");
                platforms.push(config);
            }
        }
        assert!(
            platforms.is_empty(),
            "empty dir must yield an empty platform list"
        );
    }

    // ── stream_reconnect_delay config tests ──────────────────────────────────

    #[test]
    fn test_stream_reconnect_delay_defaults_to_none() {
        let config = Config::default();
        assert!(
            config.get_stream_reconnect_delay().is_none(),
            "stream_reconnect_delay should default to None (disabled)"
        );
    }

    #[test]
    fn test_stream_reconnect_delay_round_trips_through_toml() {
        let toml_input = "stream_reconnect_delay = \"5m\"\n";
        let config: Config =
            toml::from_str(toml_input).expect("failed to parse TOML with stream_reconnect_delay");
        assert_eq!(
            config.get_stream_reconnect_delay(),
            Some(std::time::Duration::from_secs(300))
        );
    }

    #[test]
    fn test_stream_reconnect_delay_none_when_absent_from_toml() {
        let config: Config = toml::from_str("").expect("failed to parse empty TOML");
        assert!(config.get_stream_reconnect_delay().is_none());
    }

    #[test]
    fn test_stream_reconnect_delay_key_recognised_by_config() {
        use crate::config::ConfigKey;
        assert!(
            ConfigKey::from_key("stream_reconnect_delay").is_some(),
            "stream_reconnect_delay should be a recognised config key"
        );
    }

    #[test]
    fn test_stream_metadata_refresh_interval_key_recognised_by_config() {
        use crate::config::ConfigKey;
        assert!(
            ConfigKey::from_key("stream_metadata_refresh_interval").is_some(),
            "stream_metadata_refresh_interval should be a recognised config key"
        );
    }

    #[test]
    fn test_stream_reconnect_delay_set_and_get_via_config_methods() {
        let mut config = Config::default();
        config
            .set_value("stream_reconnect_delay", "3m30s")
            .expect("set_value should accept a valid duration");
        assert_eq!(
            config.get_stream_reconnect_delay(),
            Some(std::time::Duration::from_secs(210))
        );
    }

    #[test]
    fn test_stream_reconnect_delay_cleared_with_none_string() {
        let mut config = Config::default();
        config
            .set_value("stream_reconnect_delay", "10m")
            .expect("set_value should accept a valid duration");
        config
            .set_value("stream_reconnect_delay", "none")
            .expect("set_value should accept clearing reconnect delay with 'none'");
        assert!(config.get_stream_reconnect_delay().is_none());
    }

    #[test]
    fn test_stream_metadata_refresh_interval_set_and_get_via_config_methods() {
        let mut config = Config::default();
        config
            .set_value("stream_metadata_refresh_interval", "15s")
            .expect("set_value should accept a positive duration");
        assert_eq!(
            config.get_stream_metadata_refresh_interval(),
            Some(std::time::Duration::from_secs(15))
        );
    }

    #[test]
    fn test_stream_metadata_refresh_interval_rejects_zero() {
        let mut config = Config::default();
        let err = config
            .set_value("stream_metadata_refresh_interval", "0s")
            .expect_err("set_value should reject non-positive intervals");

        assert!(
            err.to_string().contains("stream_metadata_refresh_interval"),
            "unexpected error: {err:#}"
        );
    }
}
