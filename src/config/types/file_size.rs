use crate::config::types::{
    ConfigType, ConfigValidator, NoValidation, normalize_optional_value, parse_optional_value,
};
use crate::types::FileSize as FileSizeValue;
use anyhow::Result;
use std::marker::PhantomData;

fn parse_file_size_value(input: &str) -> Result<FileSizeValue> {
    FileSizeValue::from_str(input)
        .map_err(|error| anyhow::anyhow!("Invalid file size '{}': {}", input, error))
}

/// Config type for human-readable file-size strings such as `10MB` or `5GiB`.
///
/// The stored representation is the parsed [`FileSizeValue`], so typed getters
/// return the real size object and config defaults stay strongly typed.
#[allow(dead_code)]
pub struct FileSize<V = NoValidation>(PhantomData<V>);

impl<V> ConfigType for FileSize<V>
where
    V: ConfigValidator<Option<FileSizeValue>>,
{
    type Stored = Option<FileSizeValue>;
    type Default = FileSizeValue;
    type Value<'a> = FileSizeValue;

    fn get<'a>(stored: &'a Self::Stored, default: &'a Self::Default) -> Self::Value<'a> {
        stored.unwrap_or(*default)
    }

    fn parse_cli(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_optional_value(
            parse_optional_value(input, parse_file_size_value)?,
            Some(*default),
        ))
    }

    fn format_cli(stored: &Self::Stored, default: &Self::Default) -> String {
        stored
            .or(Some(*default))
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string())
    }

    fn format_default(default: &Self::Default) -> String {
        default.to_string()
    }

    fn validate(stored: &Self::Stored) -> Result<()> {
        V::validate(stored)
    }
}
