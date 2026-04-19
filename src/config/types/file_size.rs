use crate::config::types::{
    ConfigType, ConfigValidator, NoValidation, normalize_optional_value, parse_optional_value,
};
use crate::types::FileSize as FileSizeValue;
use anyhow::Result;
use std::marker::PhantomData;

fn parse_file_size_value(input: &str) -> Result<FileSizeValue> {
    input
        .parse::<FileSizeValue>()
        .map_err(|error| anyhow::anyhow!("Invalid file size '{}': {}", input, error))
}

/// File-size setting stored as bytes but configured through human-readable strings.
///
/// The stored representation is an optional [`FileSizeValue`], while CLI input
/// accepts strings such as `10MB`, `5GiB`, or `none` to clear the setting.
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

#[cfg(test)]
mod tests {
    use super::FileSize;
    use crate::config::types::{ConfigType, NoValidation};
    use crate::types::FileSize as FileSizeValue;

    type FileSizeType = FileSize<NoValidation>;

    #[test]
    fn file_size_type_parses_and_formats_cli_values() {
        let default = FileSizeValue::from_mib(2);

        assert_eq!(
            FileSizeType::parse_cli("5MiB", &default).expect("valid file size should parse"),
            Some(FileSizeValue::from_mib(5))
        );
        assert_eq!(
            FileSizeType::parse_cli("2MiB", &default).expect("default file size should normalize"),
            None
        );
        assert_eq!(
            FileSizeType::format_cli(&Some(FileSizeValue::from_mib(5)), &default),
            "5MiB"
        );
        assert_eq!(FileSizeType::format_default(&default), "2MiB");
        assert_eq!(
            FileSizeType::get(&Some(FileSizeValue::from_mib(5)), &default),
            FileSizeValue::from_mib(5)
        );
        assert_eq!(FileSizeType::get(&None, &default), default);
    }
}
