use crate::consts::*;
use crate::platform::PlatformConfig;
use crate::utils;
use anyhow::Result;
use clap::Subcommand;
use keyring_core::Entry;

#[derive(Subcommand)]
pub enum TokenAction {
    #[command(about = "Save the Bunkr token securely")]
    SaveBunkr { token: String },
    #[command(about = "Save the GoFile token securely")]
    SaveGofile { token: String },
    #[command(about = "Save the Filester token securely")]
    SaveFilester { token: String },
    #[command(
        about = "Save the Pixeldrain API key securely (https://pixeldrain.com/user/api_keys)"
    )]
    SavePixeldrain { token: String },
    #[command(about = "Save the GoonBox account credentials securely")]
    SaveGoonbox { username: String, password: String },
    #[command(
        about = "Save a token for a specific platform (uses the platform's configured token_name)"
    )]
    SavePlatform { platform_id: String, token: String },
    #[command(about = "Remove the Bunkr token from keyring")]
    RemoveBunkr,
    #[command(about = "Remove the GoFile token from keyring")]
    RemoveGofile,
    #[command(about = "Remove the Filester token from keyring")]
    RemoveFilester,
    #[command(about = "Remove the Pixeldrain API key from keyring")]
    RemovePixeldrain,
    #[command(about = "Remove the GoonBox credentials from keyring")]
    RemoveGoonbox,
    #[command(about = "Remove the token for a specific platform")]
    RemovePlatform { platform_id: String },
}

pub fn handle_token_command(action: TokenAction) -> Result<()> {
    match action {
        TokenAction::SaveBunkr { token } => save_token(BUNKR_TOKEN_KEY, &token, "Bunkr token"),
        TokenAction::SaveGofile { token } => save_token(GOFILE_TOKEN_KEY, &token, "GoFile token"),
        TokenAction::SaveFilester { token } => {
            save_token(FILESTER_TOKEN_KEY, &token, "Filester token")
        }
        TokenAction::SavePixeldrain { token } => {
            save_token(PIXELDRAIN_TOKEN_KEY, &token, "Pixeldrain API key")
        }
        TokenAction::SaveGoonbox { username, password } => {
            save_token(GOONBOX_USERNAME_KEY, &username, "GoonBox username")?;
            save_token(GOONBOX_PASSWORD_KEY, &password, "GoonBox password")
        }
        TokenAction::SavePlatform { platform_id, token } => {
            let (token_name, platform_name) = get_token_info_for_platform(&platform_id)?;
            save_token(&token_name, &token, &format!("{} token", platform_name))
        }
        TokenAction::RemoveBunkr => remove_token(BUNKR_TOKEN_KEY, "Bunkr token"),
        TokenAction::RemoveGofile => remove_token(GOFILE_TOKEN_KEY, "GoFile token"),
        TokenAction::RemoveFilester => remove_token(FILESTER_TOKEN_KEY, "Filester token"),
        TokenAction::RemovePixeldrain => remove_token(PIXELDRAIN_TOKEN_KEY, "Pixeldrain API key"),
        TokenAction::RemoveGoonbox => {
            remove_token(GOONBOX_USERNAME_KEY, "GoonBox username")?;
            remove_token(GOONBOX_PASSWORD_KEY, "GoonBox password")
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
