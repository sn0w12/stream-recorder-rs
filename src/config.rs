use crate::print::table::{Cell, Table};
use crate::utils::app_config_dir;
use anyhow::Result;
use colored::Color::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ============================================================================
// MACRO-BASED CONFIG - Define everything in ONE place!
// To add a new config field, just add ONE line below in define_config!
// ============================================================================

macro_rules! define_config {
    (
        $(
            $field:ident: $ty:ty = $toml_default:expr => $runtime_default:expr, $kind:ident, $desc:expr
        ),* $(,)?
    ) => {
        // Generate Config struct
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct Config {
            $(pub $field: $ty,)*
        }

        impl Default for Config {
            fn default() -> Self {
                Config {
                    $($field: $toml_default,)*
                }
            }
        }

        // Generate ConfigKey enum
        #[derive(Clone, Copy, Debug)]
        #[allow(non_camel_case_types)]
        pub enum ConfigKey {
            $($field,)*
        }

        impl ConfigKey {
            pub fn as_str(&self) -> &str {
                match self {
                    $(ConfigKey::$field => stringify!($field),)*
                }
            }

            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $(stringify!($field) => Some(ConfigKey::$field),)*
                    _ => None,
                }
            }

            pub const fn all() -> &'static [Self] {
                &[$(ConfigKey::$field,)*]
            }

            pub fn is_array(&self) -> bool {
                match self {
                    $(ConfigKey::$field => impl_is_array!($kind),)*
                }
            }
        }

        // Generate typed getters
        impl Config {
            $(
                paste::paste! {
                    pub fn [<get_ $field>](&self) -> impl_getter_type!($kind) {
                        impl_getter!($kind, self.$field, $runtime_default)
                    }
                }
            )*
        }

        // Generate CLI methods
        impl Config {
            pub fn get_value(&self, key: &str) -> String {
                match ConfigKey::from_str(key) {
                    $(Some(ConfigKey::$field) => impl_cli_get!($kind, self.$field),)*
                    None => "unknown key".to_string(),
                }
            }

            pub fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
                match ConfigKey::from_str(key) {
                    $(Some(ConfigKey::$field) => {
                        self.$field = impl_cli_set!($kind, value)?;
                    })*
                    None => return Err(anyhow::anyhow!("Unknown key: {}", key)),
                }
                Ok(())
            }

            pub fn get_default_string(&self, key: ConfigKey) -> String {
                match key {
                    $(ConfigKey::$field => impl_default_str!($kind, $runtime_default),)*
                }
            }

            pub fn get_description(&self, key: &str) -> String {
                match ConfigKey::from_str(key) {
                    $(Some(ConfigKey::$field) => $desc.to_string(),)*
                    None => "unknown key".to_string(),
                }
            }

            pub fn print_filtered(&self, filter: Option<String>, show_desc: bool) {
                let mut table = Table::new();

                let mut headers = vec![
                    Cell::new("Key", None),
                    Cell::new("Value", None),
                    Cell::new("Default", None),
                ];
                if show_desc {
                    headers.insert(1, Cell::new("Description", None));
                }
                table.set_headers(headers);

                let filter_lc = filter.map(|s| s.to_lowercase());

                for key in ConfigKey::all() {
                    if let Some(ref f) = filter_lc {
                        if key.as_str().to_lowercase() != *f {
                            continue;
                        }
                    }

                    let current = self.get_value(key.as_str());
                    let default = self.get_default_string(*key);

                    let current_color = if current == default {
                        Some(Green)
                    } else {
                        Some(BrightBlack)
                    };

                    let mut row = vec![
                        Cell::new(key.as_str(), None),
                        Cell::new(current, current_color),
                        Cell::new(default, Some(BrightBlack)),
                    ];
                    if show_desc {
                        row.insert(1, Cell::new(self.get_description(key.as_str()), None));
                    }
                    table.add_row(row);
                }

                table.print();
            }
        }
    };
}

// Helper macros for different field types
macro_rules! impl_getter_type {
    (str) => { String };              // Return String with default
    (str_opt) => { Option<&str> };   // Return Option when no default
    (vec) => { Vec<String> };         // Return Vec with default
    (f64) => { f64 };                  // Return f64 with default
    (u32) => { u32 };                  // Return u32 with default
    (f64_opt) => { Option<f64> };     // Return Option when no default
}

