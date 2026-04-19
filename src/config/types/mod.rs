pub mod duration;
pub mod file_size;
pub mod list;
pub mod number;
pub mod text;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::str::FromStr;

use crate::types::{DurationValue, FileSize as FileSizeValue};

pub use duration::{Duration, OptionalDuration};
pub use file_size::FileSize;
pub use list::StringList;
#[allow(unused_imports)]
pub use number::{Float, OptionalFloat, OptionalU32, U32};
pub use text::{OptionalText, Text};

/// A reusable validator for a config value.
///
/// Implement this for marker types that enforce additional constraints beyond
/// the base parsing behavior of a [`ConfigType`]. Validators are attached to a
/// config type as a type parameter so multiple rules can be composed without
/// duplicating parsing logic.
pub trait ConfigValidator<T> {
    /// Validate a parsed value.
    fn validate(value: &T) -> Result<()>;
}

/// Marker type for config values that do not need extra validation.
pub struct NoValidation;

impl<T> ConfigValidator<T> for NoValidation {
    fn validate(_: &T) -> Result<()> {
        Ok(())
    }
}

/// Compose two validators and require both of them to pass.
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

pub trait ConfigFormat<Default> {
    fn format(stored: &Self, default: &Default) -> String;
}

macro_rules! impl_option_config_format {
    ($value_ty:ty) => {
        impl ConfigFormat<$value_ty> for Option<$value_ty> {
            fn format(stored: &Self, default: &$value_ty) -> String {
                stored
                    .clone()
                    .unwrap_or_else(|| default.clone())
                    .to_string()
            }
        }

        impl ConfigFormat<Option<$value_ty>> for Option<$value_ty> {
            fn format(stored: &Self, default: &Option<$value_ty>) -> String {
                stored
                    .clone()
                    .or_else(|| default.clone())
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
            }
        }
    };
}

impl_option_config_format!(u32);
impl_option_config_format!(f64);
impl_option_config_format!(String);
impl_option_config_format!(DurationValue);
impl_option_config_format!(FileSizeValue);

/// Behavior shared by all config value families.
///
/// A `ConfigType` owns the conversion rules for a setting: how it is stored in
/// TOML, how it is parsed from the CLI, how it is displayed, and how the typed
/// getter should expose the effective value to the rest of the program.
pub trait ConfigType
where
    Self::Stored: ConfigFormat<Self::Default>,
{
    /// The exact serialized representation stored in `Config`.
    type Stored: Clone + Default + Serialize + for<'de> Deserialize<'de>;
    /// The effective default value used when the setting is absent.
    type Default;
    /// The value returned by the typed getter.
    type Value<'a>;

    /// Convert the stored value and default into the typed getter output.
    fn get<'a>(stored: &'a Self::Stored, default: &'a Self::Default) -> Self::Value<'a>;
    /// Parse a string into the stored representation.
    fn parse(input: &str, default: &Self::Default) -> Result<Self::Stored>;
    /// Format the stored value for display.
    fn format(stored: &Self::Stored, default: &Self::Default) -> String {
        <Self::Stored as ConfigFormat<Self::Default>>::format(stored, default)
    }
    /// Format the effective default for display in CLI/docs output.
    fn format_default(default: &Self::Default) -> String
    where
        Self::Stored: From<Self::Default>,
        Self::Default: Clone,
    {
        let default_stored = Self::Stored::from(default.clone());
        Self::format(&default_stored, default)
    }

    /// Validate the stored representation.
    fn validate(stored: &Self::Stored) -> Result<()>;

    /// Reset the stored value to its default/cleared form.
    ///
    /// Types that store `Option<_>` can usually rely on the default
    /// implementation, which returns `Self::Stored::default()`.
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
/// Parse a string value while treating `none` case-insensitively as empty.
pub fn parse_optional_text(input: &str) -> Option<String> {
    if input.eq_ignore_ascii_case("none") {
        None
    } else {
        Some(input.to_string())
    }
}

/// Parse an optional CLI string into an optional typed value.
///
/// The string `none` clears the setting; otherwise `parse_value` is used to
/// convert the input into the underlying type.
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

/// Collapse an explicit value that matches the default back to `None`.
pub fn normalize_optional_value<T>(parsed: Option<T>, default: Option<T>) -> Option<T>
where
    T: PartialEq,
{
    if parsed == default { None } else { parsed }
}

/// Collapse a parsed string back to `None` when it matches the default text.
pub fn normalize_text_value(parsed: Option<String>, default: &str) -> Option<String> {
    match parsed.as_deref() {
        Some(value) if value == default => None,
        _ => parsed,
    }
}

/// Collapse a parsed string list back to `None` when it matches the default list.
pub fn normalize_list_value(
    parsed: Option<Vec<String>>,
    default: &[String],
) -> Option<Vec<String>> {
    match parsed.as_ref() {
        Some(values) if values == default => None,
        _ => parsed,
    }
}

/// Parse a numeric value from a CLI string.
pub fn parse_number<T>(input: &str) -> Result<T>
where
    T: FromStr,
{
    input
        .parse::<T>()
        .map_err(|_| anyhow::anyhow!("Invalid number"))
}

/// Parse a comma-separated list from a CLI string.
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
