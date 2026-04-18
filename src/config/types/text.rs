use crate::config::types::{
    ConfigType, ConfigValidator, NoValidation, normalize_optional_value, normalize_text_value,
    parse_optional_text,
};
use anyhow::Result;
use std::marker::PhantomData;

pub struct Text<V = NoValidation>(PhantomData<V>);

impl<V> ConfigType for Text<V>
where
    V: ConfigValidator<Option<String>>,
{
    type Stored = Option<String>;
    type Default = String;
    type Value<'a> = String;

    fn get<'a>(stored: &'a Self::Stored, default: &'a Self::Default) -> Self::Value<'a> {
        stored.clone().unwrap_or_else(|| default.clone())
    }

    fn parse_cli(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_text_value(parse_optional_text(input), default))
    }

    fn format_cli(stored: &Self::Stored, default: &Self::Default) -> String {
        stored.clone().unwrap_or_else(|| default.clone())
    }

    fn format_default(default: &Self::Default) -> String {
        default.clone()
    }

    fn validate(stored: &Self::Stored) -> Result<()> {
        V::validate(stored)
    }

    fn reset_value(_: &Self::Default) -> Self::Stored {
        None
    }
}

pub struct OptionalText<V = NoValidation>(PhantomData<V>);

impl<V> ConfigType for OptionalText<V>
where
    V: ConfigValidator<Option<String>>,
{
    type Stored = Option<String>;
    type Default = Option<String>;
    type Value<'a> = Option<&'a str>;

    fn get<'a>(stored: &'a Self::Stored, default: &'a Self::Default) -> Self::Value<'a> {
        stored.as_deref().or(default.as_deref())
    }

    fn parse_cli(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_optional_value(
            parse_optional_text(input),
            default.clone(),
        ))
    }

    fn format_cli(stored: &Self::Stored, default: &Self::Default) -> String {
        stored
            .clone()
            .or_else(|| default.clone())
            .unwrap_or_else(|| "none".to_string())
    }

    fn format_default(default: &Self::Default) -> String {
        default.clone().unwrap_or_else(|| "none".to_string())
    }

    fn validate(stored: &Self::Stored) -> Result<()> {
        V::validate(stored)
    }

    fn reset_value(_: &Self::Default) -> Self::Stored {
        None
    }
}
