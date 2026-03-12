use clap::{Subcommand};
use anyhow::Result;
use keyring::Entry;
use crate::utils;
use crate::platform::PlatformConfig;

#[derive(Subcommand)]
pub enum TokenAction {
    /// Save the Bunkr token securely
    SaveBunkr { token: String },
    /// Save the GoFile token securely
    SaveGofile { token: String },
    /// Save the Filester token securely
    SaveFilester { token: String },
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
        TokenAction::SaveFilester { token } => save_token("filester_token", &token, "Filester token"),
        TokenAction::SavePlatform { platform_id, token } => {
            let platforms = PlatformConfig::load_all()?;
            let platform = PlatformConfig::find_by_id(&platforms, &platform_id)
                .ok_or_else(|| anyhow::anyhow!("Unknown platform '{}'. Check your platforms directory.", platform_id))?;
            let token_name = platform.token_name.as_deref()
                .ok_or_else(|| anyhow::anyhow!("Platform '{}' does not use a token.", platform_id))?;
            save_token(token_name, &token, &format!("{} token", platform.name))
        }
        TokenAction::RemoveBunkr => remove_token("bunkr_token", "Bunkr token"),
        TokenAction::RemoveGofile => remove_token("gofile_token", "GoFile token"),
        TokenAction::RemoveFilester => remove_token("filester_token", "Filester token"),
        TokenAction::RemovePlatform { platform_id } => {
            let platforms = PlatformConfig::load_all()?;
            let platform = PlatformConfig::find_by_id(&platforms, &platform_id)
                .ok_or_else(|| anyhow::anyhow!("Unknown platform '{}'. Check your platforms directory.", platform_id))?;
            let token_name = platform.token_name.as_deref()
                .ok_or_else(|| anyhow::anyhow!("Platform '{}' does not use a token.", platform_id))?;
            remove_token(token_name, &format!("{} token", platform.name))
        }
    }
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