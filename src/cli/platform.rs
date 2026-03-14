use crate::platform::PlatformConfig;
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum PlatformAction {
    /// List all installed platforms
    #[clap(alias = "ls")]
    List,
    /// Install a platform from a remote JSON URL
    Install {
        /// URL to the platform JSON file
        url: String,
    },
    /// Update an installed platform from its saved source URL
    Update {
        /// Platform ID to update (omit when using --all)
        platform_id: Option<String>,
        /// Update all installed platforms that have a saved source URL
        #[arg(long, conflicts_with = "platform_id")]
        all: bool,
    },
    /// Remove an installed platform
    #[clap(alias = "rm")]
    Remove {
        /// Platform ID to remove
        platform_id: String,
    },
}

pub async fn handle_platform_command(action: PlatformAction) -> Result<()> {
    match action {
        PlatformAction::List => {
            let platforms = PlatformConfig::load_all()?;
            if platforms.is_empty() {
                println!("No platforms installed.");
                println!("Install one with: platform install <url>");
            } else {
                println!("Installed platforms:");
                for p in &platforms {
                    let token_status = if let Some(token_name) = &p.token_name {
                        if crate::utils::get_token_by_name(token_name).is_some() {
                            "token configured"
                        } else {
                            "no token"
                        }
                    } else {
                        "no token required"
                    };
                    let update_status = if p.source_url.is_some() {
                        "updatable"
                    } else {
                        "no source URL"
                    };
                    println!(
                        "  {} ({}) v{} — {} steps, {}, {}",
                        p.id,
                        p.name,
                        p.version,
                        p.steps.len(),
                        token_status,
                        update_status
                    );
                }
            }
            Ok(())
        }
        PlatformAction::Install { url } => {
            println!("Downloading platform config from {}...", url);
            let config = PlatformConfig::install_from_url(&url).await?;
            println!("Installed platform '{}' ({}).", config.id, config.name);
            if let Some(token_name) = &config.token_name {
                println!(
                    "  Save its token with: token save-platform {} <token>  (key: '{}')",
                    config.id, token_name
                );
            }
            Ok(())
        }
        PlatformAction::Update { platform_id, all } => {
            if all {
                let results = PlatformConfig::update_all().await?;
                if results.is_empty() {
                    println!("No updatable platforms found (none have a saved source URL).");
                    println!("Re-install platforms with: platform install <url>");
                } else {
                    for (id, result) in results {
                        match result {
                            Ok(config) => println!("Updated '{}' ({}).", id, config.name),
                            Err(e) => eprintln!("Failed to update '{}': {}", id, e),
                        }
                    }
                }
            } else if let Some(id) = platform_id {
                println!("Updating platform '{}'...", id);
                let config = PlatformConfig::update_by_id(&id).await?;
                println!("Updated '{}' ({}).", config.id, config.name);
            } else {
                return Err(anyhow::anyhow!(
                    "Specify a platform ID to update, or pass --all to update all platforms."
                ));
            }
            Ok(())
        }
        PlatformAction::Remove { platform_id } => {
            PlatformConfig::remove_by_id(&platform_id)?;
            println!("Removed platform '{}'.", platform_id);
            Ok(())
        }
    }
}
