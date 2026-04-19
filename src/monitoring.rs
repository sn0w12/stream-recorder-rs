use crate::config::Config;
use crate::platform::PlatformConfig;
use crate::stream::monitor::monitor_stream;
use crate::utils::split_monitor_reference;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::task::JoinHandle;

pub struct MonitorSupervisor {
    platforms: Vec<PlatformConfig>,
    cli_token: Option<String>,
    active_monitors: HashMap<String, JoinHandle<()>>,
    last_monitors: Vec<String>,
}

impl MonitorSupervisor {
    pub fn new(platforms: Vec<PlatformConfig>, cli_token: Option<String>) -> Self {
        Self {
            platforms,
            cli_token,
            active_monitors: HashMap::new(),
            last_monitors: Vec::new(),
        }
    }

    pub async fn run(mut self) -> Result<()> {
        self.reconcile(Config::get().get_monitors()).await?;

        if self.active_monitors.is_empty() {
            println!("No monitors configured. Waiting for config updates...");
        }

        let mut reload_interval = tokio::time::interval(Duration::from_secs(2));
        let shutdown = tokio::signal::ctrl_c();
        tokio::pin!(shutdown);

        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                _ = reload_interval.tick() => {
                    self.reload_and_reconcile().await;
                }
            }
        }

        self.shutdown();
        Ok(())
    }

    async fn reload_and_reconcile(&mut self) {
        match Config::reload() {
            Ok(config) => {
                let monitors = config.get_monitors();
                if monitors != self.last_monitors
                    && let Err(error) = self.reconcile(monitors).await
                {
                    eprintln!("Error updating monitored users: {}", error);
                }
            }
            Err(error) => eprintln!("Error reloading config: {}", error),
        }
    }

    async fn reconcile(&mut self, desired_monitors: Vec<String>) -> Result<()> {
        let desired_set: HashSet<String> = desired_monitors.iter().cloned().collect();

        let active_keys: Vec<String> = self.active_monitors.keys().cloned().collect();
        for monitor in active_keys {
            if !desired_set.contains(&monitor)
                && let Some(handle) = self.active_monitors.remove(&monitor)
            {
                handle.abort();
            }
        }

        for monitor in &desired_monitors {
            if self.active_monitors.contains_key(monitor) {
                continue;
            }

            if let Some(handle) = self.spawn_monitor_task(monitor).await {
                self.active_monitors.insert(monitor.clone(), handle);
            }
        }

        self.last_monitors = desired_monitors;
        Ok(())
    }

    async fn spawn_monitor_task(&self, monitor: &str) -> Option<JoinHandle<()>> {
        let (platform_id, username) = match split_monitor_reference(monitor) {
            Ok(pair) => pair,
            Err(_) => {
                eprintln!("Malformed monitor string '{}', skipping.", monitor);
                return None;
            }
        };

        let platform = match PlatformConfig::find_by_id(&self.platforms, &platform_id) {
            Some(platform) => platform.clone(),
            None => {
                eprintln!(
                    "Unknown platform '{}' for monitor '{}', skipping.",
                    platform_id, monitor
                );
                return None;
            }
        };

        let token = match self.resolve_token(&platform) {
            Ok(token) => token,
            Err(error) => {
                eprintln!(
                    "Error getting token for platform '{}' while starting '{}': {}",
                    platform_id, monitor, error
                );
                return None;
            }
        };

        let username = username.clone();
        let handle = tokio::spawn(async move {
            monitor_stream(&username, &platform, &token).await;
        });

        Some(handle)
    }

    fn resolve_token(&self, platform: &PlatformConfig) -> Result<String> {
        if let Some(token) = self.cli_token.clone() {
            return Ok(token);
        }

        if let Some(token_name) = &platform.token_name {
            crate::utils::get_token_by_name(token_name).ok_or_else(|| {
				anyhow::anyhow!(
					"No token found for platform '{}' (key: '{}'). Save it with 'token save-platform {} <token>'.",
					platform.id,
					token_name,
					platform.id
				)
			})
        } else {
            Ok(String::new())
        }
    }

    fn shutdown(mut self) {
        for (_, handle) in self.active_monitors.drain() {
            handle.abort();
        }
    }
}
