use keyring::Entry;
use std::path::PathBuf;
use std::fs;

pub const SERVICE_NAME: &str = "stream_recorder";

fn env_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("stream_recorder")
        .join(".env")
}

fn load_env_var(key: &str) -> Option<String> {
    let env_file = env_path();
    if !env_file.exists() {
        return None;
    }

    // Read and parse .env file manually to avoid polluting process environment
    let content = fs::read_to_string(&env_file).ok()?;
    for line in content.lines() {
        let line = line.trim();
        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Parse key=value pairs (handle values with = characters)
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            let v = v.trim();
            if k == key {
                // Remove matching surrounding quotes if present
                let v = if (v.starts_with('"') && v.ends_with('"')) ||
                           (v.starts_with('\'') && v.ends_with('\'')) {
                    &v[1..v.len()-1]
                } else {
                    v
                };
                return Some(v.to_string());
            }
        }
    }
    None
}

/// Retrieves a token by its keyring key name.
///
/// Checks the system keyring first, then falls back to a matching
/// `KEY_NAME` (uppercased) environment variable in the `.env` file.
pub fn get_token_by_name(key_name: &str) -> Option<String> {
    if let Ok(entry) = Entry::new(SERVICE_NAME, key_name) {
        if let Ok(password) = entry.get_password() {
            return Some(password);
        }
    }
    // Fall back to an uppercase env var derived from the key name.
    let env_key = key_name.to_uppercase();
    load_env_var(&env_key)
}

pub fn get_bunkr_token() -> Option<String> {
    // Try keyring first
    if let Ok(entry) = Entry::new(SERVICE_NAME, "bunkr_token") {
        if let Ok(password) = entry.get_password() {
            return Some(password);
        }
    }

    // Fall back to .env file
    load_env_var("BUNKR_TOKEN")
}

pub fn get_gofile_token() -> Option<String> {
    // Try keyring first
    if let Ok(entry) = Entry::new(SERVICE_NAME, "gofile_token") {
        if let Ok(password) = entry.get_password() {
            return Some(password);
        }
    }

    // Fall back to .env file
    load_env_var("GOFILE_TOKEN")
}

pub fn get_filester_token() -> Option<String> {
    // Try keyring first
    if let Ok(entry) = Entry::new(SERVICE_NAME, "filester_token") {
        if let Ok(password) = entry.get_password() {
            return Some(password);
        }
    }

    // Fall back to .env file
    load_env_var("FILESTER_TOKEN")
}

pub fn slugify(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}