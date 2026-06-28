use crate::consts::*;
use crate::platform::PlatformConfig;
use crate::utils;
use crate::utils::jpg6_cookies_path;
use anyhow::Result;
use clap::Subcommand;
use keyring_core::Entry;

#[derive(Subcommand)]
pub enum TokenAction {
    /// Save the Bunkr token securely
    SaveBunkr { token: String },
    /// Save the GoFile token securely
    SaveGofile { token: String },
    /// Save the Filester token securely
    SaveFilester { token: String },
    /// Save the jpg6 session cookies as a JSON object with goonbox_session and XSRF-TOKEN to the config file
    SaveJpg6 { token: String },
    /// Save a token for a specific platform (uses the platform's configured token_name)
    SavePlatform {
        /// Platform ID as defined in its JSON config
        platform_id: String,
        /// Token value to save
        token: String,
    },
    /// Remove the Bunkr token from keyring
    RemoveBunkr,
    /// Remove the GoFile token from keyring
    RemoveGofile,
    /// Remove the Filester token from keyring
    RemoveFilester,
    /// Remove the JPG6 token from keyring
    RemoveJpg6,
    /// Remove the token for a specific platform
    RemovePlatform {
        /// Platform ID as defined in its JSON config
        platform_id: String,
    },
}

pub fn handle_token_command(action: TokenAction) -> Result<()> {
    match action {
        TokenAction::SaveBunkr { token } => save_token(BUNKR_TOKEN_KEY, &token, "Bunkr token"),
        TokenAction::SaveGofile { token } => save_token(GOFILE_TOKEN_KEY, &token, "GoFile token"),
        TokenAction::SaveFilester { token } => {
            save_token(FILESTER_TOKEN_KEY, &token, "Filester token")
        }
        TokenAction::SaveJpg6 { token } => save_jpg6_cookies(&token),
        TokenAction::SavePlatform { platform_id, token } => {
            let (token_name, platform_name) = get_token_info_for_platform(&platform_id)?;
            save_token(&token_name, &token, &format!("{} token", platform_name))
        }
        TokenAction::RemoveBunkr => remove_token(BUNKR_TOKEN_KEY, "Bunkr token"),
        TokenAction::RemoveGofile => remove_token(GOFILE_TOKEN_KEY, "GoFile token"),
        TokenAction::RemoveFilester => remove_token(FILESTER_TOKEN_KEY, "Filester token"),
        TokenAction::RemoveJpg6 => {
            remove_jpg6_cookies();
            Ok(())
        }
        TokenAction::RemovePlatform { platform_id } => {
            let (token_name, platform_name) = get_token_info_for_platform(&platform_id)?;
            remove_token(&token_name, &format!("{} token", platform_name))
        }
    }
}

fn get_token_info_for_platform(platform_id: &str) -> Result<(String, String)> {
    let platforms = PlatformConfig::load_all()?;
    let platform = PlatformConfig::find_by_id(&platforms, platform_id).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown platform '{}'. Check your platforms directory.",
            platform_id
        )
    })?;
    let token_name = platform
        .token_name
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Platform '{}' does not use a token.", platform_id))?;
    Ok((token_name.to_string(), platform.name.clone()))
}

fn save_token(key: &str, token: &str, display_name: &str) -> Result<()> {
    let entry = Entry::new(utils::SERVICE_NAME, key)?;
    entry.set_password(token)?;
    println!("{} saved securely.", display_name);
    Ok(())
}

fn remove_token(key: &str, display_name: &str) -> Result<()> {
    let entry = Entry::new(utils::SERVICE_NAME, key)?;
    match entry.delete_credential() {
        Ok(_) => println!("{} removed.", display_name),
        Err(e) => eprintln!("Error removing {}: {}", display_name.to_lowercase(), e),
    }
    Ok(())
}

fn save_jpg6_cookies(json: &str) -> Result<()> {
    let parsed: std::collections::HashMap<String, String> =
        serde_json::from_str(json).map_err(|e| {
            anyhow::anyhow!(
                "Invalid cookies JSON. Expected format: {{\"goonbox_session\":\"...\",\"XSRF-TOKEN\":\"...\"}}\nError: {}",
                e
            )
        })?;

    if !parsed.contains_key("goonbox_session") || !parsed.contains_key("XSRF-TOKEN") {
        anyhow::bail!(
            "Cookies JSON must contain both 'goonbox_session' and 'XSRF-TOKEN' keys.\n\
             Expected format: {{\"goonbox_session\":\"value\",\"XSRF-TOKEN\":\"value\"}}"
        );
    }

    let path = jpg6_cookies_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let cookies: Vec<serde_json::Value> = parsed
        .iter()
        .map(|(name, value)| serde_json::json!({"name": name, "value": value}))
        .collect();
    let formatted = serde_json::to_string_pretty(&cookies)?;
    std::fs::write(&path, &formatted)?;
    println!("jpg6 cookies saved to {}", path.display());
    Ok(())
}

fn remove_jpg6_cookies() {
    let path = jpg6_cookies_path();
    if path.exists() {
        match std::fs::remove_file(&path) {
            Ok(_) => println!("jpg6 cookies file removed."),
            Err(e) => eprintln!("Error removing jpg6 cookies file: {}", e),
        }
    } else {
        println!("No jpg6 cookies file found.");
    }
}
