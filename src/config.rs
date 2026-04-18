use crate::stream::postprocess::thumb::parse_thumbnail_string;
use crate::utils::app_config_dir;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use tiny_table::{Align, Cell, Color, Column, ColumnWidth, Table, Trunc};

static CONFIG: OnceLock<Config> = OnceLock::new();

macro_rules! define_config {
    (
        $(
            $category:ident: {
                $(
                    $field:ident: $ty:ty = $toml_default:expr => $runtime_default:expr, $kind:ident, $desc:expr $(, [$validator:expr])?
                ),* $(,)?
            } $(,)?
        )*
    ) => {
        #[derive(Debug, Deserialize, Serialize, Clone)]
        pub struct Config {
            $(
                $(
                    pub $field: $ty,
                )*
            )*
        }

        impl Default for Config {
            fn default() -> Self {
                Config {
                    $(
                        $(
                            $field: $toml_default,
                        )*
                    )*
                }
            }
        }

        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        #[allow(non_camel_case_types)]
        pub enum ConfigCategory {
            $(
                $category,
            )*
        }

        impl ConfigCategory {
            pub fn as_str(&self) -> &str {
                match self {
                    $(
                        ConfigCategory::$category => stringify!($category),
                    )*
                }
            }

            pub const fn all() -> &'static [Self] {
                &[
                    $(
                        ConfigCategory::$category,
                    )*
                ]
            }

            pub fn keys(&self) -> &'static [ConfigKey] {
                match self {
                    $(
                        ConfigCategory::$category => &[
                            $(
                                ConfigKey::$field,
                            )*
                        ],
                    )*
                }
            }

            pub fn display_name(&self) -> String {
                title_case_identifier(self.as_str())
            }
        }

        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        #[allow(non_camel_case_types)]
        pub enum ConfigKey {
            $(
                $(
                    $field,
                )*
            )*
        }

        impl ConfigKey {
            pub fn as_str(&self) -> &str {
                match self {
                    $(
                        $(
                            ConfigKey::$field => stringify!($field),
                        )*
                    )*
                }
            }

            pub fn from_key(s: &str) -> Option<Self> {
                match s {
                    $(
                        $(
                            stringify!($field) => Some(ConfigKey::$field),
                        )*
                    )*
                    _ => None,
                }
            }

            pub const fn all() -> &'static [Self] {
                &[
                    $(
                        $(
                            ConfigKey::$field,
                        )*
                    )*
                ]
            }

            pub fn category(&self) -> ConfigCategory {
                match self {
                    $(
                        $(
                            ConfigKey::$field => ConfigCategory::$category,
                        )*
                    )*
                }
            }

            pub fn is_array(&self) -> bool {
                match self {
                    $(
                        $(
                            ConfigKey::$field => impl_is_array!($kind),
                        )*
                    )*
                }
            }
        }

        impl Config {
            $(
                $(
                    paste::paste! {
                        pub fn [<get_ $field>](&self) -> impl_getter_type!($kind) {
                            impl_getter!($kind, self.$field, $runtime_default)
                        }
                    }
                )*
            )*
        }

        impl Config {
            /// Validate the raw stored values for every setting.
            ///
            /// Validators run against the serialized field type, before any
            /// runtime fallback default is applied by the generated getters.
            /// This is used by `load`, `save`, and `set_value`, and is also
            /// available to callers that deserialize `Config` manually.
            pub fn validate(&self) -> Result<()> {
                $(
                    $(
                        impl_validate!(stringify!($field), &self.$field $(, $validator)? )?;
                    )*
                )*
                Ok(())
            }

            pub fn get_value(&self, key: &str) -> String {
                match ConfigKey::from_key(key) {
                    $(
                        $(
                            Some(ConfigKey::$field) => impl_cli_get!($kind, self.$field, $runtime_default),
                        )*
                    )*
                    None => "unknown key".to_string(),
                }
            }

            pub fn set_value(&mut self, key: &str, value: &str) -> Result<()> {
                match ConfigKey::from_key(key) {
                    $(
                        $(
                            Some(ConfigKey::$field) => {
                                let parsed_value = impl_cli_set!($kind, value)?;
                                impl_validate!(stringify!($field), &parsed_value $(, $validator)? )?;
                                self.$field = parsed_value;
                            }
                        )*
                    )*
                    None => return Err(anyhow::anyhow!("Unknown key: {}", key)),
                }
                Ok(())
            }

            pub fn get_default_string(&self, key: ConfigKey) -> String {
                match key {
                    $(
                        $(
                            ConfigKey::$field => impl_default_str!($kind, $runtime_default),
                        )*
                    )*
                }
            }

            pub fn get_description(&self, key: &str) -> String {
                match ConfigKey::from_key(key) {
                    $(
                        $(
                            Some(ConfigKey::$field) => $desc.to_string(),
                        )*
                    )*
                    None => "unknown key".to_string(),
                }
            }
        }
    };
}

