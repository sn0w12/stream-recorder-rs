use std::collections::HashMap;
use std::cmp::Ordering;

#[derive(Clone)]
pub enum TemplateValue {
    String(String),
    Array(Vec<String>),
}

fn replace_condition_variables(condition: &str, context: &HashMap<String, TemplateValue>) -> String {
    let simple_re = regex::Regex::new(r"\{\{(\w+)\}\}").unwrap();
    simple_re
        .replace_all(condition, |caps: &regex::Captures| {
            let key = &caps[1];
            if let Some(TemplateValue::String(value)) = context.get(key) {
                value.clone()
            } else {
                "".to_string()
            }
        })
        .to_string()
}

fn compare_by_order(left: &str, right: &str, expected: Ordering) -> bool {
    if let (Ok(left_num), Ok(right_num)) = (left.parse::<f64>(), right.parse::<f64>()) {
        return left_num.partial_cmp(&right_num) == Some(expected);
    }

    left.cmp(right) == expected
}

fn resolve_comparison_operand(token: &str, context: &HashMap<String, TemplateValue>) -> String {
    let trimmed = token.trim();

    if let Some(TemplateValue::String(value)) = context.get(trimmed) {
        return value.clone();
    }

    if let Some(TemplateValue::Array(values)) = context.get(trimmed) {
        return values.len().to_string();
    }

    trimmed.to_string()
}

fn evaluate_comparison(condition: &str, context: &HashMap<String, TemplateValue>) -> Option<bool> {
    for operator in ["==", "!=", ">", "<"] {
        if let Some((left, right)) = condition.split_once(operator) {
            let left = resolve_comparison_operand(left, context);
            let right = resolve_comparison_operand(right, context);

            let result = match operator {
                "==" => left == right,
                "!=" => left != right,
                ">" => compare_by_order(&left, &right, Ordering::Greater),
                "<" => compare_by_order(&left, &right, Ordering::Less),
                _ => false,
            };

            return Some(result);
        }
    }

    None
}

fn evaluate_condition(condition: &str, context: &HashMap<String, TemplateValue>) -> bool {
    let processed_condition = replace_condition_variables(condition, context);

    if let Some(result) = evaluate_comparison(&processed_condition, context) {
        return result;
    }

    let key = processed_condition.trim();
    match context.get(key) {
        Some(TemplateValue::String(value)) if !value.is_empty() => true,
        Some(TemplateValue::Array(values)) if !values.is_empty() => true,
        _ => false,
    }
}

