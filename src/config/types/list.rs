use crate::config::types::{
    ConfigType, ConfigValidator, NoValidation, normalize_list_value, parse_csv_list,
};
use anyhow::Result;
use std::marker::PhantomData;

pub struct StringList<V = NoValidation>(PhantomData<V>);

impl<V> ConfigType for StringList<V>
where
    V: ConfigValidator<Option<Vec<String>>>,
{
    type Stored = Option<Vec<String>>;
    type Default = Vec<String>;
    type Value<'a> = Vec<String>;

    fn get<'a>(stored: &'a Self::Stored, default: &'a Self::Default) -> Self::Value<'a> {
        stored.clone().unwrap_or_else(|| default.clone())
    }

    fn parse_cli(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_list_value(parse_csv_list(input), default))
    }

    fn format_cli(stored: &Self::Stored, default: &Self::Default) -> String {
        let values = stored.as_ref().unwrap_or(default);
        if values.is_empty() {
            "none".to_string()
        } else {
            values.join(", ")
        }
    }

    fn format_default(default: &Self::Default) -> String {
        if default.is_empty() {
            "none".to_string()
        } else {
            default.join(", ")
        }
    }

    fn validate(stored: &Self::Stored) -> Result<()> {
        V::validate(stored)
    }

    fn reset_value(_: &Self::Default) -> Self::Stored {
        None
    }

    fn is_array() -> bool {
        true
    }
}
