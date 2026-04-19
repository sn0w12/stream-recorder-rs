use crate::config::types::{
    ConfigType, ConfigValidator, NoValidation, normalize_list_value, parse_csv_list,
};
use anyhow::Result;
use std::marker::PhantomData;

/// Comma-separated string list stored as an optional vector of strings.
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

    fn is_array() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::StringList;
    use crate::config::types::{ConfigType, NoValidation};

    type StringListType = StringList<NoValidation>;

    #[test]
    fn string_list_type_parses_csv_lists_and_marks_arrays() {
        let default = vec!["alpha".to_string(), "beta".to_string()];

        assert_eq!(
            StringListType::parse_cli("one, two", &default).expect("valid CSV list should parse"),
            Some(vec!["one".to_string(), "two".to_string()])
        );
        assert_eq!(
            StringListType::parse_cli("alpha, beta", &default)
                .expect("default CSV list should normalize"),
            None
        );
        assert_eq!(
            StringListType::format_cli(&Some(vec!["one".to_string(), "two".to_string()]), &default),
            "one, two"
        );
        assert_eq!(StringListType::format_default(&default), "alpha, beta");
        assert!(StringListType::is_array());
    }
}
