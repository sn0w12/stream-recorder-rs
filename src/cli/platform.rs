use crate::platform::PipelineOutcome;
use crate::platform::PlatformConfig;
use crate::print::table::{Cell, Table};
use crate::stream::api::run_pipeline_debug;
use crate::stream::api::{PipelineDebugReport, PipelineDebugStep};
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
    /// Run a single monitor through its platform pipeline and print debug output
    Debug {
        /// Monitor reference in platform_id:username format
        monitor: String,
        /// Override the configured platform token for this debug run
        #[arg(short, long)]
        token: Option<String>,
        /// Print the raw JSON response for each pipeline step
        #[arg(long)]
        show_response: bool,
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
        PlatformAction::Debug {
            monitor,
            token,
            show_response,
        } => handle_debug_command(&monitor, token, show_response).await,
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

async fn handle_debug_command(
    monitor: &str,
    token_override: Option<String>,
    show_response: bool,
) -> Result<()> {
    let (platform_id, username) = parse_monitor_reference(monitor)?;
    let platforms = PlatformConfig::load_all()?;
    let platform = PlatformConfig::find_by_id(&platforms, &platform_id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Platform '{}' is not installed.", platform_id))?;
    let token = resolve_platform_token(&platform, token_override)?;

    println!(
        "Debugging monitor '{}' with platform '{}' ({})",
        monitor, platform.id, platform.name
    );
    println!(
        "Step delay: {:.3}s",
        crate::config::Config::get().get_step_delay_seconds()
    );

    let (outcome, report) = run_pipeline_debug(&username, &platform, &token)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    print_debug_report(&report, show_response)?;

    match outcome {
        PipelineOutcome::Live(vars) => {
            println!("Outcome: LIVE");
            if let Some(playback_url) = vars.get("playback_url") {
                println!("playback_url: {}", playback_url);
            }
        }
        PipelineOutcome::Offline => {
            if let Some(step_number) = report.offline_at_step {
                println!(
                    "Outcome: OFFLINE (live_check failed at step {})",
                    step_number
                );
            } else {
                println!("Outcome: OFFLINE");
            }
        }
    }

    Ok(())
}

fn parse_monitor_reference(monitor: &str) -> Result<(String, String)> {
    if let Some((platform_id, username)) = monitor.split_once(':') {
        if platform_id.is_empty() || username.is_empty() {
            return Err(anyhow::anyhow!(
                "Monitor must use 'platform_id:username' format with both parts non-empty."
            ));
        }
        return Ok((platform_id.to_string(), username.to_string()));
    }

    Err(anyhow::anyhow!(
        "Monitor must use 'platform_id:username' format."
    ))
}

fn resolve_platform_token(
    platform: &PlatformConfig,
    token_override: Option<String>,
) -> Result<String> {
    if let Some(token) = token_override {
        return Ok(token);
    }

    if let Some(token_name) = &platform.token_name {
        return crate::utils::get_token_by_name(token_name).ok_or_else(|| {
            anyhow::anyhow!(
                "No token found for platform '{}' (key: '{}'). Provide --token or save one with 'token save-platform {} <token>'.",
                platform.id,
                token_name,
                platform.id
            )
        });
    }

    Ok(String::new())
}

fn print_debug_report(report: &PipelineDebugReport, show_response: bool) -> Result<()> {
    for step in &report.steps {
        print_debug_step(step, show_response)?;
    }

    println!("Final variables:");
    if report.final_vars.is_empty() {
        println!("  (none)");
    } else {
        let mut final_vars: Vec<_> = report.final_vars.iter().collect();
        final_vars.sort_by(|a, b| a.0.cmp(b.0));
        for (key, value) in final_vars {
            println!("  {} = {}", key, value);
        }
    }

    Ok(())
}

fn print_debug_step(step: &PipelineDebugStep, show_response: bool) -> Result<()> {
    println!("\nStep {}", step.step_number);
    println!("  Endpoint template: {}", step.endpoint_template);
    println!("  Resolved endpoint: {}", step.resolved_endpoint);

    if let Some(live_check) = &step.live_check {
        println!(
            "  Live check: {}",
            serde_json::to_string(&live_check.config)?
        );
        println!("  Live check matched: {}", live_check.matched);
        match &live_check.actual_value {
            Some(value) => println!("  Live check value: {}", value),
            None => println!("  Live check value: <missing>"),
        }
    } else {
        println!("  Live check: none");
    }

    println!("  Extracted variables:");
    if step.extracted_vars.is_empty() {
        println!("    (none)");
    } else {
        let mut extracted: Vec<_> = step.extracted_vars.iter().collect();
        extracted.sort_by(|a, b| a.0.cmp(b.0));
        for (key, value) in extracted {
            println!("    {} = {}", key, value);
        }
    }

    println!("  Variables after step:");
    let mut vars: Vec<_> = step.vars_after_step.iter().collect();
    vars.sort_by(|a, b| a.0.cmp(b.0));
    for (key, value) in vars {
        println!("    {} = {}", key, value);
    }

    if show_response {
        println!("  Response JSON:");
        for line in serde_json::to_string_pretty(&step.response)?.lines() {
            println!("    {}", line);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_monitor_reference_accepts_valid_value() {
        let (platform_id, username) = parse_monitor_reference("platform:user").unwrap();
        assert_eq!(platform_id, "platform");
        assert_eq!(username, "user");
    }

    #[test]
    fn parse_monitor_reference_rejects_missing_separator() {
        let err = parse_monitor_reference("user").unwrap_err().to_string();
        assert!(err.contains("platform_id:username"), "got: {}", err);
    }

    #[test]
    fn parse_monitor_reference_rejects_empty_parts() {
        let err = parse_monitor_reference("platform:")
            .unwrap_err()
            .to_string();
        assert!(err.contains("both parts non-empty"), "got: {}", err);
    }
}
