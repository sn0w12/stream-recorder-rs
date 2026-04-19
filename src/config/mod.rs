pub mod types;
pub mod validators;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use tiny_table::{Align, Cell, Color, Column, ColumnWidth, Table, Trunc};

use crate::{
    types::{DurationValue, FileSize},
    utils::app_config_dir,
};
use types::ConfigType;

static CONFIG: OnceLock<Config> = OnceLock::new();

macro_rules! define_config {
	(
		$(
			$category:ident: {
				$(
					$field:ident: $config_type:ty = $default:expr, $desc:expr
				),* $(,)?
			} $(,)?
		)*
	) => {
		#[derive(Debug, Deserialize, Serialize, Clone, Default)]
		#[serde(default)]
		pub struct Config {
			$(
				$(
					#[serde(default, skip_serializing_if = "Option::is_none")]
					pub $field: <$config_type as ConfigType>::Stored,
				)*
			)*
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
							ConfigKey::$field => <$config_type as ConfigType>::is_array(),
						)*
					)*
				}
			}
		}

		impl Config {
			$(
				$(
					paste::paste! {
						pub fn [<get_ $field>](&self) -> <$config_type as ConfigType>::Value<'_> {
							<$config_type as ConfigType>::get(&self.$field, &($default))
						}
					}
				)*
			)*
		}

		impl Config {
			/// Validate the stored values for every setting before the application
			/// reads them through the typed getters.
			pub fn validate(&self) -> Result<()> {
				$(
					$(
						validate_field::<$config_type>(stringify!($field), &self.$field)?;
					)*
				)*
				Ok(())
			}

			pub fn get_value(&self, key: &str) -> String {
				match ConfigKey::from_key(key) {
					$(
						$(
							Some(ConfigKey::$field) => <$config_type as ConfigType>::format_cli(&self.$field, &($default)),
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
								set_field_from_cli::<$config_type>(
									stringify!($field),
									&mut self.$field,
									value,
									&($default),
								)?;
							}
						)*
					)*
					None => return Err(anyhow::anyhow!("Unknown key: {}", key)),
				}
				Ok(())
			}

			pub fn reset_key(&mut self, key: &str) -> Result<String> {
				match ConfigKey::from_key(key) {
					$(
						$(
							Some(ConfigKey::$field) => {
								self.$field = <$config_type as ConfigType>::reset_value(&($default));
								return Ok(<$config_type as ConfigType>::format_default(&($default)));
							}
						)*
					)*
					None => Err(anyhow::anyhow!("Unknown key: {}", key)),
				}
			}

			pub fn get_default_string(&self, key: ConfigKey) -> String {
				match key {
					$(
						$(
							ConfigKey::$field => <$config_type as ConfigType>::format_default(&($default)),
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

fn validate_field<T: ConfigType>(key: &str, value: &T::Stored) -> Result<()> {
    T::validate(value).with_context(|| format!("Invalid value for '{}'", key))
}

fn set_field_from_cli<T: ConfigType>(
    key: &str,
    field: &mut T::Stored,
    raw_value: &str,
    default: &T::Default,
) -> Result<()> {
    let parsed =
        T::parse_cli(raw_value, default).with_context(|| format!("Invalid value for '{}'", key))?;
    validate_field::<T>(key, &parsed)?;
    *field = parsed;
    Ok(())
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

define_config! {
    monitoring: {
        monitors: types::StringList = Vec::<String>::new(), "List of usernames to monitor",
        min_stream_duration: types::OptionalDuration = None, "Minimum recorded duration required before post-processing. Accepts values like 5m, 90s, or 1h.",
        stream_reconnect_delay: types::OptionalDuration = None, "How long to wait for a stream continuation before post-processing. Accepts values like 5m, 30s, or 1h.",
        stream_metadata_refresh_interval: types::OptionalDuration<validators::PositiveDuration> = None, "Refresh extracted stream metadata during active recordings. Accepts values like 30s, 5m, or 1h.",
        step_delay: types::Duration = DurationValue::from_millis(500), "Delay between each step in a platform. Accepts values like 500ms, 2s, or 1m.",
        fetch_interval: types::Duration = DurationValue::from_secs(120), "How often monitors are fetched. Accepts values like 30s, 2m, or 1h.",
    }
    video: {
        video_quality: types::U32<validators::VideoQuality> = 26, "Quality target for variable bitrate video encoding (lower is better)",
        video_bitrate: types::OptionalText<validators::FfmpegBitrate> = None, "Constant video bitrate for CBR encoding (e.g. 6M, 5000k). When set, uses CBR mode and overrides video_quality.",
        max_bitrate: types::OptionalText<validators::FfmpegBitrate> = None, "Maximum video bitrate (e.g. 6M, 2500k). When set, adds -maxrate and -bufsize to ffmpeg",
        max_fps: types::OptionalU32<validators::PositiveU32> = None, "Maximum framerate a stream will be recorded at.",
    }
    post_processing: {
        title_clean_regex: types::StringList<validators::RegexList> = Vec::<String>::new(), "Global regular expressions used to clean stream titles for uploader naming",
    }
    uploads: {
        max_upload_retries: types::U32 = 3, "Maximum number of upload retries",
        disabled_uploaders: types::StringList = Vec::<String>::new(), "List of uploaders to skip uploading to",
    }
    thumbnails: {
        thumbnail_size: types::Text<validators::ThumbnailSize> = "320x180".to_string(), "Size of each thumbnail in the grid, in WIDTHxHEIGHT format",
        thumbnail_grid: types::Text<validators::ThumbnailGrid> = "3x3".to_string(), "Grid layout for thumbnails, in COLSxROWS format",
    }
    notifications: {
        discord_webhook_url: types::OptionalText<validators::Url> = None, "Discord webhook URL for notifications",
        upload_complete_message_template: types::OptionalText = None, "Template for upload completion messages",
    }
    storage: {
        output_directory: types::Text = "./recordings".to_string(), "Directory to save recordings",
        min_free_space: types::FileSize = FileSize::from_gb(20), "Minimum free disk space before cleanup (e.g. 20GB, 500MB)",
        retention_max_age: types::OptionalDuration<validators::PositiveDuration> = None, "Delete recordings older than this age. Accepts values like 7d, 48h, or 14d.",
        retention_keep_latest_per_user: types::OptionalU32<validators::PositiveU32> = None, "Keep only this many of the newest recordings per user",
    }
}

impl Config {
    fn render_filtered(&self, filter: Option<&str>, show_desc: bool) -> String {
        let filter_lc = filter.map(|value| value.to_lowercase());
        let is_filtered = filter_lc.as_deref().is_some_and(|value| !value.is_empty());

        let mut has_rows = false;

        let mut headers = vec![
            Column::new("Key"),
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
        CONFIG.set(config).ok();
        Ok(())
    }

    /// Return a reference to the global configuration singleton.
    ///
    /// If [`Config::init`] has not been called yet, the config is loaded lazily.
    /// Panics with a descriptive message if loading fails, since this represents
    /// a critical initialization failure that should never be silently swallowed.
    pub fn get() -> &'static Config {
        CONFIG.get_or_init(|| {
            Self::load().unwrap_or_else(|error| {
                panic!(
                    "Failed to load configuration: {error}\n\
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
            .set_value("retention_max_age", "14d")
            .expect("set_value should accept a positive retention age");
        config
            .set_value("retention_keep_latest_per_user", "3")
            .expect("set_value should accept a positive retention limit");

        assert_eq!(config.get_value("retention_max_age"), "14d");
        assert_eq!(config.get_value("retention_keep_latest_per_user"), "3");
    }

    #[test]
    fn set_value_rejects_zero_retention_limits() {
        let mut config = Config::default();

        let age_err = config
            .set_value("retention_max_age", "0s")
            .expect_err("set_value should reject zero age retention");
        assert!(age_err.to_string().contains("retention_max_age"));

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
