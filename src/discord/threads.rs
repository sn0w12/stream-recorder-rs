use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::utils::app_config_dir;

#[derive(Debug, Default, Serialize, Deserialize)]
struct ThreadStoreData {
    mapping: HashMap<String, u64>, // thread name -> thread ID
}

/// Persistent storage for thread name → ID mappings.
pub struct ThreadStore {
    path: PathBuf,
    data: ThreadStoreData,
}

impl ThreadStore {
    /// Load the store from a file, or create a new empty store if the file doesn't exist.
    pub fn load() -> Self {
        let path = app_config_dir().join("discord_threads.json");
        let data = if path.exists() {
            let contents = fs::read_to_string(&path).unwrap_or_else(|_| {
                eprintln!("Failed to read store file: {}", path.display());
                String::new()
            });
            serde_json::from_str(&contents).unwrap_or_else(|_| {
                eprintln!("Failed to parse store file: {}", path.display());
                ThreadStoreData::default()
            })
        } else {
            ThreadStoreData::default()
        };
        Self { path, data }
    }

    /// Save the current mapping to disk.
    fn save(&self) -> Result<()> {
        let contents =
            serde_json::to_string_pretty(&self.data).context("Failed to serialize thread store")?;
        fs::write(&self.path, contents)
            .with_context(|| format!("Failed to write store file: {}", self.path.display()))?;
        Ok(())
    }

    /// Get the thread ID for a name, if it exists.
    pub fn get(&self, name: &str) -> Option<u64> {
        self.data.mapping.get(name).copied()
    }

    /// Insert a new mapping and save to disk.
    pub fn insert(&mut self, name: String, id: u64) -> Result<()> {
        self.data.mapping.insert(name, id);
        self.save()
    }
}