macro_rules! impl_getter {
    (str, $field:expr, $default:expr) => {
        $field.clone().unwrap_or_else(|| $default.clone().unwrap())
    };
    (str_opt, $field:expr, $default:expr) => {
        $field.as_deref()
    };
    (vec, $field:expr, $default:expr) => {
        $field.clone().unwrap_or($default)
    };
    (f64, $field:expr, $default:expr) => {
        $field.unwrap_or($default.unwrap())
    };
    (u32, $field:expr, $default:expr) => {
        $field.unwrap_or($default.unwrap())
    };
    (f64_opt, $field:expr, $default:expr) => {
        $field
    };
}

macro_rules! impl_cli_get {
    (str, $field:expr) => {
        $field.clone().unwrap_or_else(|| "none".into())
    };
    (str_opt, $field:expr) => {
        $field.clone().unwrap_or_else(|| "none".into())
    };
    (vec, $field:expr) => {
        $field
            .as_ref()
            .map(|v| v.join(", "))
            .unwrap_or_else(|| "none".into())
    };
    (f64, $field:expr) => {
        $field
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".into())
    };
    (u32, $field:expr) => {
        $field
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".into())
    };
    (f64_opt, $field:expr) => {
        $field
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".into())
    };
}

macro_rules! impl_cli_set {
    (str, $value:expr) => {{
        let result: Option<String> = if $value == "none" {
            None
        } else {
            Some($value.to_string())
        };
        Ok::<Option<String>, anyhow::Error>(result)
    }};
    (str_opt, $value:expr) => {{
        let result: Option<String> = if $value == "none" {
            None
        } else {
            Some($value.to_string())
        };
        Ok::<Option<String>, anyhow::Error>(result)
    }};
    (vec, $value:expr) => {{
        let result: Option<Vec<String>> = if $value == "none" {
            None
        } else {
            Some($value.split(',').map(|s| s.trim().to_string()).collect())
        };
        Ok::<Option<Vec<String>>, anyhow::Error>(result)
    }};
    (f64, $value:expr) => {{
        let result: Option<f64> = if $value == "none" {
            None
        } else {
            Some(
                $value
                    .parse::<f64>()
                    .map_err(|_| anyhow::anyhow!("Invalid number"))?,
            )
        };
        Ok::<Option<f64>, anyhow::Error>(result)
    }};
    (u32, $value:expr) => {{
        let result: Option<u32> = if $value == "none" {
            None
        } else {
            Some(
                $value
                    .parse::<u32>()
                    .map_err(|_| anyhow::anyhow!("Invalid number"))?,
            )
        };
        Ok::<Option<u32>, anyhow::Error>(result)
    }};
    (f64_opt, $value:expr) => {{
        let result: Option<f64> = if $value == "none" {
            None
        } else {
            Some(
                $value
                    .parse::<f64>()
                    .map_err(|_| anyhow::anyhow!("Invalid number"))?,
            )
        };
        Ok::<Option<f64>, anyhow::Error>(result)
    }};
}

macro_rules! impl_default_str {
    (str, $default:expr) => {
        $default.clone().unwrap_or_else(|| "none".to_string())
    };
    (str_opt, $default:expr) => {
        "none".to_string()
    };
    (vec, $default:expr) => {{
        let v: Vec<String> = $default;
        if v.is_empty() {
            "none".to_string()
        } else {
            v.join(", ")
        }
    }};
    (f64, $default:expr) => {
        $default
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string())
    };
    (u32, $default:expr) => {
        $default
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string())
    };
    (f64_opt, $default:expr) => {
        "none".to_string()
    };
}

macro_rules! impl_is_array {
    (vec) => {
        true
    };
    ($other:tt) => {
        false
    };
}

// ============================================================================
// DEFINE ALL CONFIG FIELDS HERE - Single source of truth!
// Just add one line per field. Everything else is auto-generated.
// ============================================================================

