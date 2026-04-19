use keyring::Entry;
use std::fs;
use std::path::PathBuf;

pub const SERVICE_NAME: &str = "stream_recorder";

/// Returns the base application configuration directory:
/// `<system_config_dir>/stream_recorder`.
pub fn app_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("stream_recorder")
}

fn env_path() -> PathBuf {
    app_config_dir().join(".env")
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
                let v = if (v.starts_with('"') && v.ends_with('"'))
                    || (v.starts_with('\'') && v.ends_with('\''))
                {
                    &v[1..v.len() - 1]
                } else {
                    v
                };
                return Some(v.to_string());
            }
        }
    }
    None
}

fn get_token_from_sources(key_name: &str, env_key: &str) -> Option<String> {
    if let Ok(entry) = Entry::new(SERVICE_NAME, key_name)
        && let Ok(password) = entry.get_password()
    {
        return Some(password);
    }

    load_env_var(env_key)
}

/// Retrieves a token by its keyring key name.
///
/// Checks the system keyring first, then falls back to a matching
/// `KEY_NAME` (uppercased) environment variable in the `.env` file.
pub fn get_token_by_name(key_name: &str) -> Option<String> {
    // Fall back to an uppercase env var derived from the key name.
    let env_key = key_name.to_uppercase();
    get_token_from_sources(key_name, &env_key)
}

pub fn get_bunkr_token() -> Option<String> {
    get_token_from_sources("bunkr_token", "BUNKR_TOKEN")
}

pub fn get_gofile_token() -> Option<String> {
    get_token_from_sources("gofile_token", "GOFILE_TOKEN")
}

pub fn get_filester_token() -> Option<String> {
    get_token_from_sources("filester_token", "FILESTER_TOKEN")
}

pub fn get_jpg6_token() -> Option<String> {
    get_token_from_sources("jpg6_token", "JPG6_TOKEN")
}

#[derive(Debug)]
pub enum SplitMonitorReferenceError {
    InvalidFormat,
    EmptyPlatform,
    EmptyUsername,
}

/// Split a monitor reference string in the format "platform:username" into its components.
///
/// ```
/// use stream_recorder::utils::{split_monitor_reference, SplitMonitorReferenceError};
///
/// let (platform, username) = split_monitor_reference("twitch:some_user").unwrap();
/// assert_eq!(platform, "twitch");
/// assert_eq!(username, "some_user");
///
/// let err = split_monitor_reference("invalidformat").unwrap_err();
/// assert!(matches!(err, SplitMonitorReferenceError::InvalidFormat));
/// ```
pub fn split_monitor_reference(
    reference: &str,
) -> Result<(String, String), SplitMonitorReferenceError> {
    let (platform, username) = reference
        .split_once(':')
        .ok_or(SplitMonitorReferenceError::InvalidFormat)?;

    if platform.is_empty() {
        return Err(SplitMonitorReferenceError::EmptyPlatform);
    }

    if username.is_empty() {
        return Err(SplitMonitorReferenceError::EmptyUsername);
    }

    Ok((platform.to_string(), username.to_string()))
}

/// Convert a string into a slug format suitable for filenames and URLs.
///
/// ```
/// use stream_recorder::utils::slugify;
///
/// let slug = slugify("Hello, World!");
/// assert_eq!(slug, "hello-world");
/// ```
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_monitor_reference_accepts_valid_value() {
        let (platform, username) = split_monitor_reference("platform:user").unwrap();
        assert_eq!(platform, "platform");
        assert_eq!(username, "user");
    }

    #[test]
    fn split_monitor_reference_rejects_missing_separator() {
        let err = split_monitor_reference("user").unwrap_err();
        assert!(matches!(err, SplitMonitorReferenceError::InvalidFormat));
    }

    #[test]
    fn split_monitor_reference_rejects_empty_platform() {
        let err = split_monitor_reference(":user").unwrap_err();
        assert!(matches!(err, SplitMonitorReferenceError::EmptyPlatform));
    }

    #[test]
    fn split_monitor_reference_rejects_empty_username() {
        let err = split_monitor_reference("platform:").unwrap_err();
        assert!(matches!(err, SplitMonitorReferenceError::EmptyUsername));
    }

    #[test]
    fn split_monitor_reference_keeps_additional_colons_in_username() {
        let (platform, username) = split_monitor_reference("myplatform:user:extra").unwrap();
        assert_eq!(platform, "myplatform");
        assert_eq!(username, "user:extra");
    }
}
