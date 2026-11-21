use crate::utils::app_config_dir;
use anyhow::Result;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const EXTENSION_API_VERSION: u32 = 1;
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
pub const MAX_TIMEOUT_MS: u64 = 300_000;
pub const DEFAULT_MEMORY_MB: u64 = 64;
pub const MAX_MEMORY_MB: u64 = 512;
pub const MAX_SCRIPT_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Hook {
    BeforeRecording,
    AfterRecording,
    BeforeUpload,
    AfterUpload,
    OnError,
}

impl Hook {
    pub const ALL: [Hook; 5] = [
        Hook::BeforeRecording,
        Hook::AfterRecording,
        Hook::BeforeUpload,
        Hook::AfterUpload,
        Hook::OnError,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Hook::BeforeRecording => "before_recording",
            Hook::AfterRecording => "after_recording",
            Hook::BeforeUpload => "before_upload",
            Hook::AfterUpload => "after_upload",
            Hook::OnError => "on_error",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Hook::ALL.into_iter().find(|hook| hook.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    #[serde(rename = "session.read")]
    SessionRead,
    Network,
    Process,
}

impl Permission {
    pub fn as_str(self) -> &'static str {
        match self {
            Permission::SessionRead => "session.read",
            Permission::Network => "network",
            Permission::Process => "process",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Permission::SessionRead => "read the recording and thumbnail for this session",
            Permission::Network => "make HTTP requests",
            Permission::Process => "run programs as your user",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingKind {
    String,
    Number,
    Integer,
    Boolean,
    StringArray,
    Toml,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SettingDef {
    #[serde(rename = "type")]
    pub kind: SettingKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u64,
}

fn default_timeout_ms() -> u64 {
    DEFAULT_TIMEOUT_MS
}

fn default_memory_mb() -> u64 {
    DEFAULT_MEMORY_MB
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            timeout_ms: DEFAULT_TIMEOUT_MS,
            memory_mb: DEFAULT_MEMORY_MB,
        }
    }
}

impl Limits {
    pub fn effective_timeout_ms(&self) -> u64 {
        self.timeout_ms.clamp(1_000, MAX_TIMEOUT_MS)
    }

    pub fn effective_memory_bytes(&self) -> usize {
        let mb = self.memory_mb.clamp(8, MAX_MEMORY_MB);
        (mb as usize).saturating_mul(1024 * 1024)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionManifest {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_recorder_version: Option<String>,
    #[serde(default = "default_api")]
    pub api: u32,
    pub main: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub hooks: Vec<Hook>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub settings: HashMap<String, SettingDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
}

fn default_api() -> u32 {
    EXTENSION_API_VERSION
}

impl ExtensionManifest {
    pub fn extensions_dir() -> PathBuf {
        app_config_dir().join("extensions")
    }

    pub fn main_path(&self, dir: &Path) -> PathBuf {
        dir.join(&self.main)
    }

    pub fn wants(&self, hook: Hook) -> bool {
        self.hooks.contains(&hook)
    }

    pub fn files_to_install(&self) -> Vec<String> {
        let mut files = Vec::new();
        for file in std::iter::once(&self.main).chain(self.files.iter()) {
            if !files.contains(file) {
                files.push(file.clone());
            }
        }
        files
    }

    pub fn validate(&self, source: &str) -> Result<()> {
        if !valid_id(&self.id) {
            return Err(anyhow::anyhow!(
                "Extension config {}: `id` must match ^[a-z][a-z0-9_]{{0,63}}$ (got '{}')",
                source,
                self.id
            ));
        }
        if self.name.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "Extension config {}: `name` must not be empty",
                source
            ));
        }
        if self.version.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "Extension config {}: `version` must not be empty",
                source
            ));
        }
        if self.api != EXTENSION_API_VERSION {
            return Err(anyhow::anyhow!(
                "Extension config {}: unsupported `api` {} (this build supports {})",
                source,
                self.api,
                EXTENSION_API_VERSION
            ));
        }
        if self.hooks.is_empty() {
            return Err(anyhow::anyhow!(
                "Extension config {}: `hooks` must contain at least one hook",
                source
            ));
        }
        let mut seen = HashSet::new();
        for hook in &self.hooks {
            if !seen.insert(*hook) {
                return Err(anyhow::anyhow!(
                    "Extension config {}: duplicate hook '{}'",
                    source,
                    hook.as_str()
                ));
            }
        }
        let mut seen = HashSet::new();
        for permission in &self.permissions {
            if !seen.insert(*permission) {
                return Err(anyhow::anyhow!(
                    "Extension config {}: duplicate permission '{}'",
                    source,
                    permission.as_str()
                ));
            }
        }
        validate_relative(&self.main, source, "`main`")?;
        for file in &self.files {
            validate_relative(file, source, "`files`")?;
        }
        if let Some(ref req_str) = self.stream_recorder_version {
            let req = VersionReq::parse(req_str).map_err(|e| {
                anyhow::anyhow!(
                    "Extension config {}: `stream_recorder_version` '{}' is not a valid semver requirement: {}",
                    source,
                    req_str,
                    e
                )
            })?;
            let app_version = Version::parse(env!("CARGO_PKG_VERSION"))
                .expect("CARGO_PKG_VERSION is always valid semver");
            if !req.matches(&app_version) {
                return Err(anyhow::anyhow!(
                    "Extension config {}: requires stream recorder '{}' but running version is '{}'",
                    source,
                    req_str,
                    app_version
                ));
            }
        }
        for (key, def) in &self.settings {
            if !valid_key(key) {
                return Err(anyhow::anyhow!(
                    "Extension config {}: setting name '{}' must match ^[a-z][a-z0-9_]{{0,63}}$",
                    source,
                    key
                ));
            }
            if let Some(default) = &def.default
                && !value_matches_kind(default, def.kind)
            {
                return Err(anyhow::anyhow!(
                    "Extension config {}: default for setting '{}' does not match type {:?}",
                    source,
                    key,
                    def.kind
                ));
            }
        }
        Ok(())
    }

    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let manifest_path = dir.join("extension.json");
        let content = fs::read_to_string(&manifest_path).map_err(|e| {
            anyhow::anyhow!("Failed to read extension config {:?}: {}", manifest_path, e)
        })?;
        let manifest: Self = serde_json::from_str(&content).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse extension config {:?}: {}",
                manifest_path,
                e
            )
        })?;
        manifest.validate(&format!("{:?}", manifest_path))?;

        let main_path = manifest.main_path(dir);
        if !main_path.is_file() {
            return Err(anyhow::anyhow!(
                "Extension config {:?}: main script {:?} does not exist",
                manifest_path,
                main_path
            ));
        }
        let len = fs::metadata(&main_path)?.len();
        if len > MAX_SCRIPT_BYTES {
            return Err(anyhow::anyhow!(
                "Extension config {:?}: main script is {} bytes, maximum is {}",
                manifest_path,
                len,
                MAX_SCRIPT_BYTES
            ));
        }

        Ok(manifest)
    }
}

