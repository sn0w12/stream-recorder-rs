use crate::config::Config;
use anyhow::Result;
use handlebars::{Handlebars, handlebars_helper};
use serde_json::{Map, Number, Value};
use std::collections::HashMap;

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

pub fn get_template_string() -> Result<Option<String>> {
    let config = Config::get();
    let config_path = crate::utils::app_config_dir().join("template.hbr");

    if let Some(template_str) = config.get_upload_complete_message_template() {
        return Ok(Some(template_str.to_owned()));
    }

    if config_path.exists() {
        return Ok(Some(std::fs::read_to_string(config_path)?));
    }

    println!("No template found in config or template.hbr, no template will be used.");
    Ok(None)
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
    reg.register_escape_fn(handlebars::no_escape);
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

    #[test]
    fn render_template_preserves_query_string_urls() {
        let mut context = HashMap::new();
        context.insert(
            "fileditch_urls".to_string(),
            TemplateValue::Array(vec![
                "https://fileditchfiles.me/file.php?f=/alpha0/d01f7bc095616c434c08/video.mp4"
                    .to_string(),
            ]),
        );

        let rendered = render_template("{{#each fileditch_urls}}{{this}}{{/each}}", &context);

        assert_eq!(
            rendered,
            "https://fileditchfiles.me/file.php?f=/alpha0/d01f7bc095616c434c08/video.mp4"
        );
    }
}
