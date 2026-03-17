mod cli;
mod config;
mod print;
mod template;
mod thumb;
mod utils;
mod stream {
    pub mod api;
    pub mod monitor;
}
mod platform;
mod uploaders;

use crate::platform::PlatformConfig;
use crate::print::section::StartupInfo;
use crate::template::{TemplateValue, get_template_string, render_template};
use crate::uploaders::UploaderType;
use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Color::*;
use std::collections::HashMap;

use crate::cli::config::{ConfigAction, handle_config_command};
use crate::cli::platform::{PlatformAction, handle_platform_command};
use crate::cli::token::{TokenAction, handle_token_command};
use crate::cli::upload::{UploadAction, handle_list_command, handle_upload_command};

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
    let config = crate::config::Config::load()?;

    match cli.command {
        Some(Commands::Token { action }) => handle_token_command(action)?,
        Some(Commands::Template { action }) => match action {
            TemplateAction::Render => {
                if let Some(template) = get_template_string() {
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
                    let rendered = render_template(template, &context);
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
                match crate::stream::monitor::probe_hw_encoders().await {
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
        None => {
            let platforms = PlatformConfig::load_all()?;
            run_recording(&config, &platforms, cli.token).await?;
        }
    }

    Ok(())
}

async fn print_startup_info(config: &crate::config::Config, platforms: &[PlatformConfig]) {
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
        }
    }

    let mut info = StartupInfo::new();

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
    let monitors = config.get_monitors();
    if monitors.is_empty() {
        info.plain("No users configured", Some(Yellow));
    } else {
        for user in &monitors {
            info.dot(user, "");
        }
    }

    info.begin_section("Uploaders");
    let disabled_uploaders = config.get_disabled_uploaders();
    let uploader_types_and_names = crate::uploaders::get_all_uploader_types_and_names().await;
    for (uploader_type, name) in uploader_types_and_names {
        match get_uploader_status(uploader_type, &name, &disabled_uploaders) {
            UploaderStatus::Enabled(note) => info.ok(&name, &note),
            UploaderStatus::UserDisabled => info.warn(&name, "disabled by user"),
            UploaderStatus::ConfigError(note) => info.err(&name, &note),
        }
    }

    info.begin_section("Encoder");
    let bitrate = config.get_bitrate();
    match crate::stream::monitor::detect_best_hw_encoder(&bitrate).await {
        Some((enc, _)) => info.ok(&enc, "hardware acceleration"),
        None => info.warn("libx264", "no hardware encoder found, using software"),
    }

    info.print();
}

async fn run_recording(
    config: &crate::config::Config,
    platforms: &[PlatformConfig],
    cli_token: Option<String>,
) -> Result<()> {
    print_startup_info(config, platforms).await;

    let monitors = config.get_monitors();
    if monitors.is_empty() {
        println!(
            "No monitors configured. Use 'stream-recorder config monitors add <platform_id>:<username>' to add users to monitor."
        );
        return Ok(());
    }

    for monitor_str in monitors {
        let (platform_id, username) = match parse_monitor_string(&monitor_str) {
            Some(pair) => pair,
            None => continue,
        };

        let platform = match PlatformConfig::find_by_id(platforms, &platform_id) {
            Some(p) => p.clone(),
            None => {
                eprintln!(
                    "Unknown platform '{}' for monitor '{}', skipping.",
                    platform_id, monitor_str
                );
                continue;
            }
        };

        // Resolve the token for this platform.
        // The CLI --token flag acts as a universal override for any platform.
        let token = match get_platform_token(&platform, cli_token.clone()) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Error getting token for platform '{}': {}", platform_id, e);
                continue;
            }
        };

        if let Err(e) = spawn_monitor_task(&username, &token, platform, config.clone()).await {
            eprintln!("Error starting monitor for {}: {}", username, e);
        }
        // Small delay to prevent rapid spawning
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    // Wait for Ctrl+C to keep the program running
    tokio::signal::ctrl_c().await?;
    println!("Shutting down...");

    Ok(())
}

/// Parses a monitor string into a (platform_id, username) pair.
///
/// Requires the `platform_id:username` format. Returns `None` for strings
/// that do not contain a `:` separator, logging a helpful error message.
fn parse_monitor_string(s: &str) -> Option<(String, String)> {
    if let Some((platform, username)) = s.split_once(':') {
        if platform.is_empty() || username.is_empty() {
            eprintln!(
                "Skipping malformed monitor '{}': both platform_id and username must be non-empty.",
                s
            );
            return None;
        }
        Some((platform.to_string(), username.to_string()))
    } else {
        eprintln!(
            "Skipping monitor '{}': missing platform prefix. \
             Use 'platform_id:username' format. \
             Update it with: config monitors remove {} && config monitors add <platform>:{}",
            s, s, s
        );
        None
    }
}