fn title_case_identifier(identifier: &str) -> String {
    identifier
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn markdown_widths(rows: &[[String; 3]]) -> [usize; 3] {
    let mut widths = [
        "Setting".chars().count(),
        "Description".chars().count(),
        "Default".chars().count(),
    ];

    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }

    widths
}

fn write_markdown_table(output: &mut String, rows: &[[String; 3]]) {
    use std::fmt::Write;

    let widths = markdown_widths(rows);

    writeln!(
        output,
        "| {:<setting_width$} | {:<description_width$} | {:<default_width$} |",
        "Setting",
        "Description",
        "Default",
        setting_width = widths[0],
        description_width = widths[1],
        default_width = widths[2],
    )
    .unwrap();
    writeln!(
        output,
        "| {:<setting_width$} | {:<description_width$} | {:<default_width$} |",
        "-".repeat(widths[0]),
        "-".repeat(widths[1]),
        "-".repeat(widths[2]),
        setting_width = widths[0],
        description_width = widths[1],
        default_width = widths[2],
    )
    .unwrap();

    for row in rows {
        writeln!(
            output,
            "| {:<setting_width$} | {:<description_width$} | {:<default_width$} |",
            row[0],
            row[1],
            row[2],
            setting_width = widths[0],
            description_width = widths[1],
            default_width = widths[2],
        )
        .unwrap();
    }
}

// Helper macros for different field types
macro_rules! impl_getter_type {
    (str) => { String };              // Return String with default
    (str_opt) => { Option<&str> };   // Return Option when no default
    (vec) => { Vec<String> };         // Return Vec with default
    (f64) => { f64 };                  // Return f64 with default
    (u32) => { u32 };                  // Return u32 with default
    (u32_opt) => { Option<u32> };     // Return Option<u32> when no default
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
    (u32_opt, $field:expr, $default:expr) => {
        $field
    };
    (f64_opt, $field:expr, $default:expr) => {
        $field
    };
}