define_config! {
    output_directory: Option<String> = None => Some("./recordings".to_string()), str, "Directory to save recordings",
    monitors: Option<Vec<String>> = None => vec![], vec, "List of usernames to monitor",
    discord_webhook_url: Option<String> = None => None, str_opt, "Discord webhook URL for notifications",
    min_free_space_gb: Option<f64> = Some(20.0) => Some(20.0), f64, "Minimum free disk space before cleanup",
    upload_complete_message_template: Option<String> = None => None, str_opt, "Template for upload completion messages",
    max_upload_retries: Option<u32> = Some(3) => Some(3), u32, "Maximum number of upload retries",
    min_stream_duration: Option<f64> = None => None, f64_opt, "Minimum stream duration before recording",
    bitrate: Option<String> = Some("3M".to_string()) => Some("3M".to_string()), str, "Bitrate to record stream at",
    stream_reconnect_delay_minutes: Option<f64> = None => None, f64_opt, "Delay in minutes to wait for stream continuation before post-processing. Streams resumed are merged.",
    disabled_uploaders: Option<Vec<String>> = None => vec![], vec, "List of uploaders to skip uploading to",
    step_delay_seconds: Option<f64> = None => Some(0.5), f64, "Delay in seconds between each step in a platform",
    fetch_interval_seconds: Option<f64> = None => Some(120.0), f64, "The interval in seconds monitors are fetched at",
}

// ============================================================================
// Core Config methods
// ============================================================================

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    pub fn config_path() -> PathBuf {
        app_config_dir().join("config.toml")
    }
}

#[cfg(test)]
mod readme_sync_tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    /// Parse the README's "Available Settings" table and return a map of
    /// setting -> (description, default_str).
    fn parse_readme_table() -> HashMap<String, (String, String)> {
        let readme = std::fs::read_to_string("README.md").expect("README.md must be present");

        // Find the table header. The README contains a table with header
        // `| Setting                            | Description | Default |`.
        let mut in_table = false;
        let mut rows = Vec::new();

        for line in readme.lines() {
            let l = line.trim_end();
            if l.starts_with("| Setting") {
                in_table = true;
                continue;
            }
            if in_table {
                if l.starts_with("| ---") || l.starts_with("|---") {
                    // separator row, skip
                    continue;
                }
                if l.starts_with('|') {
                    // table row
                    rows.push(l.to_string());
                    continue;
                }
                // end of table
                break;
            }
        }

        let mut map = HashMap::new();
        for row in rows {
            // split columns by '|' and trim
            let parts: Vec<&str> = row.split('|').map(|s| s.trim()).collect();
            if parts.len() < 4 {
                continue;
            }
            let key = parts[1].trim().trim_matches('`').to_string();
            let desc = parts[2].trim().to_string();
            let default = parts[3].trim().trim_matches('`').to_string();
            map.insert(key, (desc, default));
        }

        map
    }

    #[test]
    fn readme_and_config_keys_match() {
        let readme_map = parse_readme_table();
        let readme_keys: HashSet<String> = readme_map.keys().cloned().collect();

        let cfg_keys: HashSet<String> = ConfigKey::all()
            .iter()
            .map(|k| k.as_str().to_string())
            .collect();

        assert_eq!(
            readme_keys, cfg_keys,
            "Config keys in README.md do not match code"
        );
    }

    #[test]
    fn readme_descriptions_match_config() {
        let readme_map = parse_readme_table();
        let cfg = Config::default();

        for (key, (readme_desc, _)) in readme_map.iter() {
            let cfg_desc = cfg.get_description(key);
            assert_eq!(cfg_desc, *readme_desc, "Description mismatch for {}", key);
        }
    }

    #[test]
    fn readme_defaults_match_config() {
        let readme_map = parse_readme_table();
        let cfg = Config::default();

        for (key, (_, readme_default)) in readme_map.iter() {
            let ck = ConfigKey::from_str(key).expect("unknown key in README");
            let cfg_default = cfg.get_default_string(ck);
            // Normalize numeric formatting: trim trailing .0 if present in README
            let mut rd = if readme_default.ends_with(".0") {
                readme_default.trim_end_matches(".0").to_string()
            } else {
                readme_default.clone()
            };

            // Normalize "None"/"none" casing from README
            if rd.eq_ignore_ascii_case("none") {
                rd = "none".to_string();
            }

            assert_eq!(cfg_default, rd, "Default mismatch for {}", key);
        }
    }
}
