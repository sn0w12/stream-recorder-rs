use std::marker::PhantomData;

use apto::{
    ConfigType, ConfigValidator, NoValidation, Optional, Required,
    helpers::{normalize_optional_value, parse_optional_value},
};

use crate::types::FileSize as FileSizeValue;

fn parse_file_size_cli(input: &str) -> Result<FileSizeValue, apto::ConfigError> {
    input.parse::<FileSizeValue>().map_err(|e| {
        apto::ConfigError::InvalidValue(format!("Invalid file size '{}': {}", input, e))
    })
}

/// File-size config value (required).
pub struct FileSize<V = NoValidation, O = Required>(PhantomData<(V, O)>);

impl<V: ConfigValidator<Option<FileSizeValue>>> ConfigType for FileSize<V, Required> {
    type Stored = Option<FileSizeValue>;
    type Default = FileSizeValue;
    type Value = FileSizeValue;

    fn get(stored: &Self::Stored, default: &Self::Default) -> Self::Value {
        stored.unwrap_or(*default)
    }

    fn parse(input: &str, default: &Self::Default) -> Result<Self::Stored, apto::ConfigError> {
        let is_none = input.eq_ignore_ascii_case("none");
        if is_none {
            return Err(apto::ConfigError::InvalidValue(
                "'none' is not valid for a required file size field".to_string(),
            ));
        }
        let parsed = parse_file_size_cli(input).map(Some)?;
        Ok(normalize_optional_value(parsed, Some(*default)))
    }

    fn format(stored: &Self::Stored, default: &Self::Default) -> String {
        stored.unwrap_or(*default).to_string()
    }

    fn stored_from_default(default: Self::Default) -> Self::Stored {
        Some(default)
    }

    fn validate(stored: &Self::Stored) -> Result<(), apto::ConfigError> {
        V::validate(stored)
    }
}

impl<V: ConfigValidator<Option<FileSizeValue>>> ConfigType for FileSize<V, Optional> {
    type Stored = Option<FileSizeValue>;
    type Default = Option<FileSizeValue>;
    type Value = Option<FileSizeValue>;

    fn get(stored: &Self::Stored, default: &Self::Default) -> Self::Value {
        (*stored).or(*default)
    }

    fn parse(input: &str, default: &Self::Default) -> Result<Self::Stored, apto::ConfigError> {
        let parsed = parse_optional_value(input, parse_file_size_cli)?;
        Ok(normalize_optional_value(parsed, *default))
    }

    fn format(stored: &Self::Stored, default: &Self::Default) -> String {
        stored
            .or(*default)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string())
    }

    fn stored_from_default(default: Self::Default) -> Self::Stored {
        default
    }

    fn validate(stored: &Self::Stored) -> Result<(), apto::ConfigError> {
        V::validate(stored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileSize as FileSizeValue;

    #[test]
    fn required_default() {
        let default = FileSizeValue::from_mib(2);
        assert_eq!(
            FileSize::<NoValidation, Required>::get(&None, &default),
            default
        );
        assert_eq!(
            FileSize::<NoValidation, Required>::get(&Some(FileSizeValue::from_mib(5)), &default),
            FileSizeValue::from_mib(5)
        );
    }
}