/// Renders a template string by replacing placeholders with values from the context.
///
/// The template supports several features:
/// - Simple variable replacement: `{{key}}` is replaced with the string value of `key`
/// - Conditional rendering: `{{if condition: content}}` renders `content` only if the condition evaluates to true
///   - Simple conditions: `key` (true if key exists and is not empty)
///   - Comparison conditions: `{{var}} == value` or `value == {{var}}` (true if equal after replacement)
///   - Inequality conditions: `{{var}} != value` or `value != {{var}}` (true if not equal after replacement)
/// - Loops: `{{for key: template}}` iterates over arrays, replacing `{{item}}` with each item and `{{i}}` with the 1-based index
///
/// Processing order: for loops → if statements → simple replacements
///
/// # Arguments
/// * `template` - The template string containing placeholders
/// * `context` - A map of variable names to their values (strings or arrays of strings)
///
/// # Returns
/// The rendered template string with all placeholders replaced
///
/// # Examples
/// ```
/// use std::collections::HashMap;
/// use template::{render_template, TemplateValue};
///
/// let mut context = HashMap::new();
/// context.insert("name".to_string(), TemplateValue::String("Alice".to_string()));
/// context.insert("urls".to_string(), TemplateValue::Array(vec!["url1".to_string(), "url2".to_string()]));
///
/// let template = "Hello {{name}}! {{if urls: URLs: {{for urls: - {{item}} {{if {{i}} != 2: (not last)\n}}}}}}";
/// let result = render_template(template, &context);
/// // Result: "Hello Alice! URLs: - url1 (not last)\n- url2 \n"
/// ```
pub fn render_template(template: &str, context: &HashMap<String, TemplateValue>) -> String {
    let mut extended_context = context.clone();

    // Automatically add _len for arrays
    for (key, value) in &*context {
        if let TemplateValue::Array(arr) = value {
            let len_key = format!("{}_len", key);
            extended_context.insert(len_key, TemplateValue::String(arr.len().to_string()));
        }
    }

    let mut result = template.to_string();

    // Handle for loops first
    while let Some(start) = result.find("{{for ") {
        let after_for = &result[start + 6..];
        if let Some(colon_pos) = after_for.find(':') {
            let key = after_for[..colon_pos].trim();
            let after_colon = &after_for[colon_pos + 1..];
            // Find the matching closing '}}' for this for-loop block, accounting for nested braces
            let mut brace_count = 0;
            let mut end_pos = None;
            let chars: Vec<_> = after_colon.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
                    brace_count += 1;
                    i += 2;
                    continue;
                }
                if i + 1 < chars.len() && chars[i] == '}' && chars[i + 1] == '}' {
                    if brace_count == 0 {
                        end_pos = Some(i);
                        break;
                    } else {
                        brace_count -= 1;
                    }
                    i += 2;
                    continue;
                }
                i += 1;
            }
            if let Some(end_pos) = end_pos {
                let inner_template: String = chars[..end_pos].iter().collect();
                let full_match_end = start + 6 + colon_pos + 1 + end_pos + 2;
                let full_match = &result[start..full_match_end];
                if let Some(TemplateValue::Array(arr)) = extended_context.get(key) {
                    if arr.is_empty() {
                        result = result.replacen(full_match, "", 1);
                    } else {
                        let mut parts = vec![];
                        for (i, item) in arr.iter().enumerate() {
                            let mut inner = inner_template.clone();
                            inner = inner.replace("{{item}}", item);
                            inner = inner.replace("{{i}}", &(i + 1).to_string());
                            // Render nested if statements inside the for block
                            let if_re = regex::Regex::new(r"\{\{if\s+(.*?):\s*(.*?)\}\}").unwrap();
                            while let Some(captures) = if_re.captures(&inner) {
                                let condition = &captures[1];
                                let inner_content = &captures[2];
                                let full_match = captures.get(0).unwrap().as_str();
                                let should_render = evaluate_condition(condition, &extended_context);
                                let replacement = if should_render { inner_content.to_string() } else { "".to_string() };
                                inner = inner.replacen(full_match, &replacement, 1);
                            }
                            // Simple replacements in inner
                            let simple_re = regex::Regex::new(r"\{\{(\w+)\}\}").unwrap();
                            inner = simple_re.replace_all(&inner, |caps: &regex::Captures| {
                                let key_inner = &caps[1];
                                if let Some(TemplateValue::String(s)) = extended_context.get(key_inner) {
                                    s.clone()
                                } else {
                                    "".to_string()
                                }
                            }).to_string();
                            parts.push(inner.trim().to_string());
                        }
                        let replacement = parts.join("");
                        result = result.replacen(full_match, &replacement, 1);
                    }
                } else {
                    result = result.replacen(full_match, "", 1);
                }
            } else {
                // No closing '}}' found, remove the block safely
                let broken_start = start;
                let broken_end = result[broken_start..].find("}}")
                    .map(|e| broken_start + e + 2)
                    .unwrap_or(result.len());
                result.replace_range(broken_start..broken_end, "");
            }
        } else {
            break;
        }
    }

    // Handle if statements next
    let if_re = regex::Regex::new(r"\{\{if\s+(.*?):\s*(.*?)\}\}").unwrap();
    while let Some(captures) = if_re.captures(&result) {
        let condition = &captures[1];
        let inner_content = &captures[2];
        let full_match = captures.get(0).unwrap().as_str();
        let should_render = evaluate_condition(condition, &extended_context);

        let replacement = if should_render { inner_content.to_string() } else { "".to_string() };
        result = result.replace(full_match, &replacement);
    }

    // Handle simple replacements last
    let simple_re = regex::Regex::new(r"\{\{(\w+)\}\}").unwrap();
    result = simple_re.replace_all(&result, |caps: &regex::Captures| {
        let key = &caps[1];
        if let Some(TemplateValue::String(s)) = extended_context.get(key) {
            s.clone()
        } else {
            "".to_string()
        }
    }).to_string();

    result
}