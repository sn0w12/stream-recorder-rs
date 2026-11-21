use crate::extension::manifest::{EXTENSION_API_VERSION, ExtensionManifest, Hook};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct ExtensionIdentity {
    pub id: String,
    pub name: String,
    pub version: String,
}

impl ExtensionIdentity {
    pub fn new(manifest: &ExtensionManifest) -> Self {
        Self {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PlatformContext {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionContext {
    pub id: String,
    pub started_at: String,
    pub username: String,
    pub platform: PlatformContext,
    pub stream: StreamContext,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RecordingContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segments: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_continuation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playback_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorContext {
    pub stage: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uploader: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retryable: Option<bool>,
}

impl ErrorContext {
    pub fn new(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            stage: stage.into(),
            message: message.into(),
            uploader: None,
            file: None,
            status_code: None,
            retryable: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct HookContext {
    pub api: u32,
    pub hook: &'static str,
    pub extension: ExtensionIdentity,
    pub settings: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording: Option<RecordingContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uploads: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorContext>,
}

impl HookContext {
    pub fn new(manifest: &ExtensionManifest, settings: Map<String, Value>, hook: Hook) -> Self {
        Self {
            api: EXTENSION_API_VERSION,
            hook: hook.as_str(),
            extension: ExtensionIdentity::new(manifest),
            settings,
            session: None,
            recording: None,
            uploads: None,
            error: None,
        }
    }

    pub fn to_value(&self) -> Result<Value, String> {
        serde_json::to_value(self).map_err(|e| format!("failed to serialize hook context: {}", e))
    }
}

pub fn uploads_value(
    video: &HashMap<String, Vec<String>>,
    image: &HashMap<String, Vec<String>>,
) -> Value {
    fn entries(results: &HashMap<String, Vec<String>>) -> Value {
        let mut names: Vec<&String> = results.keys().collect();
        names.sort();
        Value::Array(
            names
                .into_iter()
                .map(|name| {
                    let mut item = Map::new();
                    item.insert("uploader".to_string(), Value::String(name.clone()));
                    item.insert(
                        "urls".to_string(),
                        Value::Array(results[name].iter().cloned().map(Value::String).collect()),
                    );
                    Value::Object(item)
                })
                .collect(),
        )
    }

    let mut map = Map::new();
    map.insert("video".to_string(), entries(video));
    map.insert("image".to_string(), entries(image));
    Value::Object(map)
}
