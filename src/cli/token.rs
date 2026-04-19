use crate::platform::PlatformConfig;
use crate::utils;
use anyhow::Result;
use clap::Subcommand;
use keyring::Entry;

#[derive(Subcommand)]
pub enum TokenAction {
    /// Save the Bunkr token securely
    SaveBunkr { token: String },
    /// Save the GoFile token securely
    SaveGofile { token: String },
    /// Save the Filester token securely
    SaveFilester { token: String },
    /// Save the JPG6 token securely as USERNAME;PASSWORD
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
        TokenAction::SaveBunkr { token } => save_token("bunkr_token", &token, "Bunkr token"),
        TokenAction::SaveGofile { token } => save_token("gofile_token", &token, "GoFile token"),
        TokenAction::SaveFilester { token } => {
            save_token("filester_token", &token, "Filester token")
        }
        TokenAction::SaveJpg6 { token } => save_token("jpg6_token", &token, "JPG6 token"),
        TokenAction::SavePlatform { platform_id, token } => {
            let (token_name, platform_name) = get_token_info_for_platform(&platform_id)?;
            save_token(&token_name, &token, &format!("{} token", platform_name))
        }
        TokenAction::RemoveBunkr => remove_token("bunkr_token", "Bunkr token"),
        TokenAction::RemoveGofile => remove_token("gofile_token", "GoFile token"),
        TokenAction::RemoveFilester => remove_token("filester_token", "Filester token"),
        TokenAction::RemoveJpg6 => remove_token("jpg6_token", "JPG6 token"),
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