macro_rules! impl_cli_get {
    (str, $field:expr, $default:expr) => {
        $field
            .clone()
            .unwrap_or_else(|| $default.clone().unwrap_or_else(|| "none".into()))
    };
    (str_opt, $field:expr, $default:expr) => {
        $field
            .clone()
            .unwrap_or_else(|| $default.clone().unwrap_or_else(|| "none".into()))
    };
    (vec, $field:expr, $default:expr) => {
        $field.as_ref().map(|v| v.join(", ")).unwrap_or_else(|| {
            if $default.is_empty() {
                "none".into()
            } else {
                $default.join(", ")
            }
        })
    };
    (f64, $field:expr, $default:expr) => {
        $field.map(|v| v.to_string()).unwrap_or_else(|| {
            $default
                .map(|v| v.to_string())
                .unwrap_or_else(|| "none".into())
        })
    };
    (u32, $field:expr, $default:expr) => {
        $field.map(|v| v.to_string()).unwrap_or_else(|| {
            $default
                .map(|v| v.to_string())
                .unwrap_or_else(|| "none".into())
        })
    };
    (u32_opt, $field:expr, $default:expr) => {
        $field
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".into())
    };
    (f64_opt, $field:expr, $default:expr) => {
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
    (u32_opt, $value:expr) => {{
        let result: Option<u32> = if $value == "none" {
            None
        } else {
            Some(
                $value
                    .parse()
                    .with_context(|| format!("expected u32, got '{}'", $value))?,
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
    (u32_opt, $default:expr) => {
        "none".to_string()
    };
    (f64_opt, $default:expr) => {
        "none".to_string()
    };
}

macro_rules! impl_validate {
    ($key:expr, $value:expr, $validator:expr) => {
        $validator($value).with_context(|| format!("Invalid value for '{}'", $key))
    };
    ($key:expr, $value:expr) => {
        Ok::<(), anyhow::Error>(())
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

fn validate_video_quality(value: &Option<u32>) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };

    if (1..=51).contains(value) {
        Ok(())
    } else {
        Err(anyhow::anyhow!("video quality must be between 1 and 51"))
    }
}

fn validate_thumbnail_pair(value: &Option<String>, label: &str, format_hint: &str) -> Result<()> {
    let Some(value) = value.as_deref() else {
        return Ok(());
    };

    let (first, second) = parse_thumbnail_string(value)
        .ok_or_else(|| anyhow::anyhow!("{label} must use {format_hint} format"))?;

    if first == 0 || second == 0 {
        return Err(anyhow::anyhow!("{label} values must be greater than zero"));
    }

    Ok(())
}

fn validate_thumbnail_size(value: &Option<String>) -> Result<()> {
    validate_thumbnail_pair(value, "thumbnail size", "WIDTHxHEIGHT")
}

fn validate_thumbnail_grid(value: &Option<String>) -> Result<()> {
    validate_thumbnail_pair(value, "thumbnail grid", "COLSxROWS")
}

fn validate_ffmpeg_bitrate(value: &Option<String>) -> Result<()> {
    let Some(value) = value.as_deref() else {
        return Ok(());
    };

    let bitrate = value.trim();
    if bitrate.is_empty() {
        return Err(anyhow::anyhow!(
            "bitrate cannot be empty; use 'none' to clear the setting"
        ));
    }

    let split_index = bitrate
        .find(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .unwrap_or(bitrate.len());
    let (number_part, suffix) = bitrate.split_at(split_index);

    if number_part.is_empty() {
        return Err(anyhow::anyhow!(
            "bitrate must start with a number, e.g. 2500k or 6M"
        ));
    }

    if number_part.starts_with('.') || number_part.ends_with('.') {
        return Err(anyhow::anyhow!(
            "bitrate number must be a whole number or decimal like 2.5M"
        ));
    }

    if number_part.chars().filter(|&ch| ch == '.').count() > 1 {
        return Err(anyhow::anyhow!(
            "bitrate number must contain at most one decimal point"
        ));
    }

    let numeric_value = number_part
        .parse::<f64>()
        .map_err(|_| anyhow::anyhow!("bitrate must contain a valid positive number"))?;

    if !numeric_value.is_finite() || numeric_value <= 0.0 {
        return Err(anyhow::anyhow!("bitrate must be greater than zero"));
    }

    let suffix = suffix.to_ascii_lowercase();
    if matches!(suffix.as_str(), "" | "k" | "m" | "g" | "ki" | "mi" | "gi") {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "bitrate must use an ffmpeg-style suffix like 2500k, 6M, or 2.5Mi"
        ))
    }
}

fn validate_positive_u32(value: &Option<u32>) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };

    if *value > 0 {
        Ok(())
    } else {
        Err(anyhow::anyhow!("value must be greater than zero"))
    }
}

fn validate_positive_f64(value: &Option<f64>) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };

    if *value > 0.0 {
        Ok(())
    } else {
        Err(anyhow::anyhow!("value must be greater than zero"))
    }
}

fn validate_url(value: &Option<String>) -> Result<()> {
    let Some(value) = value.as_deref() else {
        return Ok(());
    };

    if value.starts_with("https://") || value.starts_with("http://") {
        Ok(())
    } else {
        Err(anyhow::anyhow!("URL must start with http:// or https://"))
    }
}

fn validate_regex(value: &Option<Vec<String>>) -> Result<()> {
    let Some(value) = value.as_deref() else {
        return Ok(());
    };

    for regex_str in value {
        regex::Regex::new(regex_str)
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Invalid regular expression: {}", e))?;
    }

    Ok(())
}

// ============================================================================
// DEFINE ALL CONFIG FIELDS HERE - single source of truth.
//
// Each line expands into:
// - a raw field on `Config`
// - a `ConfigKey` enum entry
// - a typed getter (`get_<field>()`)
// - CLI parsing/printing support
// - README/markdown table rows
// - optional validation hooks
//
// Entry format:
//   category_name: {
//       name: StoredType = toml_default => runtime_default, kind, "description"
//       name: StoredType = toml_default => runtime_default, kind, "description", [validator_fn]
//   }
//
// Meaning of each piece:
// - `category_name`: grouping used for CLI and README headings; rendered in
//   title case (for example `video_settings` becomes `Video Settings`)
// - `StoredType`: exact type serialized in `config.toml`
// - `toml_default`: value used by `Config::default()` when the key is absent
// - `runtime_default`: fallback returned by generated getters when the stored
//   value is `None`
// - `kind`: chooses getter/CLI conversion behavior (`str`, `str_opt`, `vec`,
//   `f64`, `u32`, `f64_opt`)
// - `description`: human-readable text shown by `config get` and README sync
// - `validator_fn`: optional `fn(&StoredType) -> Result<()>` checked on set,
//   save, and load
//
// The two defaults are separate on purpose: some settings should serialize as
// `None` but still behave as if they have a runtime fallback when read.
// ============================================================================

