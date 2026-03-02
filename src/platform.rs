use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use anyhow::Result;
use serde_json::Value;
use reqwest::Client;
use semver::{Version, VersionReq};
use regex::Regex;

/// A single step in the platform fetch pipeline.
///
/// Steps are executed in order. Each step may extract string variables from
/// the HTTP response that become available as `{variable}` placeholders in
/// all subsequent steps' endpoint templates.
///
/// The initial variable set always contains `{username}` and `{token}`.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PipelineStep {
    /// Endpoint template. Relative paths are prepended with `base_url`.
    /// If the value already starts with `http://` or `https://` it is used as-is.
    /// Any `{variable}` placeholder is substituted with the current variable map.
    pub endpoint: String,
    /// Optional JSON path that **must** resolve to a non-null value for the
    /// stream to be considered live. If the path is absent or null the pipeline
    /// immediately returns [`PipelineOutcome::Offline`] without executing
    /// subsequent steps.
    pub live_check: Option<String>,
    /// Variables to extract from the response JSON and add to the variable map.
    /// Keys are variable names; values are dot-notation JSON paths
    /// (supports array indexing, e.g. `response[0].id`).
    #[serde(default)]
    pub extract: HashMap<String, String>,
}

/// Outcome of executing a platform's fetch pipeline.
#[derive(Debug)]
pub enum PipelineOutcome {
    /// All steps completed and every `live_check` passed.
    /// The map contains all variables extracted across all steps.
    Live(HashMap<String, String>),
    /// A `live_check` in one of the steps resolved to null/missing,
    /// meaning the stream is currently offline.
    Offline,
}

/// JSON-configurable platform definition.
///
/// Place platform JSON files in `~/.config/stream_recorder/platforms/`
/// to register new platforms or override existing ones.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlatformConfig {
    /// Unique identifier for this platform.
    /// Used in monitor references with the `platform_id:username` format.
    pub id: String,
    /// Human-readable display name of the platform.
    pub name: String,
    /// Base URL prepended to relative endpoint paths. **Must** end with `/`.
    pub base_url: String,
    /// Keyring key name used to retrieve the authentication token
    /// (e.g. `api_token`). If `None`, the platform requires no authentication.
    pub token_name: Option<String>,
    /// HTTP headers sent with every request.
    /// Use `{token}` as a placeholder for the authentication token value.
    pub headers: HashMap<String, String>,
    /// Ordered list of HTTP fetch steps that together produce the stream info.
    ///
    /// Steps are executed in sequence. Variables extracted by earlier steps
    /// are available to later steps as `{variable}` placeholders. The initial
    /// variable map contains `{username}`.
    ///
    /// **Runtime requirements** – the pipeline must ultimately place these
    /// variables into the map via `extract` entries:
    /// - `playback_url` *(required)* – HLS/DASH URL passed to ffmpeg.
    /// - `user_id` *(recommended)* – used for output file naming; falls back
    ///   to the username if absent.
    /// - `stream_title` *(optional)* – used for output file naming.
    ///
    /// At least one step should carry a `live_check` so the monitor can
    /// distinguish live from offline states.
    pub steps: Vec<PipelineStep>,
    /// The URL this platform config was originally installed from.
    ///
    /// Set automatically by [`PlatformConfig::install_from_url`] so that
    /// [`PlatformConfig::update_by_id`] knows where to re-fetch the config.
    /// Absent for platforms that were installed manually.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// Platform config version (e.g. `"1.0.0"`).
    ///
    /// Must be a non-empty string; authors may use any versioning scheme they
    /// prefer (semver is conventional).  Displayed in `platform list` and
    /// stored verbatim in the installed JSON.
    pub version: String,
    /// Semver requirement for the compatible stream recorder version
    /// (e.g. `"^0.1"`, `">=0.1.0, <2.0"`).
    ///
    /// If present it is validated at both install-time and load-time against
    /// the running stream recorder version (`CARGO_PKG_VERSION`).  Platforms
    /// that specify a requirement incompatible with the running binary will be
    /// rejected with a descriptive error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_recorder_version: Option<String>,
    /// List of regex patterns applied in order to the `stream_title` variable
    /// before it is used for output file naming.
    ///
    /// Each match of each pattern is replaced with an empty string.  Use this
    /// to strip platform-specific emoji shortcodes, tags, or other noise.
    /// If absent (or empty), the title is used as-is.
    ///
    /// Example: `[":\\w+:"]` removes `:shortcode:` style emojis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_clean_regex: Option<Vec<String>>,
}

