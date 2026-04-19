use crate::config::types::{
    ConfigType, ConfigValidator, NoValidation, normalize_optional_value, parse_optional_value,
};
use crate::types::DurationValue;
use anyhow::Result;
use std::marker::PhantomData;
use std::time::Duration as StdDuration;

fn parse_duration_cli(input: &str) -> Result<DurationValue> {
    crate::types::parse_duration_input(input)
        .map_err(|error| anyhow::anyhow!("Invalid duration '{}': {}", input, error))
}

pub struct Duration<V = NoValidation>(PhantomData<V>);

impl<V> ConfigType for Duration<V>
where
    V: ConfigValidator<Option<DurationValue>>,
{
    type Stored = Option<DurationValue>;
    type Default = DurationValue;
    type Value<'a> = StdDuration;

    fn get<'a>(stored: &'a Self::Stored, default: &'a Self::Default) -> Self::Value<'a> {
        stored
            .map(|value| value.as_duration())
            .unwrap_or_else(|| default.as_duration())
    }

    fn parse_cli(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_optional_value(
            parse_optional_value(input, parse_duration_cli)?,
            Some(*default),
        ))
    }

    fn format_cli(stored: &Self::Stored, default: &Self::Default) -> String {
        stored.unwrap_or(*default).to_string()
    }

    fn format_default(default: &Self::Default) -> String {
        default.to_string()
    }

    fn validate(stored: &Self::Stored) -> Result<()> {
        V::validate(stored)
    }
}

pub struct OptionalDuration<V = NoValidation>(PhantomData<V>);

impl<V> ConfigType for OptionalDuration<V>
where
    V: ConfigValidator<Option<DurationValue>>,
{
    type Stored = Option<DurationValue>;
    type Default = Option<DurationValue>;
    type Value<'a> = Option<StdDuration>;

    fn get<'a>(stored: &'a Self::Stored, default: &'a Self::Default) -> Self::Value<'a> {
        stored
            .map(|value| value.as_duration())
            .or_else(|| default.map(DurationValue::into_duration))
    }

    fn parse_cli(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_optional_value(
            parse_optional_value(input, parse_duration_cli)?,
            *default,
        ))
    }

    fn format_cli(stored: &Self::Stored, default: &Self::Default) -> String {
        stored
            .or(*default)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string())
    }

    fn format_default(default: &Self::Default) -> String {
        default
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string())
    }

    fn validate(stored: &Self::Stored) -> Result<()> {
        V::validate(stored)
    }
}