define_config! {
    monitoring: {
        monitors: Option<Vec<String>> = None => Vec::<String>::new(), vec, "List of usernames to monitor",
        min_stream_duration: Option<f64> = None => None, f64_opt, "Minimum stream duration before recording",
        stream_reconnect_delay_minutes: Option<f64> = None => None, f64_opt, "Delay in minutes to wait for stream continuation before post-processing. Streams resumed are merged.",
        stream_metadata_refresh_interval_seconds: Option<f64> = None => None, f64_opt, "Refresh extracted stream metadata during active recordings every N seconds. Updates titles and avatars used by notifications and templates.", [validate_positive_f64],
        step_delay_seconds: Option<f64> = None => Some(0.5), f64, "Delay in seconds between each step in a platform",
        fetch_interval_seconds: Option<f64> = None => Some(120.0), f64, "The interval in seconds monitors are fetched at",
    }
    video: {
        video_quality: Option<u32> = Some(26) => Some(26), u32, "Quality target for variable bitrate video encoding (lower is better)", [validate_video_quality],
        video_bitrate: Option<String> = None => None, str_opt, "Constant video bitrate for CBR encoding (e.g. 6M, 5000k). When set, uses CBR mode and overrides video_quality.", [validate_ffmpeg_bitrate],
        max_bitrate: Option<String> = None => None, str_opt, "Maximum video bitrate (e.g. 6M, 2500k). When set, adds -maxrate and -bufsize to ffmpeg", [validate_ffmpeg_bitrate],
        max_fps: Option<u32> = None => None, u32_opt, "Maximum framerate a stream will be recorded at.", [validate_positive_u32],
    }
    post_processing: {
        title_clean_regex: Option<Vec<String>> = None => Vec::<String>::new(), vec, "Global regular expressions used to clean stream titles for uploader naming", [validate_regex],
    }
    uploads: {
        max_upload_retries: Option<u32> = Some(3) => Some(3), u32, "Maximum number of upload retries",
        disabled_uploaders: Option<Vec<String>> = None => Vec::<String>::new(), vec, "List of uploaders to skip uploading to",
    }
    thumbnails: {
        thumbnail_size: Option<String> = Some("320x180".to_string()) => Some("320x180".to_string()), str, "Size of each thumbnail in the grid, in WIDTHxHEIGHT format", [validate_thumbnail_size],
        thumbnail_grid: Option<String> = Some("3x3".to_string()) => Some("3x3".to_string()), str, "Grid layout for thumbnails, in COLSxROWS format", [validate_thumbnail_grid],
    }
    notifications: {
        discord_webhook_url: Option<String> = None => None, str_opt, "Discord webhook URL for notifications", [validate_url],
        upload_complete_message_template: Option<String> = None => None, str_opt, "Template for upload completion messages",
    }
    storage: {
        output_directory: Option<String> = None => Some("./recordings".to_string()), str, "Directory to save recordings",
        min_free_space_gb: Option<f64> = Some(20.0) => Some(20.0), f64, "Minimum free disk space before cleanup",
        retention_max_age_days: Option<u32> = None => None, u32_opt, "Delete recordings older than this many days", [validate_positive_u32],
        retention_keep_latest_per_user: Option<u32> = None => None, u32_opt, "Keep only this many of the newest recordings per user", [validate_positive_u32],
    }
}

