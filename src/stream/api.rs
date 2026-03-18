use crate::platform::{PipelineOutcome, PlatformConfig, extract_json_value};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

/// Builds a `HeaderMap` from the platform's header configuration, performing
/// placeholder substitution from the provided `vars` map (e.g. `token`).
fn build_headers(
    platform: &PlatformConfig,
    vars: &HashMap<String, String>,
) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    for (key, value) in &platform.headers {
        let mut value = value.clone();
        for (var_key, var_val) in vars {
            value = value.replace(&format!("{{{}}}", var_key), var_val);
        }
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
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .default_headers(headers)
        .build()?;

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
/// If a step's `live_check` path is missing or null in the response the
/// function returns [`PipelineOutcome::Offline`] immediately. If all steps
/// complete the function returns [`PipelineOutcome::Live`] with the full
/// variable map.
pub async fn run_pipeline(
    username: &str,
    platform: &PlatformConfig,
    token: &str,
    config: &crate::config::Config,
) -> Result<PipelineOutcome, Box<dyn std::error::Error + Send + Sync>> {
    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert("username".to_string(), username.to_string());

    for step in &platform.steps {
        // Substitute all known variables into the endpoint template.
        let mut endpoint = step.endpoint.clone();
        for (key, value) in &vars {
            endpoint = endpoint.replace(&format!("{{{}}}", key), value);
        }

        let data = fetch_with_platform(&endpoint, platform, token, 5, 1.0).await?;

        // Evaluate the live_check condition if present.
        if let Some(live_path) = &step.live_check
            && extract_json_value(&data, live_path).is_none()
        {
            return Ok(PipelineOutcome::Offline);
        }

        // Extract variables from the response.
        for (var_name, json_path) in &step.extract {
            if let Some(value_str) =
                extract_json_value(&data, json_path).and_then(json_value_to_string)
            {
                vars.insert(var_name.clone(), value_str);
            }
        }

        // Delay between steps if configured
        let delay = config.get_step_delay_seconds();
        if delay > 0.0 {
            sleep(Duration::from_secs_f64(delay)).await;
        }
    }

    Ok(PipelineOutcome::Live(vars))
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