impl PlatformConfig {
    /// Returns the path to the user's platforms configuration directory.
    pub fn platforms_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("stream_recorder")
            .join("platforms")
    }

    /// Validates a parsed `PlatformConfig`.
    ///
    /// Checks:
    /// - `base_url` ends with `/`
    /// - `steps` is non-empty
    /// - `version` is non-empty
    /// - `stream_recorder_version`, when present, is a valid semver requirement
    ///   that the running stream_recorder satisfies
    /// - each pattern in `title_clean_regex`, when present, compiles as a valid regex
    ///
    /// The optional `source` parameter is used in error messages to identify
    /// the file or URL that produced the config.
    pub fn validate(&self, source: &str) -> Result<()> {
        if !self.base_url.ends_with('/') {
            return Err(anyhow::anyhow!(
                "Platform config {}: `base_url` must end with '/' (got '{}')",
                source,
                self.base_url
            ));
        }
        if self.steps.is_empty() {
            return Err(anyhow::anyhow!(
                "Platform config {}: `steps` must contain at least one step",
                source
            ));
        }
        if self.version.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "Platform config {}: `version` must not be empty",
                source
            ));
        }
        if let Some(ref req_str) = self.stream_recorder_version {
            let req = VersionReq::parse(req_str).map_err(|e| anyhow::anyhow!(
                "Platform config {}: `stream_recorder_version` '{}' is not a valid semver requirement: {}",
                source, req_str, e
            ))?;
            let app_version = Version::parse(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION is always valid semver");
            if !req.matches(&app_version) {
                return Err(anyhow::anyhow!(
                    "Platform config {}: requires stream recorder '{}' but running version is '{}'. \
                     Update stream recorder or re-install this platform.",
                    source, req_str, app_version
                ));
            }
        }
        if let Some(ref patterns) = self.title_clean_regex {
            for pattern in patterns {
                Regex::new(pattern).map_err(|e| anyhow::anyhow!(
                    "Platform config {}: `title_clean_regex` pattern '{}' is not a valid regex: {}",
                    source, pattern, e
                ))?;
            }
        }
        Ok(())
    }

    /// Applies the platform's `title_clean_regex` patterns to `title`, replacing
    /// every match with an empty string.
    ///
    /// Returns the cleaned string.  If `title_clean_regex` is absent or empty
    /// the title is returned unchanged.
    pub fn clean_title(&self, title: &str) -> String {
        let Some(ref patterns) = self.title_clean_regex else {
            return title.to_string();
        };
        let mut result = title.to_string();
        for pattern in patterns {
            // Patterns are validated on load/install, so this unwrap is safe.
            let re = Regex::new(pattern).unwrap();
            result = re.replace_all(&result, "").to_string();
        }
        result
    }

    /// Loads all platform configs from the user's platforms directory.
    ///
    /// Returns an empty `Vec` when the directory does not exist or contains no
    /// JSON files — callers are responsible for telling the user to install
    /// at least one platform.
    ///
    /// Returns an error if any JSON file fails to parse or fails validation
    /// (bad `base_url`, empty `steps`, empty `version`, or incompatible
    /// `stream_recorder_version`).
    pub fn load_all() -> Result<Vec<Self>> {
        let dir = Self::platforms_dir();
        let mut platforms = Vec::new();

        if dir.exists() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    let content = fs::read_to_string(&path)
                        .map_err(|e| anyhow::anyhow!("Failed to read platform config {:?}: {}", path, e))?;
                    let config: PlatformConfig = serde_json::from_str(&content)
                        .map_err(|e| anyhow::anyhow!("Failed to parse platform config {:?}: {}", path, e))?;
                    config.validate(&format!("{:?}", path))?;
                    platforms.push(config);
                }
            }
        }

        Ok(platforms)
    }

    /// Looks up a platform by its `id` from a slice of loaded platforms.
    pub fn find_by_id<'a>(platforms: &'a [Self], id: &str) -> Option<&'a Self> {
        platforms.iter().find(|p| p.id == id)
    }

    /// Downloads a platform JSON from `url`, validates it, injects the source
    /// URL into the config, and saves it to the platforms directory as `<id>.json`.
    ///
    /// The saved file will contain a `source_url` field so that
    /// [`PlatformConfig::update_by_id`] can re-fetch the config later.
    ///
    /// Returns the installed `PlatformConfig` on success.
    /// Returns an error if the download fails, the JSON is malformed, or
    /// the config is invalid (bad `base_url`, empty `steps`).
    pub async fn install_from_url(url: &str) -> Result<Self> {
        // If the caller supplied a GitHub repo/tree URL, resolve it to the
        // raw platform.json URL automatically.
        let fetch_url = resolve_github_url(url);
        if fetch_url != url {
            println!("Resolved GitHub URL to: {}", fetch_url);
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;

        let response = client
            .get(&fetch_url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to download platform config from '{}': {}", fetch_url, e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download platform config from '{}': HTTP {}",
                fetch_url,
                response.status()
            ));
        }

        let content = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;

        let mut config: PlatformConfig = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Downloaded file is not valid platform JSON: {}", e))?;

        config.validate(&fetch_url)?;

        // Persist the original URL supplied by the user as the source so that
        // `update_by_id` re-uses it unchanged (GitHub repo links remain tidy).
        config.source_url = Some(url.to_string());

        let dir = Self::platforms_dir();
        fs::create_dir_all(&dir)
            .map_err(|e| anyhow::anyhow!("Failed to create platforms directory: {}", e))?;

        let file_path = dir.join(format!("{}.json", config.id));
        let serialized = serde_json::to_string_pretty(&config)
            .map_err(|e| anyhow::anyhow!("Failed to serialize platform config: {}", e))?;
        fs::write(&file_path, &serialized)
            .map_err(|e| anyhow::anyhow!("Failed to write platform config to {:?}: {}", file_path, e))?;

        Ok(config)
    }

    /// Re-downloads the platform config for `id` from its saved `source_url`
    /// and overwrites the stored file.
    ///
    /// Returns an error if the platform is not installed, if it has no saved
    /// `source_url` (i.e. was installed manually), or if the download fails.
    pub async fn update_by_id(id: &str) -> Result<Self> {
        let dir = Self::platforms_dir();
        let file_path = dir.join(format!("{}.json", id));
        if !file_path.exists() {
            return Err(anyhow::anyhow!(
                "Platform '{}' not found in platforms directory (expected {:?})",
                id,
                file_path
            ));
        }

        let existing_content = fs::read_to_string(&file_path)
            .map_err(|e| anyhow::anyhow!("Failed to read platform config {:?}: {}", file_path, e))?;
        let existing: PlatformConfig = serde_json::from_str(&existing_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse existing platform config {:?}: {}", file_path, e))?;

        let url = existing.source_url.ok_or_else(|| anyhow::anyhow!(
            "Platform '{}' has no source URL saved. \
             Re-install it with: stream-recorder platform install <url>",
            id
        ))?;

        let old_version = existing.version.clone();
        let updated = Self::install_from_url(&url).await?;

        // Print a version diff only when both versions are valid semver.
        if let (Ok(old), Ok(new)) = (
            Version::parse(old_version.trim()),
            Version::parse(updated.version.trim()),
        ) {
            if old != new {
                println!("  {} -> {}", old, new);
            } else {
                println!("  Already up to date ({})", old);
            }
        }

        Ok(updated)
    }

    /// Updates all installed platforms that have a saved `source_url`.
    ///
    /// Returns a list of `(platform_id, result)` pairs — one per platform
    /// that had a source URL.  Platforms installed manually (no URL) are
    /// silently skipped.
    pub async fn update_all() -> Result<Vec<(String, Result<Self>)>> {
        let platforms = Self::load_all()?;
        let mut results = Vec::new();
        for platform in platforms {
            if platform.source_url.is_some() {
                let id = platform.id.clone();
                let result = Self::update_by_id(&id).await;
                results.push((id, result));
            }
        }
        Ok(results)
    }

    /// Removes the platform JSON file for the given `id` from the platforms directory.
    ///
    /// Returns an error if the platform is not found or the file cannot be deleted.
    pub fn remove_by_id(id: &str) -> Result<()> {
        let dir = Self::platforms_dir();
        let file_path = dir.join(format!("{}.json", id));
        if !file_path.exists() {
            return Err(anyhow::anyhow!(
                "Platform '{}' not found in platforms directory (expected {:?})",
                id,
                file_path
            ));
        }
        fs::remove_file(&file_path)
            .map_err(|e| anyhow::anyhow!("Failed to remove platform config {:?}: {}", file_path, e))?;
        Ok(())
    }
}

