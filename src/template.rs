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

    // register the small set of helpers used in templates
    handlebars_helper!(add: |a: i64, b: i64| { a + b });
    handlebars_helper!(sub: |a: i64, b: i64| { a - b });
    handlebars_helper!(gt: |a: i64, b: i64| { a > b });
    handlebars_helper!(lt: |a: i64, b: i64| { a < b });
    handlebars_helper!(ne: |a: i64, b: i64| { a != b });
    handlebars_helper!(eq: |a: i64, b: i64| { a == b });
    handlebars_helper!(lower: |s: str| { s.to_lowercase() });
    handlebars_helper!(upper: |s: str| { s.to_uppercase() });

    let mut reg = Handlebars::new();
    reg.register_escape_fn(handlebars::no_escape);
    reg.register_helper("add", Box::new(add));
    reg.register_helper("gt", Box::new(gt));
    reg.register_helper("lt", Box::new(lt));
    reg.register_helper("ne", Box::new(ne));
    reg.register_helper("eq", Box::new(eq));
    reg.register_helper("lower", Box::new(lower));
    reg.register_helper("upper", Box::new(upper));

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

    #[test]
    fn render_template_supports_basic_comparisons_and_string_helpers() {
        let mut context = HashMap::new();
        context.insert(
            "username".to_string(),
            TemplateValue::String("MiXeDCaseUser".to_string()),
        );
        context.insert(
            "stream_title".to_string(),
            TemplateValue::String("Live Session".to_string()),
        );
        context.insert(
            "upload_urls".to_string(),
            TemplateValue::Array(vec![
                "https://example.com/one".to_string(),
                "https://example.com/two".to_string(),
                "https://example.com/three".to_string(),
            ]),
        );

        let rendered = render_template(
            "{{#if (eq upload_urls_len 3)}}{{lower username}}|{{upper stream_title}}{{/if}}",
            &context,
        );

        assert_eq!(rendered, "mixedcaseuser|LIVE SESSION");
    }
}
