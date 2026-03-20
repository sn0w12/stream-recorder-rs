use crate::config::Config;
use anyhow::Result;
use handlebars::{Handlebars, handlebars_helper};
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug)]
pub enum TemplateValue {
    String(String),
    Array(Vec<String>),
}

fn to_json_value(tv: &TemplateValue) -> Value {
    match tv {
        TemplateValue::String(s) => Value::String(s.clone()),
        TemplateValue::Array(arr) => {
            Value::Array(arr.iter().map(|s| Value::String(s.clone())).collect())
        }
    }
}

fn resolve_template_string(config: &Config, config_path: &Path) -> Result<Option<String>> {
    if let Some(template_str) = config.get_upload_complete_message_template() {
        return Ok(Some(template_str.to_owned()));
    }

    if config_path.exists() {
        return Ok(Some(std::fs::read_to_string(config_path)?));
    }

    println!("No template found in config or template.hbr, no template will be used.");
    Ok(None)
}

pub fn get_template_string() -> Result<Option<String>> {
    let config = Config::load()?;
    let config_path = crate::utils::app_config_dir().join("template.hbr");
    resolve_template_string(&config, &config_path)
}

/// Render the provided template using Handlebars.
///
/// The function accepts the existing `TemplateValue` context type and converts it
/// into a `serde_json::Value` map, adding a `<key>_len` numeric property for arrays
/// (to preserve previous _len behavior). Errors during rendering are printed to
/// stderr and an empty string is returned on failure.
pub fn render_template(template: &str, context: &HashMap<String, TemplateValue>) -> String {
    let mut map = Map::new();

    for (k, v) in context.iter() {
        map.insert(k.clone(), to_json_value(v));
        if let TemplateValue::Array(arr) = v {
            map.insert(
                format!("{}_len", k),
                Value::Number(Number::from(arr.len() as u64)),
            );
        }
    }

    let data = Value::Object(map);

    // register simple helpers used in our templates (add, gt, ne)
    handlebars_helper!(add: |a: i64, b: i64| { a + b });
    handlebars_helper!(gt: |a: i64, b: i64| { a > b });
    handlebars_helper!(ne: |a: i64, b: i64| { a != b });

    let mut reg = Handlebars::new();
    reg.register_helper("add", Box::new(add));
    reg.register_helper("gt", Box::new(gt));
    reg.register_helper("ne", Box::new(ne));

    match reg.render_template(template, &data) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("handlebars render error: {}", e);
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolve_template_string_prefers_config_value() {
        let config = Config {
            upload_complete_message_template: Some("from-config".to_string()),
            ..Config::default()
        };
        let dir = tempdir().expect("tempdir");
        let template_path = dir.path().join("template.hbr");
        std::fs::write(&template_path, "from-file").expect("write template");

        let template = resolve_template_string(&config, &template_path).expect("resolve template");

        assert_eq!(template.as_deref(), Some("from-config"));
    }

    #[test]
    fn resolve_template_string_falls_back_to_file() {
        let config = Config::default();
        let dir = tempdir().expect("tempdir");
        let template_path = dir.path().join("template.hbr");
        std::fs::write(&template_path, "from-file").expect("write template");

        let template = resolve_template_string(&config, &template_path).expect("resolve template");

        assert_eq!(template.as_deref(), Some("from-file"));
    }

    #[test]
    fn resolve_template_string_returns_none_when_missing() {
        let config = Config::default();
        let dir = tempdir().expect("tempdir");
        let template_path = dir.path().join("missing.hbr");

        let template = resolve_template_string(&config, &template_path).expect("resolve template");

        assert!(template.is_none());
    }
}