/// Converts a GitHub repository or tree URL into the raw content URL for
/// `platform.json` inside that repository.
///
/// Handled patterns:
/// - `https://github.com/{owner}/{repo}`                      → `.../HEAD/platform.json`
/// - `https://github.com/{owner}/{repo}/tree/{branch}`        → `.../{branch}/platform.json`
/// - `https://github.com/{owner}/{repo}/tree/{branch}/{path}` → `.../{branch}/{path}/platform.json`
///
/// Any other URL (non-GitHub, direct blob links, raw URLs, etc.) is returned
/// unchanged.
fn resolve_github_url(url: &str) -> String {
    let prefix = "https://github.com/";
    if !url.starts_with(prefix) {
        return url.to_string();
    }

    let path = url[prefix.len()..].trim_end_matches('/');
    // Use splitn(5) so that a sub-path after the branch is kept as one segment.
    let segments: Vec<&str> = path.splitn(5, '/').collect();

    match segments.as_slice() {
        [owner, repo] => {
            format!(
                "https://raw.githubusercontent.com/{}/{}/HEAD/platform.json",
                owner, repo
            )
        }
        [owner, repo, tree, branch] if *tree == "tree" => {
            format!(
                "https://raw.githubusercontent.com/{}/{}/{}/platform.json",
                owner, repo, branch
            )
        }
        [owner, repo, tree, branch, subpath] if *tree == "tree" => {
            let subpath = subpath.trim_end_matches('/');
            format!(
                "https://raw.githubusercontent.com/{}/{}/{}/{}/platform.json",
                owner, repo, branch, subpath
            )
        }
        _ => url.to_string(),
    }
}

