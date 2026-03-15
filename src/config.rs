use crate::utils::app_config_dir;
use anyhow::Result;
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
            $field:ident: $ty:ty = $toml_default:expr => $runtime_default:expr, $kind:ident
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

            pub fn print_all(&self) {
                let reset = "\x1b[0m";
                let green = "\x1b[32m";
                let gray = "\x1b[90m";
                let italic = "\x1b[3m";

                println!("Key                                Value     | Default");
                println!("─────────────────────────────────────────────────────");
                for key in ConfigKey::all() {
                    let current = self.get_value(key.as_str());
                    let default = self.get_default_string(*key);

                    let default_display = format!("{}{}{}{}", gray, italic, default, reset);

                    let current_display = if current == default {
                        // If current equals the runtime default, render it dim/gray to indicate it's the default
                        format!("{}{}{}", gray, current, reset)
                    } else {
                        // If current differs from default, highlight it in green
                        format!("{}{}{}", green, current, reset)
                    };

                    println!("{:<34} {:<9} | {}", key.as_str(), current_display, default_display);
                }
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
    output_directory: Option<String> = None => Some("./recordings".to_string()), str,
    monitors: Option<Vec<String>> = None => vec![], vec,
    discord_webhook_url: Option<String> = None => None, str_opt,
    min_free_space_gb: Option<f64> = Some(20.0) => Some(20.0), f64,
    upload_complete_message_template: Option<String> = None => None, str_opt,
    max_upload_retries: Option<u32> = Some(3) => Some(3), u32,
    min_stream_duration: Option<f64> = None => None, f64_opt,
    bitrate: Option<String> = Some("3M".to_string()) => Some("3M".to_string()), str,
    stream_reconnect_delay_minutes: Option<f64> = None => None, f64_opt,
    disabled_uploaders: Option<Vec<String>> = None => vec![], vec,
    step_delay_seconds: Option<f64> = None => Some(0.5), f64,
    fetch_interval_seconds: Option<f64> = None => Some(120.0), f64,
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
