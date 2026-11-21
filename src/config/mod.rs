pub mod types;
pub mod validators;

use std::sync::{Arc, OnceLock, RwLock};

use anyhow::Result;
use tiny_table::{Align, Cell, Color, Column, ColumnWidth, Table, TableStyle, Trunc};

use crate::types::{DurationValue, FileSize};
use crate::utils::app_config_dir;

use apto::define_config;

fn strip_ansi_sequences(input: &str) -> String {
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

define_config! {
    monitoring {
        monitors: types::StringList = Vec::<String>::new(),
            "List of platform_id:username to monitor",
        min_stream_duration: types::OptionalDuration = None,
            "Minimum recorded duration required before post-processing. Accepts values like 5m, 90s, or 1h.",
        stream_reconnect_delay: types::OptionalDuration = None,
            "How long to wait for a stream continuation before post-processing. Accepts values like 5m, 30s, or 1h.",
        stream_metadata_refresh_interval: types::OptionalDuration = None,
            "Refresh extracted stream metadata during active recordings. Accepts values like 30s, 5m, or 1h.",
        monitor_spawn_delay: types::Duration = DurationValue::from_secs(10),
            "Delay between spawning each monitor task on startup or config reload. Accepts values like 500ms, 2s, or 5s.",
        step_delay: types::Duration = DurationValue::from_millis(500),
            "Delay between each step in a platform. Accepts values like 500ms, 2s, or 1m.",
        fetch_interval: types::Duration = DurationValue::from_secs(120),
            "How often monitors are fetched. Accepts values like 30s, 2m, or 1h.",
    }
    video {
        video_quality: apto::U32<validators::VideoQuality> = 26,
            "Quality target for variable bitrate video encoding (lower is better)",
        video_bitrate: apto::Text<validators::FfmpegBitrate, apto::Optional> = None,
            "Constant video bitrate for CBR encoding (e.g. 6M, 5000k). When set, uses CBR mode and overrides video_quality.",
        max_bitrate: apto::Text<validators::FfmpegBitrate, apto::Optional> = None,
            "Maximum video bitrate (e.g. 6M, 2500k). When set, adds -maxrate and -bufsize to ffmpeg",
        max_fps: apto::U32<validators::PositiveU32, apto::Optional> = None,
            "Maximum framerate a stream will be recorded at.",
    }
    post_processing {
        title_clean_regex: types::StringList<validators::RegexList> = Vec::<String>::new(),
            "Global regular expressions used to clean stream titles for uploader naming",
    }
    uploads {
        max_upload_retries: apto::U32 = 3,
            "Maximum number of upload retries",
        disabled_uploaders: types::StringList = Vec::<String>::new(),
            "List of uploaders to skip uploading to",
    }
    thumbnails {
        thumbnail_size: apto::Text<validators::ThumbnailSize> = "320x180".to_string(),
            "Size of each thumbnail in the grid, in WIDTHxHEIGHT format",
        thumbnail_grid: apto::Text<validators::ThumbnailGrid> = "3x3".to_string(),
            "Grid layout for thumbnails, in COLSxROWS format",
    }
    notifications {
        discord_webhook_url: apto::Text<validators::Url, apto::Optional> = None,
            "Discord webhook URL for notifications",
        upload_complete_message_template: apto::Text<apto::NoValidation, apto::Optional> = None,
            "Template for upload completion messages",
    }
    storage {
        output_directory: types::Text = "./recordings".to_string(),
            "Directory to save recordings",
        min_free_space: types::FileSize = FileSize::from_gb(20),
            "Minimum free disk space before cleanup (e.g. 20GB, 500MB)",
        retention_max_age: types::OptionalDuration = None,
            "Delete recordings older than this age. Accepts values like 7d, 48h, or 14d.",
        retention_keep_latest_per_user: apto::U32<validators::PositiveU32, apto::Optional> = None,
            "Keep only this many of the newest recordings per user",
    }
    extensions {
        disabled_extensions: types::StringList = Vec::<String>::new(),
            "List of extension IDs to skip",
    }
}

static CONFIG: OnceLock<RwLock<Arc<Config>>> = OnceLock::new();

fn config_store() -> &'static RwLock<Arc<Config>> {
    CONFIG.get_or_init(|| {
        let config =
            Config::load().unwrap_or_else(|error| panic!("Failed to load configuration: {error}"));
        RwLock::new(Arc::new(config))
    })
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
                .filter(|key| key.category() == Some(*category))
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
                let current_truncation = Trunc::Middle;

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

        // Also render root fields
        let root_keys: Vec<ConfigKey> = ConfigKey::all()
            .iter()
            .copied()
            .filter(|key| key.category().is_none())
            .filter(|key| match filter_lc.as_deref() {
                Some(value) => key.as_str().eq_ignore_ascii_case(value),
                None => true,
            })
            .collect();

        if !root_keys.is_empty() {
            has_rows = true;
            if !is_filtered {
                table.add_section("General").align(Align::Center);
            }

            for key in root_keys {
                let current = self.get_value(key.as_str());
                let default = self.get_default_string(key);
                let current_color = if current != default {
                    Color::Green
                } else {
                    Color::BrightBlack
                };

                let mut row = vec![
                    Cell::new(key.as_str()),
                    Cell::new(current)
                        .color(current_color)
                        .truncate(Trunc::Middle),
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

    pub fn markdown_table(&self) -> String {
        let style = TableStyle::from_string("||||-||||||").unwrap_or(TableStyle::unicode());

        let mut tables = vec![];
        for category in ConfigCategory::all() {
            let mut table = Table::with_columns(vec![
                Column::new("Setting"),
                Column::new("Description"),
                Column::new("Default"),
            ])
            .with_style(style);

            let keys = category.keys();
            if keys.is_empty() {
                continue;
            }

            for key in keys {
                let description = self.get_description(key.as_str());
                let default = self.get_default_string(*key);
                table.add_row(vec![
                    Cell::new(format!("`{}`", key.as_str())),
                    Cell::new(description),
                    Cell::new(format!("`{}`", default)),
                ]);
            }

            tables.push((category.display_name(), table));
        }

        // Add root fields as "General" section
        let root_keys: Vec<ConfigKey> = ConfigKey::all()
            .iter()
            .copied()
            .filter(|k| k.category().is_none())
            .collect();
        if !root_keys.is_empty() {
            let mut root_table = Table::with_columns(vec![
                Column::new("Setting"),
                Column::new("Description"),
                Column::new("Default"),
            ])
            .with_style(style);

            for key in &root_keys {
                let description = self.get_description(key.as_str());
                let default = self.get_default_string(*key);
                root_table.add_row(vec![
                    Cell::new(format!("`{}`", key.as_str())),
                    Cell::new(description),
                    Cell::new(format!("`{}`", default)),
                ]);
            }

            tables.insert(0, ("General".to_string(), root_table));
        }

        let mut markdown = String::new();
        for (index, (category_name, table)) in tables.iter().enumerate() {
            markdown.push_str(&format!("#### {}\n\n", category_name));
            let rendered = table.render();
            let stripped = rendered
                .lines()
                .skip(1)
                .take(rendered.lines().count() - 2)
                .collect::<Vec<_>>()
                .join("\n");
            let cleaned = strip_ansi_sequences(&stripped)
                .replace("|-", "| ")
                .replace("-|", " |");
            markdown.push_str(&cleaned);
            if index < tables.len() - 1 {
                markdown.push_str("\n\n");
            }
        }

        markdown
    }

    pub fn init() -> Result<()> {
        Self::reload()?;
        Ok(())
    }

    pub fn get() -> Arc<Config> {
        config_store().read().expect("config lock poisoned").clone()
    }

    pub fn reload() -> Result<Arc<Config>> {
        let config = Arc::new(Self::load()?);
        *config_store().write().expect("config lock poisoned") = Arc::clone(&config);
        Ok(config)
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        let config = if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
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
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    pub fn config_path() -> std::path::PathBuf {
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

    #[test]
    fn category_lists_cover_all_config_keys_in_order() {
        let categorized_keys: Vec<ConfigKey> = ConfigCategory::all()
            .iter()
            .flat_map(|category| category.keys().iter().copied())
            .collect();

        let all_root_keys: Vec<ConfigKey> = ConfigKey::all()
            .iter()
            .copied()
            .filter(|k| k.category().is_none())
            .collect();

        // Root keys first, then categorized keys — matches ConfigKey::all() order
        let mut combined = all_root_keys;
        combined.extend(categorized_keys);

        assert_eq!(
            combined.as_slice(),
            ConfigKey::all(),
            "Config keys must appear in exactly one category or be a root field"
        );
    }

    #[test]
    fn readme_settings_section_matches_generated_markdown() {
        assert_eq!(
            extract_readme_settings_section(),
            Config::default().markdown_table()
        );
    }
}