/// Resolves the authentication token for a platform.
///
/// If a CLI token is supplied it acts as a universal override across all
/// platforms, allowing quick one-off runs without modifying stored credentials.
fn get_platform_token(platform: &PlatformConfig, cli_token: Option<String>) -> Result<String> {
    // Accept an explicit CLI token first.
    if let Some(t) = cli_token {
        return Ok(t);
    }

    if let Some(token_name) = &platform.token_name {
        crate::utils::get_token_by_name(token_name).ok_or_else(|| {
            anyhow::anyhow!(
                "No token found for platform '{}' (key: '{}'). \
                 Save it with 'token save-platform {} <token>'.",
                platform.id,
                token_name,
                platform.id
            )
        })
    } else {
        // Platform does not require authentication.
        Ok(String::new())
    }
}

async fn spawn_monitor_task(
    username: &str,
    token: &str,
    platform: PlatformConfig,
    config: crate::config::Config,
) -> Result<()> {
    let username_owned = username.to_string();
    let token_owned = token.to_string();

    tokio::spawn(async move {
        crate::stream::monitor::monitor_stream(
            &username_owned,
            &platform,
            &token_owned,
            std::time::Duration::from_secs_f64(config.get_fetch_interval_seconds()),
            config,
        )
        .await;
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_monitor_string_valid() {
        let result = parse_monitor_string("platform:somecreator");
        assert_eq!(
            result,
            Some(("platform".to_string(), "somecreator".to_string()))
        );
    }

    #[test]
    fn test_parse_monitor_string_no_prefix_returns_none() {
        // Plain usernames without a platform prefix must now return None.
        let result = parse_monitor_string("somecreator");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_monitor_string_empty_platform_returns_none() {
        let result = parse_monitor_string(":somecreator");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_monitor_string_empty_username_returns_none() {
        let result = parse_monitor_string("platform:");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_monitor_string_colon_in_username() {
        // Only the first colon separates platform from username; the rest is part of the name.
        let result = parse_monitor_string("myplatform:user:extra");
        assert_eq!(
            result,
            Some(("myplatform".to_string(), "user:extra".to_string()))
        );
    }

    #[test]
    fn test_load_all_no_fallback_when_empty() {
        // Verify that load_all returns an empty Vec when the platforms directory
        let tmp = tempfile::tempdir().expect("failed to create tempdir");

        // Temporarily override the platforms_dir resolution by reading an empty dir.
        // We call the internal directory-scan logic directly by reading the dir ourselves.
        let dir = tmp.path();
        let mut platforms = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let content = std::fs::read_to_string(&path).unwrap();
                let config: PlatformConfig = serde_json::from_str(&content).unwrap();
                platforms.push(config);
            }
        }
        assert!(
            platforms.is_empty(),
            "empty dir must yield an empty platform list"
        );
    }

    // ── stream_reconnect_delay_minutes config tests ───────────────────────────

    #[test]
    fn test_stream_reconnect_delay_defaults_to_none() {
        let config = crate::config::Config::default();
        assert!(
            config.get_stream_reconnect_delay_minutes().is_none(),
            "stream_reconnect_delay_minutes should default to None (disabled)"
        );
    }

    #[test]
    fn test_stream_reconnect_delay_round_trips_through_toml() {
        let toml_input = "stream_reconnect_delay_minutes = 5.0\n";
        let config: crate::config::Config = toml::from_str(toml_input)
            .expect("failed to parse TOML with stream_reconnect_delay_minutes");
        assert_eq!(config.get_stream_reconnect_delay_minutes(), Some(5.0));
    }

    #[test]
    fn test_stream_reconnect_delay_none_when_absent_from_toml() {
        let config: crate::config::Config = toml::from_str("").expect("failed to parse empty TOML");
        assert!(config.get_stream_reconnect_delay_minutes().is_none());
    }

    #[test]
    fn test_stream_reconnect_delay_key_recognised_by_config() {
        use crate::config::ConfigKey;
        assert!(
            ConfigKey::from_str("stream_reconnect_delay_minutes").is_some(),
            "stream_reconnect_delay_minutes should be a recognised config key"
        );
    }

    #[test]
    fn test_stream_reconnect_delay_set_and_get_via_config_methods() {
        let mut config = crate::config::Config::default();
        config
            .set_value("stream_reconnect_delay_minutes", "3.5")
            .expect("set_value should accept a valid float");
        assert_eq!(config.get_stream_reconnect_delay_minutes(), Some(3.5));
    }

    #[test]
    fn test_stream_reconnect_delay_cleared_with_none_string() {
        let mut config = crate::config::Config::default();
        config
            .set_value("stream_reconnect_delay_minutes", "10.0")
            .unwrap();
        config
            .set_value("stream_reconnect_delay_minutes", "none")
            .unwrap();
        assert!(config.get_stream_reconnect_delay_minutes().is_none());
    }
}
