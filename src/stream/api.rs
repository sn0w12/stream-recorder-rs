use crate::config::Config;
use crate::platform::{LiveCheck, PipelineOutcome, PlatformConfig, extract_json_value};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct PipelineDebugLiveCheck {
    pub config: Value,
    pub actual_value: Option<Value>,
    pub matched: bool,
}

#[derive(Debug, Clone)]
pub struct PipelineDebugStep {
    pub step_number: usize,
    pub endpoint_template: String,
    pub resolved_endpoint: String,
    pub response: Value,
    pub live_check: Option<PipelineDebugLiveCheck>,
    pub extracted_vars: HashMap<String, String>,
    pub vars_after_step: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PipelineDebugReport {
    pub steps: Vec<PipelineDebugStep>,
    pub final_vars: HashMap<String, String>,
    pub offline_at_step: Option<usize>,
}

fn substitute_variables(template: &str, vars: &HashMap<String, String>) -> String {
    let mut resolved = template.to_string();
    for (var_key, var_val) in vars {
        resolved = resolved.replace(&format!("{{{}}}", var_key), var_val);
    }
    resolved
}

/// Builds a `HeaderMap` from the platform's header configuration, performing
/// placeholder substitution from the provided `vars` map (e.g. `token`).
fn build_headers(
    platform: &PlatformConfig,
    vars: &HashMap<String, String>,
) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    for (key, value) in &platform.headers {
        let value = substitute_variables(value, vars);
        if let (Ok(header_name), Ok(header_value)) = (
            reqwest::header::HeaderName::from_bytes(key.as_bytes()),
            reqwest::header::HeaderValue::from_str(&value),
        ) {
            headers.insert(header_name, header_value);
        }
    }
    headers
}

/// Generic HTTP GET with retry logic, driven by the provided `PlatformConfig`.
///
/// If `url` starts with `http://` or `https://` it is used as-is; otherwise
/// it is appended to `platform.base_url`. Respects `Retry-After` headers on
/// 429 responses and applies exponential back-off on other errors.
pub async fn fetch_with_platform(
    url: &str,
    platform: &PlatformConfig,
    token: &str,
    max_retries: usize,
    initial_delay: f64,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert("token".to_string(), token.to_string());

    let headers = build_headers(platform, &vars);
    let client = Client::builder().default_headers(headers).build()?;

    let full_url = if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("{}{}", platform.base_url, url)
    };

    let mut retry_count = 0;
    let mut delay = initial_delay;
    loop {
        let response_result = client.get(&full_url).send().await;
        let response = match response_result {
            Ok(r) => r,
            Err(e) => {
                if retry_count >= max_retries {
                    return Err(e.into());
                }
                retry_count += 1;
                sleep(Duration::from_secs_f64(delay)).await;
                delay *= 2.0;
                continue;
            }
        };
        let status = response.status();
        if status == 429 {
            if retry_count >= max_retries {
                return Err("429 status after max retries".into());
            }
            retry_count += 1;
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(delay);
            sleep(Duration::from_secs_f64(retry_after)).await;
            delay *= 2.0;
            continue;
        }

        if !status.is_success() {
            if retry_count >= max_retries {
                return Err(format!("HTTP error: {}", status).into());
            }
            retry_count += 1;
            sleep(Duration::from_secs_f64(delay)).await;
            delay *= 2.0;
            continue;
        }

        let json: Value = response.json().await?;
        return Ok(json);
    }
}

/// Executes the platform's fetch pipeline for a given username.
///
/// Steps are run in sequence. The variable map starts with `username` and
/// grows with each step's `extract` entries. `{variable}` placeholders in
/// endpoint templates are substituted before each request is made.
///
/// If a step's `live_check` condition fails against the response JSON the
/// function returns [`PipelineOutcome::Offline`] immediately. If all steps
/// complete the function returns [`PipelineOutcome::Live`] with the full
/// variable map.
pub async fn run_pipeline(
    username: &str,
    platform: &PlatformConfig,
    token: &str,
) -> Result<PipelineOutcome, Box<dyn std::error::Error + Send + Sync>> {
    let (outcome, _) = run_pipeline_internal(username, platform, token, false).await?;
    Ok(outcome)
}

pub async fn run_pipeline_debug(
    username: &str,
    platform: &PlatformConfig,
    token: &str,
) -> Result<(PipelineOutcome, PipelineDebugReport), Box<dyn std::error::Error + Send + Sync>> {
    let (outcome, report) = run_pipeline_internal(username, platform, token, true).await?;
    Ok((
        outcome,
        report.expect("debug report is always collected in debug mode"),
    ))
}