pub fn valid_id(value: &str) -> bool {
    if value.len() > 64 {
        return false;
    }
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub fn valid_key(value: &str) -> bool {
    valid_id(value)
}

pub fn validate_relative(value: &str, source: &str, field: &str) -> Result<()> {
    if value.is_empty()
        || value.contains('\\')
        || Path::new(value).is_absolute()
        || Path::new(value).components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(anyhow::anyhow!(
            "Extension config {}: {} path '{}' must be relative and must not contain '..'",
            source,
            field,
            value
        ));
    }
    Ok(())
}

fn value_matches_kind(value: &Value, kind: SettingKind) -> bool {
    match kind {
        SettingKind::String => value.is_string(),
        SettingKind::Number => value.is_number(),
        SettingKind::Integer => value.as_i64().is_some() || value.as_u64().is_some(),
        SettingKind::Boolean => value.is_boolean(),
        SettingKind::StringArray => value
            .as_array()
            .is_some_and(|items| items.iter().all(Value::is_string)),
        SettingKind::Toml => true,
    }
}

/// Loads `config.toml` for an extension, fills in manifest defaults, and
/// validates every provided value against its declared type.
pub fn load_settings(
    manifest: &ExtensionManifest,
    dir: &Path,
) -> Result<Map<String, Value>, String> {
    let mut values = Map::new();

    for (key, def) in &manifest.settings {
        if let Some(default) = &def.default {
            values.insert(key.clone(), default.clone());
        }
    }

    let config_path = dir.join("config.toml");
    if !config_path.exists() {
        return Ok(values);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("failed to read {:?}: {}", config_path, e))?;
    let table: toml::Value = toml::from_str(&content)
        .map_err(|e| format!("failed to parse {:?}: {}", config_path, e))?;
    let table = table
        .as_table()
        .ok_or_else(|| format!("{:?} must contain a TOML table", config_path))?;

    for (key, raw) in table {
        let value = serde_json::to_value(raw)
            .map_err(|e| format!("failed to convert setting '{}': {}", key, e))?;
        let Some(def) = manifest.settings.get(key) else {
            eprintln!(
                "Extension {}: ignoring unknown setting '{}' in {:?}",
                manifest.id, key, config_path
            );
            continue;
        };
        if !value_matches_kind(&value, def.kind) {
            return Err(format!(
                "setting '{}' in {:?} does not match declared type {:?}",
                key, config_path, def.kind
            ));
        }
        values.insert(key.clone(), value);
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_manifest() -> ExtensionManifest {
        ExtensionManifest {
            schema: None,
            id: "test_ext".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            icon: None,
            stream_recorder_version: None,
            api: EXTENSION_API_VERSION,
            main: "main.lua".to_string(),
            files: Vec::new(),
            hooks: vec![Hook::AfterRecording],
            permissions: Vec::new(),
            limits: Limits::default(),
            settings: HashMap::new(),
            source_url: None,
        }
    }

    #[test]
    fn valid_ids_are_accepted() {
        assert!(valid_id("death_counter"));
        assert!(valid_id("a"));
        assert!(valid_id("abc123"));
    }

    #[test]
    fn invalid_ids_are_rejected() {
        assert!(!valid_id("Death"));
        assert!(!valid_id("1abc"));
        assert!(!valid_id("a-b"));
        assert!(!valid_id(""));
        assert!(!valid_id(&"a".repeat(65)));
    }

    #[test]
    fn relative_paths_reject_traversal() {
        assert!(validate_relative("main.lua", "test", "`main`").is_ok());
        assert!(validate_relative("assets/a.png", "test", "`main`").is_ok());
        assert!(validate_relative("../escape.lua", "test", "`main`").is_err());
        assert!(validate_relative("/abs.lua", "test", "`main`").is_err());
        assert!(validate_relative("a\\b.lua", "test", "`main`").is_err());
    }

    #[test]
    fn defaults_and_limits_are_clamped() {
        let limits = Limits {
            timeout_ms: 1,
            memory_mb: 10_000,
        };
        assert_eq!(limits.effective_timeout_ms(), 1_000);
        assert_eq!(
            limits.effective_memory_bytes(),
            MAX_MEMORY_MB as usize * 1024 * 1024
        );
    }

    #[test]
    fn manifest_requires_at_least_one_hook() {
        let mut manifest = minimal_manifest();
        manifest.hooks.clear();
        assert!(manifest.validate("test").is_err());
    }

    #[test]
    fn manifest_rejects_duplicate_permissions() {
        let mut manifest = minimal_manifest();
        manifest.permissions = vec![Permission::Network, Permission::Network];
        assert!(manifest.validate("test").is_err());
    }

    #[test]
    fn repository_example_manifest_is_valid() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("extensions")
            .join("example");
        let manifest =
            ExtensionManifest::load_from_dir(&dir).expect("example manifest should load");
        assert_eq!(manifest.id, "example");

        let settings = load_settings(&manifest, &dir).expect("example settings should load");
        assert_eq!(
            settings.get("label").and_then(Value::as_str),
            Some("Duration")
        );
    }

    #[test]
    fn settings_merge_defaults_with_user_config() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("config.toml"), "label = \"Custom\"\n")
            .expect("write config");
        let mut manifest = minimal_manifest();
        manifest.settings.insert(
            "label".to_string(),
            SettingDef {
                kind: SettingKind::String,
                default: Some(Value::String("Duration".to_string())),
                description: None,
            },
        );

        let settings = load_settings(&manifest, dir.path()).expect("settings should load");
        assert_eq!(
            settings.get("label"),
            Some(&Value::String("Custom".to_string()))
        );
    }

    #[test]
    fn toml_settings_accept_arrays_of_tables() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            dir.path().join("config.toml"),
            r#"
