pub mod file_size;
pub mod list;
pub mod number;
pub mod text;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::str::FromStr;

#[allow(unused_imports)]
pub use file_size::FileSize;
pub use list::StringList;
pub use number::{Float, OptionalFloat, OptionalU32, U32};
pub use text::{OptionalText, Text};

pub trait ConfigValidator<T> {
    fn validate(value: &T) -> Result<()>;
}

pub struct NoValidation;

impl<T> ConfigValidator<T> for NoValidation {
    fn validate(_: &T) -> Result<()> {
        Ok(())
    }
}

#[allow(dead_code)]
pub struct All<A, B>(PhantomData<(A, B)>);

impl<T, A, B> ConfigValidator<T> for All<A, B>
where
    A: ConfigValidator<T>,
    B: ConfigValidator<T>,
{
    fn validate(value: &T) -> Result<()> {
        A::validate(value)?;
        B::validate(value)
    }
}

pub trait ConfigType {
    type Stored: Clone + Default + Serialize + for<'de> Deserialize<'de>;
    type Default;
    type Value<'a>;

    fn get<'a>(stored: &'a Self::Stored, default: &'a Self::Default) -> Self::Value<'a>;
    fn parse_cli(input: &str, default: &Self::Default) -> Result<Self::Stored>;
    fn format_cli(stored: &Self::Stored, default: &Self::Default) -> String;
    fn format_default(default: &Self::Default) -> String;
    fn validate(stored: &Self::Stored) -> Result<()>;

    fn reset_value(default: &Self::Default) -> Self::Stored
    where
        Self::Stored: Default,
    {
        let _ = default;
        Self::Stored::default()
    }

    fn is_array() -> bool {
        false
    }
}

/// Parses a string as an optional value, treating "none" (case-insensitive) as `None`.
///
/// ```
/// use stream_recorder::config::types::parse_optional_text;
/// assert_eq!(parse_optional_text("none"), None);
/// assert_eq!(parse_optional_text("None"), None);
/// assert_eq!(parse_optional_text("NONE"), None);
/// assert_eq!(parse_optional_text("some value"), Some("some value".to_string()));
/// ```
pub fn parse_optional_text(input: &str) -> Option<String> {
    if input.eq_ignore_ascii_case("none") {
        None
    } else {
        Some(input.to_string())
    }
}

pub fn parse_optional_value<T, F>(input: &str, parse_value: F) -> Result<Option<T>>
where
    F: FnOnce(&str) -> Result<T>,
{
    if input.eq_ignore_ascii_case("none") {
        Ok(None)
    } else {
        parse_value(input).map(Some)
    }
}

pub fn normalize_optional_value<T>(parsed: Option<T>, default: Option<T>) -> Option<T>
where
    T: PartialEq,
{
    if parsed == default { None } else { parsed }
}

pub fn normalize_text_value(parsed: Option<String>, default: &str) -> Option<String> {
    match parsed.as_deref() {
        Some(value) if value == default => None,
        _ => parsed,
    }
}

pub fn normalize_list_value(
    parsed: Option<Vec<String>>,
    default: &[String],
) -> Option<Vec<String>> {
    match parsed.as_ref() {
        Some(values) if values == default => None,
        _ => parsed,
    }
}

pub fn parse_number<T>(input: &str) -> Result<T>
where
    T: FromStr,
{
    input
        .parse::<T>()
        .map_err(|_| anyhow::anyhow!("Invalid number"))
}

pub fn parse_csv_list(input: &str) -> Option<Vec<String>> {
    if input.eq_ignore_ascii_case("none") {
        None
    } else {
        Some(
            input
                .split(',')
                .map(|value| value.trim().to_string())
                .collect(),
        )
    }
}
