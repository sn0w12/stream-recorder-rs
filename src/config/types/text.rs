use crate::config::types::{
    ConfigType, ConfigValidator, NoValidation, normalize_optional_value, normalize_text_value,
    parse_optional_text,
};
use anyhow::Result;
use std::marker::PhantomData;

/// Text setting stored as an optional string with a non-optional default.
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

    fn parse(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_text_value(parse_optional_text(input), default))
    }

    fn validate(stored: &Self::Stored) -> Result<()> {
        V::validate(stored)
    }
}

/// Text setting stored as an optional string with an optional default.
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

    fn parse(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_optional_value(
            parse_optional_text(input),
            default.clone(),
        ))
    }

    fn validate(stored: &Self::Stored) -> Result<()> {
        V::validate(stored)
    }
}

#[cfg(test)]
mod tests {
    use super::{OptionalText, Text};
    use crate::config::types::{ConfigType, NoValidation};

    type TextType = Text<NoValidation>;
    type OptionalTextType = OptionalText<NoValidation>;

    #[test]
    fn text_type_normalizes_default_values() {
        let default = "alpha".to_string();

        assert_eq!(
            TextType::parse("beta", &default).expect("valid text should parse"),
            Some("beta".to_string())
        );
        assert_eq!(
            TextType::parse("alpha", &default).expect("default text should normalize"),
            None
        );
        assert_eq!(
            TextType::format(&Some("beta".to_string()), &default),
            "beta"
        );
        assert_eq!(TextType::format_default(&default), "alpha");
        assert_eq!(TextType::get(&Some("beta".to_string()), &default), "beta");
        assert_eq!(TextType::get(&None, &default), "alpha");
    }

    #[test]
    fn optional_text_type_handles_none_and_defaults() {
        let default = Some("alpha".to_string());

        assert_eq!(
            OptionalTextType::parse("none", &default).expect("none should clear optional text"),
            None
        );
        assert_eq!(
            OptionalTextType::parse("beta", &default).expect("valid optional text should parse"),
            Some("beta".to_string())
        );
        assert_eq!(OptionalTextType::format(&None, &default), "alpha");
        assert_eq!(OptionalTextType::format_default(&default), "alpha");
        assert_eq!(OptionalTextType::get(&None, &default), Some("alpha"));
    }
}
