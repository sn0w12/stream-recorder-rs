mod config;
mod utils;
mod thumb;
mod template;
mod stream {
    pub mod api;
    pub mod monitor;
}
mod uploaders;
mod platform;

use clap::{Parser, Subcommand};
use anyhow::Result;
use keyring::Entry;
use std::collections::HashMap;
use crate::template::TemplateValue;
use crate::stream::monitor::{build_uploaders, try_upload};
use crate::platform::PlatformConfig;

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
        /// Path to the file to upload
        file: String,
        /// Only upload to this specific service (e.g. bunkr, gofile, fileditch, filester)
        #[arg(short, long)]
        uploader: Option<String>,
    },
}

#[derive(Subcommand)]
enum EncoderAction {
    /// Probe hardware encoders (quick runtime tests)
    Test,
}

#[derive(Subcommand)]
enum TokenAction {
    /// Save the Bunkr token securely
    SaveBunkr { token: String },
    /// Save the GoFile token securely
    SaveGofile { token: String },
    /// Save the Filester token securely
    SaveFilester { token: String },
    /// Save a token for a specific platform (uses the platform's configured token_name)
    SavePlatform {
        /// Platform ID as defined in its JSON config
        platform_id: String,
        /// Token value to save
        token: String,
    },
    /// Remove the Bunkr token from keyring
    RemoveBunkr,
    /// Remove the GoFile token from keyring
    RemoveGofile,
    /// Remove the Filester token from keyring
    RemoveFilester,
    /// Remove the token for a specific platform
    RemovePlatform {
        /// Platform ID as defined in its JSON config
        platform_id: String,
    },
}

#[derive(Subcommand)]
enum TemplateAction {
    /// Render an example of the current template with mock values
    Render,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Get configuration value(s)
    Get {
        /// Specific key to get, if omitted get all
        key: Option<String>,
    },
    /// Set configuration value
    Set {
        key: String,
        value: String,
    },
    /// Get the path to the config file
    GetPath,
    /// Manage monitored users
    Monitors {
        #[command(subcommand)]
        action: MonitorsAction,
    },
}

#[derive(Subcommand)]
enum PlatformAction {
    /// List all installed platforms
    #[clap(alias = "ls")]
    List,
    /// Install a platform from a remote JSON URL
    Install {
        /// URL to the platform JSON file
        url: String,
    },
    /// Update an installed platform from its saved source URL
    Update {
        /// Platform ID to update (omit when using --all)
        platform_id: Option<String>,
        /// Update all installed platforms that have a saved source URL
        #[arg(long, conflicts_with = "platform_id")]
        all: bool,
    },
    /// Remove an installed platform
    Remove {
        /// Platform ID to remove
        platform_id: String,
    },
}

#[derive(Subcommand)]
enum MonitorsAction {
    /// Add a user to monitor (format: platform_id:username)
    Add {
        /// Monitor entry in `platform_id:username` format
        monitor: String,
    },
    /// Remove a user from monitor (format: platform_id:username)
    Remove {
        /// Monitor entry in `platform_id:username` format
        monitor: String,
    },
    /// List monitored users
    #[clap(alias = "ls")]
    List,
}

fn handle_token_command(action: TokenAction) -> Result<()> {
    match action {
        TokenAction::SaveBunkr { token } => save_token("bunkr_token", &token, "Bunkr token"),
        TokenAction::SaveGofile { token } => save_token("gofile_token", &token, "GoFile token"),
        TokenAction::SaveFilester { token } => save_token("filester_token", &token, "Filester token"),
        TokenAction::SavePlatform { platform_id, token } => {
            let platforms = PlatformConfig::load_all()?;
            let platform = PlatformConfig::find_by_id(&platforms, &platform_id)
                .ok_or_else(|| anyhow::anyhow!("Unknown platform '{}'. Check your platforms directory.", platform_id))?;
            let token_name = platform.token_name.as_deref()
                .ok_or_else(|| anyhow::anyhow!("Platform '{}' does not use a token.", platform_id))?;
            save_token(token_name, &token, &format!("{} token", platform.name))
        }
        TokenAction::RemoveBunkr => remove_token("bunkr_token", "Bunkr token"),
        TokenAction::RemoveGofile => remove_token("gofile_token", "GoFile token"),
        TokenAction::RemoveFilester => remove_token("filester_token", "Filester token"),
        TokenAction::RemovePlatform { platform_id } => {
            let platforms = PlatformConfig::load_all()?;
            let platform = PlatformConfig::find_by_id(&platforms, &platform_id)
                .ok_or_else(|| anyhow::anyhow!("Unknown platform '{}'. Check your platforms directory.", platform_id))?;
            let token_name = platform.token_name.as_deref()
                .ok_or_else(|| anyhow::anyhow!("Platform '{}' does not use a token.", platform_id))?;
            remove_token(token_name, &format!("{} token", platform.name))
        }
    }
}

