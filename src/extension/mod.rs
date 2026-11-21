pub mod context;
pub mod hooks;
pub mod http;
pub mod install;
pub mod manifest;
pub mod process;
pub mod runtime;
pub mod store;

use crate::config::Config;
use manifest::{ExtensionManifest, Hook};
use serde_json::{Map, Value};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};

#[derive(Debug, Clone)]
pub struct ExtensionLoadError {
    pub path: PathBuf,
    pub error: String,
}

impl fmt::Display for ExtensionLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path.display(), self.error)
    }
}

#[derive(Debug, Clone)]
pub struct LoadedExtension {
    pub manifest: ExtensionManifest,
    pub dir: PathBuf,
    pub settings: Map<String, Value>,
}

impl LoadedExtension {
    pub fn workspace_dir(&self, session_id: &str) -> PathBuf {
        self.dir.join(".workspace").join(session_id)
    }

    pub fn hooks(&self) -> Vec<&'static str> {
        self.manifest
            .hooks
            .iter()
            .map(|hook| hook.as_str())
            .collect()
    }
}

#[derive(Debug, Default)]
pub struct ExtensionLoadReport {
    pub extensions: Vec<LoadedExtension>,
    pub errors: Vec<ExtensionLoadError>,
}

static REGISTRY: LazyLock<RwLock<Arc<Vec<LoadedExtension>>>> =
    LazyLock::new(|| RwLock::new(Arc::new(Vec::new())));

pub fn installed() -> Arc<Vec<LoadedExtension>> {
    REGISTRY
        .read()
        .expect("extension registry lock poisoned")
        .clone()
}

pub fn set_installed(extensions: Vec<LoadedExtension>) {
    *REGISTRY.write().expect("extension registry lock poisoned") = Arc::new(extensions);
}

pub fn init() -> ExtensionLoadReport {
    let report = load_report();
    set_installed(report.extensions.clone());
    report
}

pub fn extensions_dir() -> PathBuf {
    ExtensionManifest::extensions_dir()
}

pub fn extension_dir(id: &str) -> PathBuf {
    extensions_dir().join(id)
}

pub fn config_path(id: &str) -> PathBuf {
    extension_dir(id).join("config.toml")
}

pub fn load_report() -> ExtensionLoadReport {
    load_report_inner(false)
}

pub fn load_report_including_disabled() -> ExtensionLoadReport {
    load_report_inner(true)
}

fn load_report_inner(include_disabled: bool) -> ExtensionLoadReport {
    let dir = extensions_dir();
    let mut report = ExtensionLoadReport::default();

    if !dir.exists() {
        return report;
    }

    let disabled: Vec<String> = Config::get()
        .get_disabled_extensions()
        .into_iter()
        .map(|value| value.to_lowercase())
        .collect();

    let Ok(entries) = std::fs::read_dir(&dir) else {
        return report;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with('.') || !path.is_dir() {
            continue;
        }

        match load_one(&path) {
            Ok(extension) => {
                if !include_disabled && disabled.contains(&extension.manifest.id.to_lowercase()) {
                    continue;
                }
                report.extensions.push(extension);
            }
            Err(error) => report.errors.push(ExtensionLoadError { path, error }),
        }
    }

    report
        .extensions
        .sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    report
}

fn load_one(dir: &Path) -> Result<LoadedExtension, String> {
    let manifest = ExtensionManifest::load_from_dir(dir).map_err(|e| e.to_string())?;
    if let Some(name) = dir.file_name().and_then(|name| name.to_str())
        && name != manifest.id
    {
        return Err(format!(
            "extension directory name '{}' must match manifest id '{}'",
            name, manifest.id
        ));
    }
    let settings = manifest::load_settings(&manifest, dir)?;
    Ok(LoadedExtension {
        manifest,
        dir: dir.to_path_buf(),
        settings,
    })
}

pub fn load_extension_by_id(id: &str) -> Result<LoadedExtension, String> {
    let dir = extension_dir(id);
    if !dir.is_dir() {
        return Err(format!("extension '{}' is not installed", id));
    }
    load_one(&dir)
}

pub fn cleanup_session(session_id: &str) {
    store::remove_session(session_id);
    for extension in installed().iter() {
        let workspace = extension.workspace_dir(session_id);
        let _ = std::fs::remove_dir_all(workspace);
    }
}

pub fn find_hook_extensions(hook: Hook) -> Vec<LoadedExtension> {
    installed()
        .iter()
        .filter(|extension| extension.manifest.wants(hook))
        .cloned()
        .collect()
}