impl Config {
    fn render_filtered(&self, filter: Option<&str>, show_desc: bool) -> String {
        let filter_lc = filter.map(|value| value.to_lowercase());
        let is_filtered = filter_lc.as_deref().is_some_and(|f| !f.is_empty());

        let mut has_rows = false;

        let mut headers = vec![
            Column::new("Key").max_width(0.1),
            Column::new("Value").max_width(ColumnWidth::fill()),
            Column::new("Default").max_width(0.2),
        ];
        if show_desc {
            headers.insert(1, Column::new("Description").max_width(0.3));
        }
        let mut table = Table::with_columns(headers);

        for category in ConfigCategory::all() {
            let keys: Vec<ConfigKey> = ConfigKey::all()
                .iter()
                .copied()
                .filter(|key| key.category() == *category)
                .filter(|key| match filter_lc.as_deref() {
                    Some(value) => key.as_str().eq_ignore_ascii_case(value),
                    None => true,
                })
                .collect();

            if keys.is_empty() {
                continue;
            }

            has_rows = true;
            if !is_filtered {
                table
                    .add_section(category.display_name())
                    .align(Align::Center);
            }

            for key in keys {
                let current = self.get_value(key.as_str());
                let default = self.get_default_string(key);
                let current_color = if current != default {
                    Color::Green
                } else {
                    Color::BrightBlack
                };
                let current_truncation = if key.is_array() {
                    Trunc::NewLine
                } else {
                    Trunc::Middle
                };

                let mut row = vec![
                    Cell::new(key.as_str()),
                    Cell::new(current)
                        .color(current_color)
                        .truncate(current_truncation),
                    Cell::new(default),
                ];
                if show_desc {
                    row.insert(1, Cell::new(self.get_description(key.as_str())));
                }
                table.add_row(row);
            }
        }

        if has_rows {
            table.render()
        } else {
            String::new()
        }
    }

    pub fn print_filtered(&self, filter: Option<String>, show_desc: bool) {
        let rendered = self.render_filtered(filter.as_deref(), show_desc);

        if !rendered.is_empty() {
            println!("{rendered}");
        }
    }

    pub fn markdown_table() -> String {
        use std::fmt::Write;

        let config = Self::default();
        let mut output = String::new();

        for (index, category) in ConfigCategory::all().iter().enumerate() {
            if index > 0 {
                writeln!(output).unwrap();
            }

            writeln!(output, "#### {}", category.display_name()).unwrap();
            writeln!(output).unwrap();

            let rows: Vec<[String; 3]> = category
                .keys()
                .iter()
                .map(|key| {
                    [
                        format!("`{}`", key.as_str()),
                        config.get_description(key.as_str()),
                        format!("`{}`", config.get_default_string(*key)),
                    ]
                })
                .collect();

            write_markdown_table(&mut output, &rows);
        }

        output.trim_end().to_string()
    }

    /// Load the configuration from disk and store it in the global singleton.
    ///
    /// Must be called once at program startup before any call to [`Config::get`].
    /// Subsequent calls are no-ops: once the singleton is set the stored value
    /// is never replaced.
    pub fn init() -> Result<()> {
        let config = Self::load()?;
        CONFIG.set(config).ok(); // Silently ignored if already initialized
        Ok(())
    }

