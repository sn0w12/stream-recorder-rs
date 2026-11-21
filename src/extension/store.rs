use crate::extension::manifest::valid_key;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

pub const MAX_TEMPLATE_KEYS: usize = 100;
pub const MAX_TEMPLATE_VALUE_BYTES: usize = 64 * 1024;
pub const MAX_TEMPLATE_TOTAL_BYTES: usize = 1024 * 1024;
pub const MAX_STATE_VALUE_BYTES: usize = 64 * 1024;
pub const MAX_STATE_TOTAL_BYTES: usize = 256 * 1024;
pub const MAX_VALUE_DEPTH: usize = 4;

const SESSION_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Debug, Default)]
struct SessionState {
    templates: HashMap<String, Map<String, Value>>,
    state: HashMap<String, Map<String, Value>>,
    created: Option<Instant>,
    started_at: Option<String>,
}

impl SessionState {
    fn touch(&mut self) {
        if self.created.is_none() {
            self.created = Some(Instant::now());
        }
        if self.started_at.is_none() {
            self.started_at = Some(chrono::Utc::now().to_rfc3339());
        }
    }
}

static SESSIONS: LazyLock<Mutex<HashMap<String, SessionState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn with_sessions<T>(f: impl FnOnce(&mut HashMap<String, SessionState>) -> T) -> T {
    let mut guard = SESSIONS.lock().expect("extension session lock poisoned");
    let now = Instant::now();
    guard.retain(|_, session| {
        session
            .created
            .is_none_or(|created| now.duration_since(created) < SESSION_TTL)
    });
    f(&mut guard)
}

pub fn ensure_session(session_id: &str) {
    with_sessions(|sessions| {
        let session = sessions.entry(session_id.to_string()).or_default();
        session.touch();
    });
}

pub fn remove_session(session_id: &str) {
    with_sessions(|sessions| {
        sessions.remove(session_id);
    });
}

pub fn session_started_at(session_id: &str) -> String {
    with_sessions(|sessions| {
        sessions
            .get(session_id)
            .and_then(|session| session.started_at.clone())
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
    })
}

pub fn template_values(session_id: &str) -> Map<String, Value> {
    with_sessions(|sessions| {
        let Some(session) = sessions.get(session_id) else {
            return Map::new();
        };
        let mut result = Map::new();
        for (extension_id, values) in &session.templates {
            if values.is_empty() {
                continue;
            }
            result.insert(extension_id.clone(), Value::Object(values.clone()));
        }
        result
    })
}

pub fn set_template_value(
    session_id: &str,
    extension_id: &str,
    key: &str,
    value: Value,
) -> Result<(), String> {
    if !valid_key(key) {
        return Err(format!(
            "template key '{}' must match ^[a-z][a-z0-9_]{{0,63}}$",
            key
        ));
    }
    validate_value(&value)?;
    let size = serde_json::to_vec(&value)
        .map_err(|e| format!("failed to serialize template value: {}", e))?
        .len();
    if size > MAX_TEMPLATE_VALUE_BYTES {
        return Err(format!(
            "template value for '{}' is {} bytes, maximum is {}",
            key, size, MAX_TEMPLATE_VALUE_BYTES
        ));
    }

    with_sessions(|sessions| {
        let session = sessions.entry(session_id.to_string()).or_default();
        session.touch();
        let values = session
            .templates
            .entry(extension_id.to_string())
            .or_default();

        if !values.contains_key(key) && values.len() >= MAX_TEMPLATE_KEYS {
            return Err(format!(
                "extension already wrote {} template values, maximum is {}",
                values.len(),
                MAX_TEMPLATE_KEYS
            ));
        }

        let total: usize = values
            .iter()
            .filter(|(existing_key, _)| existing_key.as_str() != key)
            .map(|(_, existing)| serde_json::to_vec(existing).map(|b| b.len()).unwrap_or(0))
            .sum();

        if total + size > MAX_TEMPLATE_TOTAL_BYTES {
            return Err(format!(
                "extension template values exceed {} bytes",
                MAX_TEMPLATE_TOTAL_BYTES
            ));
        }

        values.insert(key.to_string(), value);
        Ok(())
    })
}

pub fn state_get(session_id: &str, extension_id: &str, key: &str) -> Option<Value> {
    with_sessions(|sessions| {
        sessions
            .get(session_id)
            .and_then(|session| session.state.get(extension_id))
            .and_then(|values| values.get(key))
            .cloned()
    })
}