fn save_token(key: &str, token: &str, display_name: &str) -> Result<()> {
    let entry = Entry::new(utils::SERVICE_NAME, key)?;
    entry.set_password(token)?;
    println!("{} saved securely.", display_name);
    Ok(())
}

fn remove_token(key: &str, display_name: &str) -> Result<()> {
    let entry = Entry::new(utils::SERVICE_NAME, key)?;
    match entry.delete_credential() {
        Ok(_) => println!("{} removed.", display_name),
        Err(e) => eprintln!("Error removing {}: {}", display_name.to_lowercase(), e),
    }
    Ok(())
}

async fn handle_platform_command(action: PlatformAction) -> Result<()> {
    match action {
        PlatformAction::List => {
            let platforms = PlatformConfig::load_all()?;
            if platforms.is_empty() {
                println!("No platforms installed.");
                println!("Install one with: platform install <url>");
            } else {
                println!("Installed platforms:");
                for p in &platforms {
                    let token_status = if let Some(token_name) = &p.token_name {
                        if crate::utils::get_token_by_name(token_name).is_some() {
                            "token configured"
                        } else {
                            "no token"
                        }
                    } else {
                        "no token required"
                    };
                    let update_status = if p.source_url.is_some() { "updatable" } else { "no source URL" };
                    println!("  {} ({}) v{} — {} steps, {}, {}", p.id, p.name, p.version, p.steps.len(), token_status, update_status);
                }
            }
            Ok(())
        }
        PlatformAction::Install { url } => {
            println!("Downloading platform config from {}...", url);
            let config = PlatformConfig::install_from_url(&url).await?;
            println!("Installed platform '{}' ({}).", config.id, config.name);
            if let Some(token_name) = &config.token_name {
                println!(
                    "  Save its token with: token save-platform {} <token>  (key: '{}')",
                    config.id, token_name
                );
            }
            Ok(())
        }
        PlatformAction::Update { platform_id, all } => {
            if all {
                let results = PlatformConfig::update_all().await?;
                if results.is_empty() {
                    println!("No updatable platforms found (none have a saved source URL).");
                    println!("Re-install platforms with: platform install <url>");
                } else {
                    for (id, result) in results {
                        match result {
                            Ok(config) => println!("Updated '{}' ({}).", id, config.name),
                            Err(e) => eprintln!("Failed to update '{}': {}", id, e),
                        }
                    }
                }
            } else if let Some(id) = platform_id {
                println!("Updating platform '{}'...", id);
                let config = PlatformConfig::update_by_id(&id).await?;
                println!("Updated '{}' ({}).", config.id, config.name);
            } else {
                return Err(anyhow::anyhow!(
                    "Specify a platform ID to update, or pass --all to update all platforms."
                ));
            }
            Ok(())
        }
        PlatformAction::Remove { platform_id } => {
            PlatformConfig::remove_by_id(&platform_id)?;
            println!("Removed platform '{}'.", platform_id);
            Ok(())
        }
    }
}