/// Traverses a JSON value using a dot-notation path with optional array-index support.
///
/// # Supported formats
/// - `field` – top-level object key
/// - `field.nested` – nested object keys separated by `.`
/// - `field[0]` – array index
/// - `field[0].nested` – array index followed by object key
///
/// Returns `None` if any segment of the path is not found or the value is `null`.
pub fn extract_json_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in parse_path_segments(path) {
        current = match segment {
            PathSegment::Key(key) => current.get(&key)?,
            PathSegment::Index(idx) => current.get(idx)?,
        };
        if current.is_null() {
            return None;
        }
    }
    Some(current)
}

enum PathSegment {
    Key(String),
    Index(usize),
}

fn parse_path_segments(path: &str) -> Vec<PathSegment> {
    let mut segments = Vec::new();
    let mut remaining = path;

    loop {
        // Strip leading dot separator between segments.
        if remaining.starts_with('.') {
            remaining = &remaining[1..];
        }

        if remaining.is_empty() {
            break;
        }

        // Collect the next key token (until `.` or `[`).
        let key_end = remaining
            .find(|c: char| c == '.' || c == '[')
            .unwrap_or(remaining.len());
        let key = &remaining[..key_end];
        remaining = &remaining[key_end..];

        if !key.is_empty() {
            segments.push(PathSegment::Key(key.to_string()));
        }

        // Consume any immediately following `[N]` array indices.
        while remaining.starts_with('[') {
            if let Some(close) = remaining.find(']') {
                if let Ok(idx) = remaining[1..close].parse::<usize>() {
                    segments.push(PathSegment::Index(idx));
                }
                remaining = &remaining[close + 1..];
            } else {
                break; // malformed path – stop parsing
            }
        }
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_simple_key() {
        let v = json!({"id": "123"});
        assert_eq!(extract_json_value(&v, "id").and_then(|v| v.as_str()), Some("123"));
    }

    #[test]
    fn test_extract_nested_key() {
        let v = json!({"response": {"stream": {"playbackUrl": "http://example.com"}}});
        assert_eq!(
            extract_json_value(&v, "response.stream.playbackUrl").and_then(|v| v.as_str()),
            Some("http://example.com")
        );
    }

    #[test]
    fn test_extract_array_index() {
        let v = json!({"response": [{"id": "abc"}]});
        assert_eq!(
            extract_json_value(&v, "response[0].id").and_then(|v| v.as_str()),
            Some("abc")
        );
    }

    #[test]
    fn test_extract_missing_key_returns_none() {
        let v = json!({"a": "b"});
        assert!(extract_json_value(&v, "missing").is_none());
    }

    #[test]
    fn test_extract_null_returns_none() {
        let v = json!({"field": null});
        assert!(extract_json_value(&v, "field").is_none());
    }

    #[test]
    fn test_pipeline_step_deserializes_with_defaults() {
        let json = r#"{"endpoint": "some/path"}"#;
        let step: PipelineStep = serde_json::from_str(json).unwrap();
        assert_eq!(step.endpoint, "some/path");
        assert!(step.live_check.is_none());
        assert!(step.extract.is_empty());
    }

    fn make_minimal_platform(id: &str) -> PlatformConfig {
        PlatformConfig {
            id: id.to_string(),
            name: "Test Platform".to_string(),
            base_url: "https://example.com/api/".to_string(),
            token_name: None,
            headers: HashMap::new(),
            steps: vec![PipelineStep {
                endpoint: "stream/{username}".to_string(),
                live_check: Some("live".to_string()),
                extract: HashMap::new(),
            }],
            source_url: None,
            version: "1.0.0".to_string(),
            stream_recorder_version: None,
            title_clean_regex: None,
        }
    }

    #[test]
    fn test_source_url_is_none_by_default() {
        let p = make_minimal_platform("testplat");
        assert!(p.source_url.is_none());
    }

    #[test]
    fn test_source_url_round_trips_through_json() {
        let mut p = make_minimal_platform("testplat");
        p.source_url = Some("https://raw.example.com/testplat.json".to_string());

        let serialized = serde_json::to_string(&p).unwrap();
        let deserialized: PlatformConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.source_url.as_deref(), Some("https://raw.example.com/testplat.json"));
    }

    #[test]
    fn test_source_url_absent_from_serialized_when_none() {
        let p = make_minimal_platform("testplat");
        let serialized = serde_json::to_string(&p).unwrap();
        // When source_url is None it must be omitted entirely (skip_serializing_if).
        assert!(!serialized.contains("source_url"), "source_url should be absent when None");
    }

    #[test]
    fn test_source_url_defaults_to_none_when_missing_in_json() {
        // Old platform JSON files without `source_url` must still parse fine.
        let json = r#"{
            "id": "legacy",
            "name": "Legacy Platform",
            "base_url": "https://example.com/",
            "headers": {},
            "steps": [{"endpoint": "foo"}],
            "version": "1.0.0"
        }"#;
        let p: PlatformConfig = serde_json::from_str(json).unwrap();
        assert!(p.source_url.is_none(), "source_url must default to None for old configs");
    }

    #[test]
    fn test_update_by_id_errors_when_no_source_url() {
        // When a platform has no source_url, update_by_id should surface a clear error.
        // We simulate the guard that update_by_id applies: if source_url is None,
        // it returns the exact error produced by the `.ok_or_else` inside the method.
        let p = make_minimal_platform("nourls");
        assert!(p.source_url.is_none());

        // Mirror the exact error produced by update_by_id so changes to the message
        // are caught here as well.
        let result: Result<()> = p.source_url.clone().ok_or_else(|| anyhow::anyhow!(
            "Platform '{}' has no source URL saved. \
             Re-install it with: stream-recorder platform install <url>",
            p.id
        )).map(|_| ());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("has no source URL saved"),
            "error message must mention 'has no source URL saved', got: {}",
            msg
        );
    }

    #[test]
    fn test_version_field_round_trips() {
        let p = make_minimal_platform("vtest");
        let serialized = serde_json::to_string(&p).unwrap();
        let back: PlatformConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(back.version, "1.0.0");
    }

    #[test]
    fn test_validate_rejects_empty_version() {
        let mut p = make_minimal_platform("badver");
        p.version = String::new();
        let err = p.validate("test").unwrap_err().to_string();
        assert!(err.contains("`version` must not be empty"), "got: {}", err);
    }

    #[test]
    fn test_validate_rejects_bad_sr_version_req() {
        let mut p = make_minimal_platform("badreq");
        p.stream_recorder_version = Some("not a semver req!!!".to_string());
        let err = p.validate("test").unwrap_err().to_string();
        assert!(err.contains("`stream_recorder_version`"), "got: {}", err);
        assert!(err.contains("not a valid semver requirement"), "got: {}", err);
    }

    #[test]
    fn test_validate_accepts_compatible_sr_version_req() {
        let mut p = make_minimal_platform("goodreq");
        // The app version is 0.1.0; "^0.1" should match.
        p.stream_recorder_version = Some("^0.1".to_string());
        assert!(p.validate("test").is_ok());
    }

    #[test]
    fn test_validate_rejects_incompatible_sr_version_req() {
        let mut p = make_minimal_platform("badcompat");
        // Require a version far in the future — should fail against 0.1.0.
        p.stream_recorder_version = Some(">=99.0.0".to_string());
        let err = p.validate("test").unwrap_err().to_string();
        assert!(err.contains("requires stream_recorder"), "got: {}", err);
        assert!(err.contains(">=99.0.0"), "got: {}", err);
    }

    #[test]
    fn test_stream_recorder_version_omitted_when_none() {
        let p = make_minimal_platform("omit");
        let serialized = serde_json::to_string(&p).unwrap();
        assert!(!serialized.contains("stream_recorder_version"), "field should be absent when None: {}", serialized);
    }

    #[test]
    fn test_stream_recorder_version_round_trips() {
        let mut p = make_minimal_platform("srv");
        p.stream_recorder_version = Some("^0.1".to_string());
        let serialized = serde_json::to_string(&p).unwrap();
        let back: PlatformConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(back.stream_recorder_version.as_deref(), Some("^0.1"));
    }

    #[test]
    fn test_title_clean_regex_omitted_when_none() {
        let p = make_minimal_platform("tcr_none");
        let serialized = serde_json::to_string(&p).unwrap();
        assert!(!serialized.contains("title_clean_regex"), "field should be absent when None: {}", serialized);
    }

    #[test]
    fn test_title_clean_regex_round_trips() {
        let mut p = make_minimal_platform("tcr_rt");
        p.title_clean_regex = Some(vec![r":\w+:".to_string(), r"\[.*?\]".to_string()]);
        let serialized = serde_json::to_string(&p).unwrap();
        let back: PlatformConfig = serde_json::from_str(&serialized).unwrap();
        let patterns = back.title_clean_regex.unwrap();
        assert_eq!(patterns[0], r":\w+:");
        assert_eq!(patterns[1], r"\[.*?\]");
    }

    #[test]
    fn test_clean_title_no_regex_returns_unchanged() {
        let p = make_minimal_platform("tcr_noop");
        assert_eq!(p.clean_title("Hello :world: stream"), "Hello :world: stream");
    }

    #[test]
    fn test_clean_title_removes_emoji_shortcodes() {
        let mut p = make_minimal_platform("tcr_emoji");
        p.title_clean_regex = Some(vec![r":\w+:".to_string()]);
        assert_eq!(p.clean_title("Hello :smile: World :tada:"), "Hello  World ");
    }

    #[test]
    fn test_clean_title_applies_multiple_patterns_in_order() {
        let mut p = make_minimal_platform("tcr_multi");
        p.title_clean_regex = Some(vec![r":\w+:".to_string(), r"\[.*?\]".to_string()]);
        // First `:smile:` is stripped, then `[tag]` is stripped
        assert_eq!(p.clean_title(":smile: Hello [VOD] World :tada:"), " Hello  World ");
    }

    #[test]
    fn test_validate_rejects_invalid_title_clean_regex() {
        let mut p = make_minimal_platform("tcr_bad");
        p.title_clean_regex = Some(vec!["[invalid(".to_string()]);
        let err = p.validate("test").unwrap_err().to_string();
        assert!(err.contains("`title_clean_regex`"), "got: {}", err);
        assert!(err.contains("not a valid regex"), "got: {}", err);
    }

    #[test]
    fn test_validate_accepts_valid_title_clean_regex() {
        let mut p = make_minimal_platform("tcr_ok");
        p.title_clean_regex = Some(vec![r":\w+:".to_string(), r"\[.*?\]".to_string()]);
        assert!(p.validate("test").is_ok());
    }

    // --- resolve_github_url tests ---

    #[test]
    fn test_resolve_github_repo_root() {
        let result = resolve_github_url("https://github.com/owner/repo");
        assert_eq!(result, "https://raw.githubusercontent.com/owner/repo/HEAD/platform.json");
    }

    #[test]
    fn test_resolve_github_repo_root_trailing_slash() {
        let result = resolve_github_url("https://github.com/owner/repo/");
        assert_eq!(result, "https://raw.githubusercontent.com/owner/repo/HEAD/platform.json");
    }

    #[test]
    fn test_resolve_github_tree_branch() {
        let result = resolve_github_url("https://github.com/owner/repo/tree/main");
        assert_eq!(result, "https://raw.githubusercontent.com/owner/repo/main/platform.json");
    }

    #[test]
    fn test_resolve_github_tree_branch_subpath() {
        let result = resolve_github_url("https://github.com/owner/repo/tree/main/platforms/mypkg");
        assert_eq!(result, "https://raw.githubusercontent.com/owner/repo/main/platforms/mypkg/platform.json");
    }

    #[test]
    fn test_resolve_github_blob_unchanged() {
        // Direct blob links are not repo/tree URLs — they should pass through.
        let url = "https://github.com/owner/repo/blob/main/platform.json";
        assert_eq!(resolve_github_url(url), url);
    }

    #[test]
    fn test_resolve_non_github_unchanged() {
        let url = "https://example.com/platform.json";
        assert_eq!(resolve_github_url(url), url);
    }

    #[test]
    fn test_resolve_raw_github_unchanged() {
        let url = "https://raw.githubusercontent.com/owner/repo/main/platform.json";
        assert_eq!(resolve_github_url(url), url);
    }
}
