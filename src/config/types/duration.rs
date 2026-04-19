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

#[cfg(test)]
mod tests {
    use super::{Duration, OptionalDuration};
    use crate::config::types::{ConfigType, NoValidation};
    use crate::types::DurationValue;
    use std::time::Duration as StdDuration;

    type DurationType = Duration<NoValidation>;
    type OptionalDurationType = OptionalDuration<NoValidation>;

    #[test]
    fn duration_type_parses_and_formats_cli_values() {
        let default = DurationValue::from_millis(500);

        assert_eq!(
            DurationType::parse_cli("2s", &default).expect("valid duration should parse"),
            Some(DurationValue::from_secs(2))
        );
        assert_eq!(
            DurationType::parse_cli("500ms", &default).expect("default duration should normalize"),
            None
        );
        assert_eq!(
            DurationType::format_cli(&Some(DurationValue::from_secs(2)), &default),
            "2s"
        );
        assert_eq!(DurationType::format_default(&default), "500ms");
        assert_eq!(
            DurationType::get(&Some(DurationValue::from_secs(2)), &default),
            StdDuration::from_secs(2)
        );
        assert_eq!(
            DurationType::get(&None, &default),
            StdDuration::from_millis(500)
        );
    }

    #[test]
    fn optional_duration_type_handles_none_and_defaults() {
        let default = Some(DurationValue::from_secs(90));

        assert_eq!(
            OptionalDurationType::parse_cli("none", &default)
                .expect("none should clear optional durations"),
            None
        );
        assert_eq!(
            OptionalDurationType::parse_cli("2m", &default)
                .expect("valid optional duration should parse"),
            Some(DurationValue::from_secs(120))
        );
        assert_eq!(OptionalDurationType::format_cli(&None, &default), "1m30s");
        assert_eq!(OptionalDurationType::format_default(&default), "1m30s");
        assert_eq!(
            OptionalDurationType::get(&None, &default),
            Some(StdDuration::from_secs(90))
        );
    }
}
