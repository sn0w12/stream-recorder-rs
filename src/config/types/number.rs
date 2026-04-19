use crate::config::types::{
    ConfigType, ConfigValidator, NoValidation, normalize_optional_value, parse_number,
    parse_optional_value,
};
use anyhow::Result;
use std::marker::PhantomData;

/// Unsigned integer setting stored as an optional value with a non-optional default.
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

    fn parse(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_optional_value(
            parse_optional_value(input, parse_number::<u32>)?,
            Some(*default),
        ))
    }

    fn validate(stored: &Self::Stored) -> Result<()> {
        V::validate(stored)
    }
}

/// Unsigned integer setting stored as an optional value with an optional default.
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

    fn parse(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_optional_value(
            parse_optional_value(input, |value| {
                value
                    .parse::<u32>()
                    .map_err(|_| anyhow::anyhow!("expected u32, got '{}'", value))
            })?,
            *default,
        ))
    }

    fn validate(stored: &Self::Stored) -> Result<()> {
        V::validate(stored)
    }
}

/// Floating-point setting stored as an optional value with a non-optional default.
#[allow(dead_code)]
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

    fn parse(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_optional_value(
            parse_optional_value(input, parse_number::<f64>)?,
            Some(*default),
        ))
    }

    fn validate(stored: &Self::Stored) -> Result<()> {
        V::validate(stored)
    }
}

/// Floating-point setting stored as an optional value with an optional default.
#[allow(dead_code)]
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

    fn parse(input: &str, default: &Self::Default) -> Result<Self::Stored> {
        Ok(normalize_optional_value(
            parse_optional_value(input, parse_number::<f64>)?,
            *default,
        ))
    }

    fn validate(stored: &Self::Stored) -> Result<()> {
        V::validate(stored)
    }
}

#[cfg(test)]
mod tests {
    use super::{Float, OptionalFloat, OptionalU32, U32};
    use crate::config::types::{ConfigType, NoValidation};

    type U32Type = U32<NoValidation>;
    type OptionalU32Type = OptionalU32<NoValidation>;
    type FloatType = Float<NoValidation>;
    type OptionalFloatType = OptionalFloat<NoValidation>;

    #[test]
    fn u32_type_parses_and_formats_cli_values() {
        let default = 26;

        assert_eq!(
            U32Type::parse("30", &default).expect("valid u32 should parse"),
            Some(30)
        );
        assert_eq!(
            U32Type::parse("26", &default).expect("default u32 should normalize"),
            None
        );
        assert_eq!(U32Type::format(&Some(30), &default), "30");
        assert_eq!(U32Type::format_default(&default), "26");
        assert_eq!(U32Type::get(&Some(30), &default), 30);
        assert_eq!(U32Type::get(&None, &default), 26);
    }

    #[test]
    fn optional_u32_type_handles_none_and_defaults() {
        let default = Some(12);

        assert_eq!(
            OptionalU32Type::parse("none", &default).expect("none should clear optional u32"),
            None
        );
        assert_eq!(
            OptionalU32Type::parse("14", &default).expect("valid optional u32 should parse"),
            Some(14)
        );
        assert_eq!(OptionalU32Type::format(&None, &default), "12");
        assert_eq!(OptionalU32Type::format_default(&default), "12");
        assert_eq!(OptionalU32Type::get(&None, &default), Some(12));
    }

    #[test]
    fn float_type_parses_and_formats_cli_values() {
        let default = 1.5;

        assert_eq!(
            FloatType::parse("2.25", &default).expect("valid float should parse"),
            Some(2.25)
        );
        assert_eq!(
            FloatType::parse("1.5", &default).expect("default float should normalize"),
            None
        );
        assert_eq!(FloatType::format(&Some(2.25), &default), "2.25");
        assert_eq!(FloatType::format_default(&default), "1.5");
        assert_eq!(FloatType::get(&Some(2.25), &default), 2.25);
        assert_eq!(FloatType::get(&None, &default), 1.5);
    }

    #[test]
    fn optional_float_type_handles_none_and_defaults() {
        let default = Some(3.5);

        assert_eq!(
            OptionalFloatType::parse("none", &default).expect("none should clear optional float"),
            None
        );
        assert_eq!(
            OptionalFloatType::parse("4.5", &default).expect("valid optional float should parse"),
            Some(4.5)
        );
        assert_eq!(OptionalFloatType::format(&None, &default), "3.5");
        assert_eq!(OptionalFloatType::format_default(&default), "3.5");
        assert_eq!(OptionalFloatType::get(&None, &default), Some(3.5));
    }
}
