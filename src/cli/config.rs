use clap::{Subcommand};
use anyhow::Result;
use crate::platform::PlatformConfig;

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Get configuration value(s)
    #[clap(alias = "ls")]
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
    #[clap(alias = "gp")]
    GetPath,
    /// Manage monitored users
    #[clap(alias = "m")]
    Monitors {
        #[command(subcommand)]
        action: MonitorsAction,
    },
}

#[derive(Subcommand)]
pub enum MonitorsAction {
    /// Add a user to monitor (format: platform_id:username)
    Add {
        /// Monitor entry in `platform_id:username` format
        monitor: String,
    },
    /// Remove a user from monitor (format: platform_id:username)
    #[clap(alias = "rm")]
    Remove {
        /// Monitor entry in `platform_id:username` format
        monitor: String,
    },
    /// List monitored users
    #[clap(alias = "ls")]
    List,
}

pub fn handle_config_command(action: ConfigAction) -> Result<()> {
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
    Ok(())
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