use std::marker::PhantomData;
use std::time::Duration as StdDuration;

use apto::{
    ConfigType, ConfigValidator, NoValidation, Optional, Required,
    helpers::{normalize_optional_value, parse_optional_value},
};

use crate::types::DurationValue;

fn parse_duration_cli(input: &str) -> Result<DurationValue, apto::ConfigError> {
    crate::types::parse_duration_input(input).map_err(|e| {
        apto::ConfigError::InvalidValue(format!("Invalid duration '{}': {}", input, e))
    })
}

/// Duration config value (required — always resolves to a `StdDuration`).
pub struct Duration<V = NoValidation, O = Required>(PhantomData<(V, O)>);

impl<V: ConfigValidator<Option<DurationValue>>> ConfigType for Duration<V, Required> {
    type Stored = Option<DurationValue>;
    type Default = DurationValue;
    type Value = StdDuration;

    fn get(stored: &Self::Stored, default: &Self::Default) -> Self::Value {
        stored
            .map(|v| v.as_duration())
            .unwrap_or_else(|| default.as_duration())
    }

    fn parse(input: &str, default: &Self::Default) -> Result<Self::Stored, apto::ConfigError> {
        let is_none = input.eq_ignore_ascii_case("none");
        if is_none {
            return Err(apto::ConfigError::InvalidValue(
                "'none' is not valid for a required duration field".to_string(),
            ));
        }
        let parsed = parse_duration_cli(input).map(Some)?;
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

/// Duration config value (optional — may resolve to `None`).
pub type OptionalDuration<V = NoValidation> = Duration<V, Optional>;

impl<V: ConfigValidator<Option<DurationValue>>> ConfigType for Duration<V, Optional> {
    type Stored = Option<DurationValue>;
    type Default = Option<DurationValue>;
    type Value = Option<StdDuration>;

    fn get(stored: &Self::Stored, default: &Self::Default) -> Self::Value {
        stored
            .map(|v| v.as_duration())
            .or_else(|| default.map(DurationValue::into_duration))
    }

    fn parse(input: &str, default: &Self::Default) -> Result<Self::Stored, apto::ConfigError> {
        let parsed = parse_optional_value(input, parse_duration_cli)?;
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
    use crate::types::DurationValue;

    #[test]
    fn required_default() {
        let default = DurationValue::from_millis(500);
        assert_eq!(
            Duration::<NoValidation, Required>::get(&None, &default),
            StdDuration::from_millis(500)
        );
        assert_eq!(
            Duration::<NoValidation, Required>::get(&Some(DurationValue::from_secs(2)), &default),
            StdDuration::from_secs(2)
        );
    }

    #[test]
    fn required_rejects_none() {
        let default = DurationValue::from_millis(500);
        assert!(Duration::<NoValidation, Required>::parse("none", &default).is_err());
    }

    #[test]
    fn optional_accepts_none() {
        let default = Some(DurationValue::from_secs(90));
        assert_eq!(
            Duration::<NoValidation, Optional>::parse("none", &default).unwrap(),
            None
        );
        assert_eq!(
            Duration::<NoValidation, Optional>::get(&None, &default),
            Some(StdDuration::from_secs(90))
        );
    }
}