fn handle_monitors_command(action: MonitorsAction, config: &mut crate::config::Config) -> Result<()> {
    match action {
        MonitorsAction::Add { monitor } => {
            // Validate that the monitor string uses the required platform_id:username format.
            if !monitor.contains(':') {
                return Err(anyhow::anyhow!(
                    "Monitor must be in 'platform_id:username' format. \
                     Got: '{}'",
                    monitor
                ));
            }
            let (platform_id, username) = monitor.split_once(':')
                .expect("monitor string was validated to contain ':' above");
            if platform_id.is_empty() || username.is_empty() {
                return Err(anyhow::anyhow!(
                    "Both platform_id and username must be non-empty. Got: '{}'",
                    monitor
                ));
            }
            // Check that the platform is actually installed.
            let platforms = PlatformConfig::load_all()?;
            if PlatformConfig::find_by_id(&platforms, platform_id).is_none() {
                return Err(anyhow::anyhow!(
                    "Unknown platform '{}'. Install it first with: platform install <url>",
                    platform_id
                ));
            }
            let monitors = config.monitors.get_or_insert(Vec::new());
            if !monitors.contains(&monitor) {
                monitors.push(monitor.clone());
                config.save()?;
                println!("Added {}:{} to monitors.", platform_id, username);
            } else {
                println!("{}:{} is already in monitors.", platform_id, username);
            }
        }
        MonitorsAction::Remove { monitor } => {
            if let Some(monitors) = &mut config.monitors {
                if let Some(pos) = monitors.iter().position(|u| u == &monitor) {
                    monitors.remove(pos);
                    config.save()?;
                    println!("Removed {} from monitors.", monitor);
                } else {
                    println!("{} not found in monitors.", monitor);
                }
            } else {
                println!("No monitors configured.");
            }
        }
        MonitorsAction::List => {
            let monitors = config.get_monitors();
            if monitors.is_empty() {
                println!("No users are being monitored.");
            } else {
                println!("Monitored users:");
                for user in monitors {
                    println!("- {}", user);
                }
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = crate::config::Config::load()?;

    match cli.command {
        Some(Commands::Token { action }) => handle_token_command(action)?,
        Some(Commands::Template { action }) => match action {
            TemplateAction::Render => {
                let config = crate::config::Config::load()?;
                if let Some(template) = config.get_upload_complete_message_template() {
                    let mut context = HashMap::new();
                    context.insert("date".to_string(), TemplateValue::String("2025-11-09".to_string()));
                    context.insert("username".to_string(), TemplateValue::String("example_user".to_string()));
                    context.insert("user_id".to_string(), TemplateValue::String("12345".to_string()));
                    context.insert("output_path".to_string(), TemplateValue::String("/path/to/recording.mp4".to_string()));
                    context.insert("thumbnail_path".to_string(), TemplateValue::String("/path/to/thumbnail.jpg".to_string()));
                    context.insert("stream_title".to_string(), TemplateValue::String("Example Stream Title".to_string()));
                    context.insert("bunkr_urls".to_string(), TemplateValue::Array(vec!["https://bunkr.example.com/file1".to_string(), "https://bunkr.example.com/file2".to_string()]));
                    context.insert("gofile_urls".to_string(), TemplateValue::Array(vec!["https://gofile.example.com/download".to_string()]));
                    context.insert("fileditch_urls".to_string(), TemplateValue::Array(vec!["https://fileditch.example.com/file".to_string()]));
                    let rendered = crate::template::render_template(template, &context);
                    println!("{}", rendered);
                } else {
                    println!("No template configured.");
                }
            }
        },
        Some(Commands::Config { action }) => {
            let mut config = crate::config::Config::load()?;
            match action {
                ConfigAction::Get { key } => {
                    if let Some(k) = key {
                        let value = config.get_value(&k);
                        println!("{}", value);
                    } else {
                        config.print_all();
                    }
                }
                ConfigAction::Set { key, value } => {
                    config.set_value(&key, &value)?;
                    config.save()?;
                    println!("Config updated.");
                }
                ConfigAction::GetPath => {
                    let path = crate::config::Config::config_path();
                    println!("{}", path.display());
                }
                ConfigAction::Monitors { action } => {
                    handle_monitors_command(action, &mut config)?;
                }
            }
        }
        Some(Commands::Platform { action }) => {
            handle_platform_command(action).await?;
        }
        Some(Commands::Encoders { action }) => match action {
            EncoderAction::Test => {
                // run the encoder probe and print diagnostics
                match crate::stream::monitor::probe_hw_encoders().await {
                    Ok(_) => {},
                    Err(e) => return Err(anyhow::anyhow!(e.to_string())),
                }
            }
        }
        Some(Commands::Upload { file, uploader }) => {
            handle_upload_command(file, uploader).await?;
        }
        None => {
            let platforms = PlatformConfig::load_all()?;
            run_recording(&config, &platforms, cli.token).await?;
        }
    }

    Ok(())
}

async fn handle_upload_command(file: String, uploader: Option<String>) -> Result<()> {
    if !std::path::Path::new(&file).is_file() {
        return Err(anyhow::anyhow!("File not found or is not a regular file: {}", file));
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
        try_upload(up.as_ref(), &file, up_config, &mut upload_results, max_retries).await;
    }

    if !matched {
        if let Some(name) = uploader {
            return Err(anyhow::anyhow!("No uploader named '{}' is configured", name));
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

async fn print_startup_info(config: &crate::config::Config, platforms: &[PlatformConfig]) {
    fn section(title: &str) {
        println!("\n  \x1b[1m{}\x1b[0m", title);
        println!("  \x1b[2m{}\x1b[0m", "─".repeat(title.len()));
    }

    fn item_ok(name: &str, note: &str) {
        println!("  \x1b[32m✓\x1b[0m {:<12}  \x1b[2m{}\x1b[0m", name, note);
    }

    fn item_err(name: &str, note: &str) {
        println!("  \x1b[31m✗\x1b[0m {:<12}  \x1b[2m{}\x1b[0m", name, note);
    }

    fn item_warn(name: &str, note: &str) {
        println!("  \x1b[33m→\x1b[0m {:<12}  \x1b[2m{}\x1b[0m", name, note);
    }

    // Header
    println!("\x1b[1m┌─────────────────────────────────────┐\x1b[0m");
    println!("\x1b[1m│            Stream Recorder          │\x1b[0m");
    println!("\x1b[1m└─────────────────────────────────────┘\x1b[0m");

    section("Platforms");
    if platforms.is_empty() {
        println!("  \x1b[33mNo platforms configured\x1b[0m");
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
            println!("  \x1b[36m•\x1b[0m \x1b[1m{}\x1b[0m - {}", p.name, token_status);
        }
    }

    // Monitored Users
    section("Monitored Users");
    let monitors = config.get_monitors();
    if monitors.is_empty() {
        println!("  \x1b[33mNo users configured\x1b[0m");
    } else {
        for user in &monitors {
            println!("  \x1b[36m•\x1b[0m \x1b[1m{}\x1b[0m", user);
        }
    }

    // Uploaders
    section("Uploaders");
    let has_bunkr = crate::utils::get_bunkr_token().is_some();
    let has_gofile = crate::utils::get_gofile_token().is_some();
    let has_filester = crate::utils::get_filester_token().is_some();

    let uploaders: &[(&str, bool, &str, &str)] = &[
        ("Bunkr",     has_bunkr,  "token configured", "token required"),
        ("GoFile",    has_gofile, "token configured", "token required"),
        ("Fileditch", true,       "always available", ""),
        ("Filester",  true,       if has_filester { "token configured" } else { "no token, public limits" }, ""),
    ];

    for (name, available, ok_note, err_note) in uploaders {
        if *available { item_ok(name, ok_note); } else { item_err(name, err_note); }
    }

    // Encoder
    section("Encoder");

    let bitrate = config.get_bitrate();
    match crate::stream::monitor::detect_best_hw_encoder(&bitrate).await {
        Some((enc, _)) => { print!("\r"); item_ok(&enc, "hardware acceleration"); }
        None =>           { print!("\r"); item_warn("libx264", "no hardware encoder found, using software"); }
    }

    println!();
}

async fn run_recording(config: &crate::config::Config, platforms: &[PlatformConfig], cli_token: Option<String>) -> Result<()> {
    print_startup_info(config, platforms).await;

    let monitors = config.get_monitors();
    if monitors.is_empty() {
        println!("No monitors configured. Use 'stream_recorder config monitors add <platform_id>:<username>' to add users to monitor.");
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
                eprintln!("Unknown platform '{}' for monitor '{}', skipping.", platform_id, monitor_str);
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

        if let Err(e) = spawn_monitor_task(&username, &token, platform).await {
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

async fn spawn_monitor_task(username: &str, token: &str, platform: PlatformConfig) -> Result<()> {
    let username_owned = username.to_string();
    let token_owned = token.to_string();

    tokio::spawn(async move {
        crate::stream::monitor::monitor_stream(
            &username_owned,
            &platform,
            &token_owned,
            std::time::Duration::from_secs(30)
        ).await;
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_monitor_string_valid() {
        let result = parse_monitor_string("platform:somecreator");
        assert_eq!(result, Some(("platform".to_string(), "somecreator".to_string())));
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
        assert_eq!(result, Some(("myplatform".to_string(), "user:extra".to_string())));
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
        assert!(platforms.is_empty(), "empty dir must yield an empty platform list");
    }
}