async fn run_pipeline_internal(
    username: &str,
    platform: &PlatformConfig,
    token: &str,
    collect_debug: bool,
) -> Result<(PipelineOutcome, Option<PipelineDebugReport>), Box<dyn std::error::Error + Send + Sync>>
{
    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert("username".to_string(), username.to_string());
    let mut debug_steps = Vec::new();

    for (index, step) in platform.steps.iter().enumerate() {
        // Substitute all known variables into the endpoint template.
        let endpoint = substitute_variables(&step.endpoint, &vars);

        let data = fetch_with_platform(&endpoint, platform, token, 5, 1.0).await?;
        let live_check_debug = step
            .live_check
            .as_ref()
            .map(|live_check| PipelineDebugLiveCheck {
                config: serde_json::to_value(live_check).unwrap_or(Value::Null),
                actual_value: live_check_actual_value(live_check, &data),
                matched: live_check.matches(&data),
            });

        // Evaluate the live_check condition if present.
        if let Some(debug) = &live_check_debug
            && !debug.matched
        {
            if collect_debug {
                debug_steps.push(PipelineDebugStep {
                    step_number: index + 1,
                    endpoint_template: step.endpoint.clone(),
                    resolved_endpoint: endpoint,
                    response: data,
                    live_check: live_check_debug,
                    extracted_vars: HashMap::new(),
                    vars_after_step: vars.clone(),
                });
            }
            return Ok((
                PipelineOutcome::Offline,
                collect_debug.then_some(PipelineDebugReport {
                    steps: debug_steps,
                    final_vars: vars,
                    offline_at_step: Some(index + 1),
                }),
            ));
        }

        // Extract variables from the response.
        let mut extracted_vars = HashMap::new();
        for (var_name, json_path) in &step.extract {
            if let Some(value_str) =
                extract_json_value(&data, json_path).and_then(json_value_to_string)
            {
                vars.insert(var_name.clone(), value_str);
                extracted_vars.insert(var_name.clone(), vars[var_name].clone());
            }
        }

        if collect_debug {
            debug_steps.push(PipelineDebugStep {
                step_number: index + 1,
                endpoint_template: step.endpoint.clone(),
                resolved_endpoint: endpoint,
                response: data,
                live_check: live_check_debug,
                extracted_vars,
                vars_after_step: vars.clone(),
            });
        }

        // Delay between steps if configured
        let delay = Config::get().get_step_delay_seconds();
        if delay > 0.0 {
            sleep(Duration::from_secs_f64(delay)).await;
        }
    }

    Ok((
        PipelineOutcome::Live(vars.clone()),
        collect_debug.then_some(PipelineDebugReport {
            steps: debug_steps,
            final_vars: vars,
            offline_at_step: None,
        }),
    ))
}

/// Converts a JSON value to a `String` for use as a pipeline variable.
///
/// Accepts plain strings and all numeric types. Returns `None` for objects,
/// arrays, booleans, and null (which are not useful as template variables).
fn json_value_to_string(v: &Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    if v.is_number() {
        return Some(v.to_string());
    }
    None
}

fn live_check_actual_value(live_check: &LiveCheck, response: &Value) -> Option<Value> {
    match live_check {
        LiveCheck::Path(path) => extract_json_value(response, path).cloned(),
        LiveCheck::Condition(condition) => extract_json_value(response, &condition.path).cloned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PlatformConfig;
    use std::collections::HashMap;

    #[test]
    fn test_build_headers_substitution() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer {token}".to_string());
        headers.insert("X-User".to_string(), "{username}".to_string());

        let platform = PlatformConfig {
            id: "p1".to_string(),
            name: "P1".to_string(),
            icon: None,
            base_url: "https://example.com/".to_string(),
            token_name: None,
            headers,
            steps: Vec::new(),
            source_url: None,
            version: "1.0.0".to_string(),
            stream_recorder_version: None,
            title_clean_regex: None,
        };

        let mut vars = HashMap::new();
        vars.insert("token".to_string(), "tok-123".to_string());
        vars.insert("username".to_string(), "alice".to_string());

        let header_map = build_headers(&platform, &vars);
        assert_eq!(
            header_map.get("Authorization").unwrap().to_str().unwrap(),
            "Bearer tok-123"
        );
        assert_eq!(header_map.get("X-User").unwrap().to_str().unwrap(), "alice");
    }

    #[test]
    fn test_build_headers_unknown_placeholder_left_intact() {
        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_string(), "Value {missing}".to_string());

        let platform = PlatformConfig {
            id: "p2".to_string(),
            name: "P2".to_string(),
            icon: None,
            base_url: "https://example.com/".to_string(),
            token_name: None,
            headers,
            steps: Vec::new(),
            source_url: None,
            version: "1.0.0".to_string(),
            stream_recorder_version: None,
            title_clean_regex: None,
        };

        let vars: HashMap<String, String> = HashMap::new();
        let header_map = build_headers(&platform, &vars);
        assert_eq!(
            header_map.get("X-Custom").unwrap().to_str().unwrap(),
            "Value {missing}"
        );
    }
}
