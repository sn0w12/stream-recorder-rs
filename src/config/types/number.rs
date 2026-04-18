use crate::config::types::{
    ConfigType, ConfigValidator, NoValidation, normalize_optional_value, parse_number,
    parse_optional_value,
};
use anyhow::Result;
use std::marker::PhantomData;

pub struct U32<V = NoValidation>(PhantomData<V>);

impl<V> ConfigType for U32<V>
where
    V: ConfigValidator<Option<u32>>,
{
    type Stored = Option<u32>;
    type Default = u32;
    type Value<'a> = u32;

    fn get<'a>(stored: &'a Self::Stored, default: &'a Self::Default) -> Self::Value<'a> {
        stored.unwrap_or(*default)
    }

    fn parse_cli(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_optional_value(
            parse_optional_value(input, parse_number::<u32>)?,
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

    fn reset_value(_: &Self::Default) -> Self::Stored {
        None
    }
}

pub struct OptionalU32<V = NoValidation>(PhantomData<V>);

impl<V> ConfigType for OptionalU32<V>
where
    V: ConfigValidator<Option<u32>>,
{
    type Stored = Option<u32>;
    type Default = Option<u32>;
    type Value<'a> = Option<u32>;

    fn get<'a>(stored: &'a Self::Stored, default: &'a Self::Default) -> Self::Value<'a> {
        (*stored).or(*default)
    }

    fn parse_cli(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_optional_value(
            parse_optional_value(input, |value| {
                value
                    .parse::<u32>()
                    .map_err(|_| anyhow::anyhow!("expected u32, got '{}'", value))
            })?,
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

    fn reset_value(_: &Self::Default) -> Self::Stored {
        None
    }
}

pub struct Float<V = NoValidation>(PhantomData<V>);

impl<V> ConfigType for Float<V>
where
    V: ConfigValidator<Option<f64>>,
{
    type Stored = Option<f64>;
    type Default = f64;
    type Value<'a> = f64;

    fn get<'a>(stored: &'a Self::Stored, default: &'a Self::Default) -> Self::Value<'a> {
        stored.unwrap_or(*default)
    }

    fn parse_cli(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_optional_value(
            parse_optional_value(input, parse_number::<f64>)?,
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

    fn reset_value(_: &Self::Default) -> Self::Stored {
        None
    }
}

pub struct OptionalFloat<V = NoValidation>(PhantomData<V>);

impl<V> ConfigType for OptionalFloat<V>
where
    V: ConfigValidator<Option<f64>>,
{
    type Stored = Option<f64>;
    type Default = Option<f64>;
    type Value<'a> = Option<f64>;

    fn get<'a>(stored: &'a Self::Stored, default: &'a Self::Default) -> Self::Value<'a> {
        (*stored).or(*default)
    }

    fn parse_cli(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_optional_value(
            parse_optional_value(input, parse_number::<f64>)?,
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

    fn reset_value(_: &Self::Default) -> Self::Stored {
        None
    }
}