    /// Return a reference to the global configuration singleton.
    ///
    /// If [`Config::init`] has not been called yet, the config is loaded lazily.
    /// Panics with a descriptive message if loading fails, since this represents
    /// a critical initialization failure that should never be silently swallowed.
    pub fn get() -> &'static Config {
        CONFIG.get_or_init(|| {
            Self::load().unwrap_or_else(|e| {
                panic!(
                    "Failed to load configuration: {e}\n\
                     Call Config::init() at startup to handle this error gracefully."
                )
            })
        })
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        let config = if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            toml::from_str(&content)?
        } else {
            Self::default()
        };

        config.validate()?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        self.validate()?;

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

    fn extract_readme_settings_section() -> String {
        let readme = std::fs::read_to_string("README.md").expect("README.md must be present");

        let mut in_section = false;
        let mut lines = Vec::new();

        for line in readme.lines() {
            if line == "### Available Settings" {
                in_section = true;
                continue;
            }

            if in_section {
                if line.starts_with("### ") {
                    break;
                }
                lines.push(line);
            }
        }

        lines.join("\n").trim().to_string()
    }

    fn strip_ansi(input: &str) -> String {
        let mut plain = String::new();
        let mut chars = input.chars();

        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' {
                for next in chars.by_ref() {
                    if next == 'm' {
                        break;
                    }
                }
                continue;
            }

            plain.push(ch);
        }

        plain
    }

    #[test]
    fn category_lists_cover_all_config_keys_in_order() {
        let categorized_keys: Vec<ConfigKey> = ConfigCategory::all()
            .iter()
            .flat_map(|category| category.keys().iter().copied())
            .collect();

        assert_eq!(
            categorized_keys.as_slice(),
            ConfigKey::all(),
            "Config keys must appear in exactly one category"
        );
    }

    #[test]
    fn readme_settings_section_matches_generated_markdown() {
        assert_eq!(extract_readme_settings_section(), Config::markdown_table());
    }

    #[test]
    fn markdown_table_contains_all_config_categories() {
        let markdown = Config::markdown_table();

        for category in ConfigCategory::all() {
            assert!(
                markdown.contains(&format!("#### {}", category.display_name())),
                "Markdown table is missing category {}",
                category.as_str()
            );
        }
    }

    #[test]
    fn markdown_table_contains_all_config_keys() {
        let markdown = Config::markdown_table();

        for key in ConfigKey::all() {
            assert!(
                markdown.contains(&format!("`{}`", key.as_str())),
                "Markdown table is missing key {}",
                key.as_str()
            );
        }
    }

    #[test]
    fn render_filtered_groups_settings_by_category() {
        let rendered = strip_ansi(&Config::default().render_filtered(None, false));
        let mut last_index = 0;

        assert_eq!(
            rendered.matches('┌').count(),
            1,
            "expected a single table top border"
        );
        assert_eq!(
            rendered.matches('└').count(),
            1,
            "expected a single table bottom border"
        );

        for category in ConfigCategory::all() {
            let marker = category.display_name();
            let index = rendered
                .find(&marker)
                .unwrap_or_else(|| panic!("missing category heading: {}", category.as_str()));

            assert!(
                index >= last_index,
                "category {} rendered out of order",
                category.as_str()
            );
            last_index = index;
        }

        for key in ConfigKey::all() {
            assert!(
                rendered.contains(key.as_str()),
                "grouped output is missing key {}",
                key.as_str()
            );
        }
    }

    #[test]
    fn render_filtered_single_key_shows_only_matching_category() {
        let rendered = strip_ansi(&Config::default().render_filtered(Some("video_quality"), true));

        assert_eq!(
            rendered.matches('┌').count(),
            1,
            "expected a single table top border"
        );
        assert!(rendered.contains("video_quality"));
        assert!(!rendered.contains("Monitoring"));
        assert!(!rendered.contains("thumbnail_size"));
    }

    #[test]
    fn set_value_accepts_valid_ffmpeg_bitrate() {
        let mut config = Config::default();

        config
            .set_value("video_bitrate", "2.5M")
            .expect("set_value should accept a valid ffmpeg bitrate");

        assert_eq!(config.get_video_bitrate(), Some("2.5M"));
    }

    #[test]
    fn set_value_rejects_invalid_ffmpeg_bitrate() {
        let mut config = Config::default();

        let err = config
            .set_value("video_bitrate", "fast")
            .expect_err("set_value should reject an invalid ffmpeg bitrate");

        assert!(
            err.to_string()
                .contains("Invalid value for 'video_bitrate'"),
            "unexpected error: {err}"
        );
        assert!(config.get_video_bitrate().is_none());
    }

    #[test]
    fn set_value_rejects_out_of_range_video_quality() {
        let mut config = Config::default();

        let err = config
            .set_value("video_quality", "0")
            .expect_err("set_value should reject out-of-range video quality");

        assert!(
            err.to_string()
                .contains("Invalid value for 'video_quality'"),
            "unexpected error: {err:#}"
        );
        assert!(
            err.chain()
                .any(|cause| cause.to_string().contains("between 1 and 51")),
            "unexpected error chain: {err:#}"
        );
    }

    #[test]
    fn validate_rejects_invalid_thumbnail_size() {
        let config = Config {
            thumbnail_size: Some("320".to_string()),
            ..Config::default()
        };

        let err = config
            .validate()
            .expect_err("validate should reject malformed thumbnail size");

        assert!(
            err.to_string()
                .contains("Invalid value for 'thumbnail_size'"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn set_value_accepts_retention_limits() {
        let mut config = Config::default();

        config
            .set_value("retention_max_age_days", "14")
            .expect("set_value should accept a positive retention age");
        config
            .set_value("retention_keep_latest_per_user", "3")
            .expect("set_value should accept a positive retention limit");

        assert_eq!(config.get_value("retention_max_age_days"), "14");
        assert_eq!(config.get_value("retention_keep_latest_per_user"), "3");
    }

    #[test]
    fn set_value_rejects_zero_retention_limits() {
        let mut config = Config::default();

        let age_err = config
            .set_value("retention_max_age_days", "0")
            .expect_err("set_value should reject zero age retention");
        assert!(age_err.to_string().contains("retention_max_age_days"));

        let keep_err = config
            .set_value("retention_keep_latest_per_user", "0")
            .expect_err("set_value should reject zero per-user retention");
        assert!(
            keep_err
                .to_string()
                .contains("retention_keep_latest_per_user")
        );
    }
}
