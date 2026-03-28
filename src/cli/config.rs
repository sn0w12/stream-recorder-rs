use crate::config::{Config, ConfigKey};
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Get configuration value(s)
    #[clap(alias = "ls")]
    Get {
        /// Specific key to get, if omitted get all
        key: Option<String>,
    },
    /// Set configuration value
    Set { key: String, value: String },
    /// Add a value to an array setting
    #[clap(alias = "a")]
    Add { key: String, value: String },
    /// Remove a value from an array setting
    #[clap(alias = "rm")]
    Remove { key: String, value: String },
    /// Reset a setting to its default value
    #[clap(alias = "r")]
    Reset { key: String },
    /// Get the path to the config file
    #[clap(alias = "gp")]
    GetPath,
    /// Output config in markdown format. Used in docs generation
    #[clap(alias = "md")]
    MarkDown,
}

pub fn handle_config_command(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Get { key } => {
            let config = Config::load()?;
            if let Some(k) = key {
                config.print_filtered(Some(k), true);
            } else {
                config.print_filtered(None, false);
            }
        }
        ConfigAction::Set { key, value } => {
            let mut config = Config::load()?;
            config.set_value(&key, &value)?;
            config.save()?;
            println!("Config updated.");
        }
        ConfigAction::Add { key, value } => {
            let mut config = Config::load()?;
            let config_key =
                ConfigKey::from_str(&key).ok_or_else(|| anyhow::anyhow!("Unknown key: {}", key))?;
            if !config_key.is_array() {
                return Err(anyhow::anyhow!("Key '{}' is not an array setting", key));
            }
            let current = config.get_value(&key);
            let mut vec: Vec<String> = if current == "none" {
                vec![]
            } else {
                current.split(", ").map(|s| s.trim().to_string()).collect()
            };
            if !vec.contains(&value) {
                vec.push(value.clone());
                let new_value = vec.join(", ");
                config.set_value(&key, &new_value)?;
                config.save()?;
                println!("Added '{}' to '{}'", value, key);
            } else {
                println!("'{}' is already in '{}'", value, key);
            }
        }
        ConfigAction::Remove { key, value } => {
            let mut config = Config::load()?;
            let config_key =
                ConfigKey::from_str(&key).ok_or_else(|| anyhow::anyhow!("Unknown key: {}", key))?;
            if !config_key.is_array() {
                return Err(anyhow::anyhow!("Key '{}' is not an array setting", key));
            }
            let current = config.get_value(&key);
            if current == "none" {
                println!("'{}' is empty", key);
                return Ok(());
            }
            let mut vec: Vec<String> = current.split(", ").map(|s| s.trim().to_string()).collect();
            if let Some(pos) = vec.iter().position(|v| v == &value) {
                vec.remove(pos);
                let new_value = if vec.is_empty() {
                    "none".to_string()
                } else {
                    vec.join(", ")
                };
                config.set_value(&key, &new_value)?;
                config.save()?;
                println!("Removed '{}' from '{}'", value, key);
            } else {
                println!("'{}' not found in '{}'", value, key);
            }
        }
        ConfigAction::Reset { key } => {
            let mut config = Config::load()?;
            let config_key =
                ConfigKey::from_str(&key).ok_or_else(|| anyhow::anyhow!("Unknown key: {}", key))?;
            let default = config.get_default_string(config_key);
            config.set_value(&key, &default)?;
            config.save()?;
            println!("Reset '{}' to default: {}", key, default);
        }
        ConfigAction::GetPath => {
            let path = Config::config_path();
            println!("{}", path.display());
        }
        ConfigAction::MarkDown => {
            print!("{}", Config::markdown_table());
        }
    }
    Ok(())
}