pub fn state_set(
    session_id: &str,
    extension_id: &str,
    key: &str,
    value: Value,
) -> Result<(), String> {
    if !valid_key(key) {
        return Err(format!(
            "state key '{}' must match ^[a-z][a-z0-9_]{{0,63}}$",
            key
        ));
    }
    validate_value(&value)?;
    let size = serde_json::to_vec(&value)
        .map_err(|e| format!("failed to serialize state value: {}", e))?
        .len();
    if size > MAX_STATE_VALUE_BYTES {
        return Err(format!(
            "state value for '{}' is {} bytes, maximum is {}",
            key, size, MAX_STATE_VALUE_BYTES
        ));
    }

    with_sessions(|sessions| {
        let session = sessions.entry(session_id.to_string()).or_default();
        session.touch();
        let values = session.state.entry(extension_id.to_string()).or_default();

        let total: usize = values
            .iter()
            .filter(|(existing_key, _)| existing_key.as_str() != key)
            .map(|(_, existing)| serde_json::to_vec(existing).map(|b| b.len()).unwrap_or(0))
            .sum();
        if total + size > MAX_STATE_TOTAL_BYTES {
            return Err(format!(
                "extension state exceeds {} bytes",
                MAX_STATE_TOTAL_BYTES
            ));
        }

        values.insert(key.to_string(), value);
        Ok(())
    })
}

pub fn validate_value(value: &Value) -> Result<(), String> {
    fn walk(value: &Value, depth: usize) -> Result<(), String> {
        if depth > MAX_VALUE_DEPTH {
            return Err(format!("value nesting exceeds {} levels", MAX_VALUE_DEPTH));
        }
        match value {
            Value::Null | Value::Bool(_) | Value::Number(_) => Ok(()),
            Value::String(text) => {
                if text.len() > MAX_TEMPLATE_VALUE_BYTES {
                    return Err("string value is too large".to_string());
                }
                Ok(())
            }
            Value::Array(items) => {
                if items.len() > 256 {
                    return Err("array has too many items".to_string());
                }
                for item in items {
                    walk(item, depth + 1)?;
                }
                Ok(())
            }
            Value::Object(map) => {
                if map.len() > 256 {
                    return Err("object has too many keys".to_string());
                }
                for item in map.values() {
                    walk(item, depth + 1)?;
                }
                Ok(())
            }
        }
    }
    walk(value, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn session(name: &str) -> String {
        format!("test-{}-{}", name, uuid::Uuid::new_v4())
    }

    #[test]
    fn template_values_are_namespaced_per_extension() {
        let id = session("namespaced");
        ensure_session(&id);
        set_template_value(&id, "one", "value", json!(1)).unwrap();
        set_template_value(&id, "two", "value", json!("two")).unwrap();

        let values = template_values(&id);
        assert_eq!(values["one"]["value"], json!(1));
        assert_eq!(values["two"]["value"], json!("two"));
        remove_session(&id);
    }

    #[test]
    fn template_key_validation_rejects_bad_names() {
        let id = session("keys");
        ensure_session(&id);
        assert!(set_template_value(&id, "ext", "Bad-Key", json!(1)).is_err());
        assert!(set_template_value(&id, "ext", "good_key", json!(1)).is_ok());
        remove_session(&id);
    }

    #[test]
    fn nested_values_beyond_depth_limit_are_rejected() {
        let id = session("depth");
        ensure_session(&id);
        let value = json!({"a": {"b": {"c": {"d": {"e": 1}}}}});
        assert!(set_template_value(&id, "ext", "deep", value).is_err());
        remove_session(&id);
    }

    #[test]
    fn state_round_trips_within_a_session() {
        let id = session("state");
        ensure_session(&id);
        state_set(&id, "ext", "cursor", json!({"index": 4})).unwrap();
        assert_eq!(state_get(&id, "ext", "cursor"), Some(json!({"index": 4})));
        remove_session(&id);
        assert_eq!(state_get(&id, "ext", "cursor"), None);
    }

    #[test]
    fn removing_a_session_drops_templates() {
        let id = session("remove");
        ensure_session(&id);
        set_template_value(&id, "ext", "value", json!("x")).unwrap();
        remove_session(&id);
        assert!(template_values(&id).is_empty());
    }
}