[[rules]]
username = "MaliceTheSuccubus"
key = "spanks"
label = "Spanks"
type = "number"

[[rules]]
username = "MaliceTheSuccubus"
key = "cums"
label = "Cums"
type = "number"
"#,
        )
        .expect("write config");

        let mut manifest = minimal_manifest();
        manifest.settings.insert(
            "rules".to_string(),
            SettingDef {
                kind: SettingKind::Toml,
                default: Some(Value::Array(Vec::new())),
                description: None,
            },
        );

        let settings = load_settings(&manifest, dir.path()).expect("settings should load");
        let rules = settings
            .get("rules")
            .and_then(Value::as_array)
            .expect("rules should be an array");
        assert_eq!(rules.len(), 2);
        assert_eq!(
            rules[0]["username"],
            Value::String("MaliceTheSuccubus".to_string())
        );
        assert_eq!(rules[1]["label"], Value::String("Cums".to_string()));
    }

    #[test]
    fn settings_default_types_are_validated() {
        let mut manifest = minimal_manifest();
        manifest.settings.insert(
            "count".to_string(),
            SettingDef {
                kind: SettingKind::Integer,
                default: Some(Value::String("nope".to_string())),
                description: None,
            },
        );
        assert!(manifest.validate("test").is_err());
    }
}
