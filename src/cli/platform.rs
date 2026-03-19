use crate::platform::PlatformConfig;
use crate::print::table::{Cell, Table};
use anyhow::Result;
use clap::Subcommand;
use reqwest::Client;
use serde::Deserialize;

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
    Search {
        /// Optional search query to find platforms by name or description
        query: Option<String>,
        /// Page number of results to show (default: 1)
        #[arg(short, long)]
        page: Option<u32>,
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

#[derive(Deserialize)]
struct RepoSearchResponse {
    items: Vec<RepoItem>,
}

#[derive(Deserialize)]
struct RepoItem {
    full_name: String,
    description: Option<String>,
    stargazers_count: u32,
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
        PlatformAction::Search { query, page } => {
            let client = Client::new();
            let page = page.unwrap_or(1);
            let q = if let Some(ref qs) = query {
                format!(
                    "{} topic:stream-recorder-rs-platform in:name,description",
                    qs
                )
            } else {
                "topic:stream-recorder-rs-platform".to_string()
            };
            let page_str = page.to_string();

            let resp = client
                .get("https://api.github.com/search/repositories")
                .header(reqwest::header::USER_AGENT, "stream-recorder-rs")
                .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
                .query(&[
                    ("q", q.as_str()),
                    ("per_page", "10"),
                    ("page", page_str.as_str()),
                ])
                .send()
                .await?;

            let status = resp.status();
            if !status.is_success() {
                let txt = resp.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!("GitHub API error: {} - {}", status, txt));
            }

            let body: RepoSearchResponse = resp.json().await?;

            if body.items.is_empty() {
                println!("No results found.");
            } else {
                let mut table = Table::new();
                table.set_headers(vec![
                    Cell::new("No.", None),
                    Cell::new("Name", None),
                    Cell::new("Description", None),
                    Cell::new("Stars", None),
                ]);
                for (i, item) in body.items.iter().enumerate() {
                    table.add_row(vec![
                        Cell::new((i + 1).to_string(), None),
                        Cell::new(item.full_name.clone(), None),
                        Cell::new(item.description.clone().unwrap_or_default(), None),
                        Cell::new(item.stargazers_count.to_string(), None),
                    ]);
                }
                table.print();
            }

            Ok(())
        }
    }
}
