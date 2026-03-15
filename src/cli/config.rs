use crate::config::ConfigKey;
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
}

pub fn handle_config_command(action: ConfigAction) -> Result<()> {
    let mut config = crate::config::Config::load()?;
    match action {
        ConfigAction::Get { key } => {
            if let Some(k) = key {
                let reset = "\x1b[0m";
                let green = "\x1b[32m";
                let gray = "\x1b[90m";

                let config_key =
                    ConfigKey::from_str(&k).ok_or_else(|| anyhow::anyhow!("Unknown key: {}", k))?;

                let value = config.get_value(&k);
                let default = config.get_default_string(config_key);
                let description = config.get_description(&k);

                println!("{} | {}{}{}", k, gray, description, reset);
                println!("{}{}{} | {}{}{}", green, value, reset, gray, default, reset);
            } else {
                config.print_all();
            }
        }
        ConfigAction::Set { key, value } => {
            config.set_value(&key, &value)?;
            config.save()?;
            println!("Config updated.");
        }
        ConfigAction::Add { key, value } => {
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
            let config_key =
                ConfigKey::from_str(&key).ok_or_else(|| anyhow::anyhow!("Unknown key: {}", key))?;
            let default = config.get_default_string(config_key);
            config.set_value(&key, &default)?;
            config.save()?;
            println!("Reset '{}' to default: {}", key, default);
        }
        ConfigAction::GetPath => {
            let path = crate::config::Config::config_path();
            println!("{}", path.display());
        }
    }
    Ok(())
}
