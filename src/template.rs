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

/// Returns `true` when the trimmed line contains a complete single‑line
/// `{{#if}}…{{/if}}`, `{{#each}}…{{/each}}` or `{{#unless}}…{{/unless}}` block.
fn is_standalone_block(line: &str) -> bool {
    let line = line.trim();
    let has_opener =
        line.starts_with("{{#if") || line.starts_with("{{#each") || line.starts_with("{{#unless");
    let has_closer =
        line.contains("{{/if}}") || line.contains("{{/each}}") || line.contains("{{/unless}}");
    has_opener && has_closer
}

/// Render the provided template using Handlebars.
///
/// Lines that are standalone `{{#if}}` / `{{#each}}` / `{{#unless}}` blocks are
/// rendered individually; if they produce empty output the line is omitted.
/// All other lines are always kept, even when they render to nothing (e.g. a
/// line containing only `{{date}}` when `date` is empty).
///
/// If the template contains multi‑line blocks (which cannot be rendered
/// line‑by‑line) the function falls back to rendering the entire template at
/// once, which preserves the original behaviour.
///
/// Errors during rendering are printed to stderr and an empty string is
/// returned on failure.
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

    // Try line‑by‑line rendering so standalone conditional/loop blocks that
    // produce nothing can be suppressed.  Lines that are NOT standalone blocks
    // are always kept – even when their output is empty.
    let mut lines = Vec::new();
    for line in template.lines() {
        match reg.render_template(line, &data) {
            Ok(s) => {
                if is_standalone_block(line) && s.trim().is_empty() {
                    continue;
                }
                lines.push(s);
            }
            Err(e) => {
                // Multi‑line block – line‑by‑line won't work.  Fall back to
                // full‑template rendering and stop collecting.
                eprintln!("handlebars render error: {}", e);
                return match reg.render_template(template, &data) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("handlebars render error: {}", e);
                        String::new()
                    }
                };
            }
        }
    }

    lines.join("\n")
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

    #[test]
    fn whitespace_only_line_is_preserved() {
        let mut context = HashMap::new();
        context.insert(
            "title".to_string(),
            TemplateValue::String("hello".to_string()),
        );

        let template = "line1\n   \n{{title}}";
        let rendered = render_template(template, &context);

        assert_eq!(rendered, "line1\n   \nhello");
    }

    #[test]
    fn empty_variable_line_is_preserved() {
        let context = HashMap::new();

        let template = "before\n{{maybe}}\nafter";
        let rendered = render_template(template, &context);

        assert_eq!(rendered, "before\n\nafter");
    }

    #[test]
    fn false_conditional_line_is_skipped() {
        let mut context = HashMap::new();
        context.insert(
            "fileditch_urls".to_string(),
            TemplateValue::Array(vec!["https://example.com/fd.mp4".to_string()]),
        );

        let template = "\
[HR][/HR]
{{#if bunkr_urls}}[CENTER]bunkr[/CENTER]{{/if}}
{{#if fileditch_urls}}[CENTER]fileditch[/CENTER]{{/if}}";

        let rendered = render_template(template, &context);

        assert_eq!(rendered, "[HR][/HR]\n[CENTER]fileditch[/CENTER]");
    }

    #[test]
    fn true_conditional_line_is_preserved() {
        let mut context = HashMap::new();
        context.insert(
            "urls".to_string(),
            TemplateValue::Array(vec!["https://example.com".to_string()]),
        );

        let template = "\
before
{{#if urls}}[CENTER]content[/CENTER]{{/if}}
after";

        let rendered = render_template(template, &context);

        assert_eq!(rendered, "before\n[CENTER]content[/CENTER]\nafter");
    }

    #[test]
    fn consecutive_true_conditionals_preserve_separator() {
        let mut context = HashMap::new();
        context.insert("a".to_string(), TemplateValue::Array(vec!["x".to_string()]));
        context.insert("b".to_string(), TemplateValue::Array(vec!["y".to_string()]));

        let template = "\
{{#if a}}[CENTER]a[/CENTER]{{/if}}
{{#if b}}[CENTER]b[/CENTER]{{/if}}";

        let rendered = render_template(template, &context);

        assert_eq!(rendered, "[CENTER]a[/CENTER]\n[CENTER]b[/CENTER]");
    }

    #[test]
    fn multi_line_block_falls_back_to_full_render() {
        let mut context = HashMap::new();
        context.insert("cond".to_string(), TemplateValue::String("yes".to_string()));

        let template = "start\n{{#if cond}}\nbody\n{{/if}}\nend";
        let rendered = render_template(template, &context);

        assert_eq!(rendered, "start\nbody\nend");
    }

    #[test]
    fn both_false_and_true_conditionals_in_one_template() {
        let mut context = HashMap::new();
        context.insert("a".to_string(), TemplateValue::Array(vec!["x".to_string()]));

        let template = "\
header
{{#if missing}}[CENTER]missing[/CENTER]{{/if}}
{{#if a}}[CENTER]present[/CENTER]{{/if}}
footer";

        let rendered = render_template(template, &context);

        assert_eq!(rendered, "header\n[CENTER]present[/CENTER]\nfooter");
    }
}
