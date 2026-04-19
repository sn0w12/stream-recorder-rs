use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DurationValue(Duration);

impl DurationValue {
    pub const ZERO: Self = Self(Duration::ZERO);

    pub fn from_duration(duration: Duration) -> Self {
        Self(duration)
    }

    pub fn from_secs(seconds: u64) -> Self {
        Self(Duration::from_secs(seconds))
    }

    pub fn from_millis(milliseconds: u64) -> Self {
        Self(Duration::from_millis(milliseconds))
    }

    pub fn from_secs_f64(seconds: f64) -> Result<Self, String> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("duration must be a finite non-negative number".to_string());
        }

        Ok(Self(Duration::from_secs_f64(seconds)))
    }

    pub fn into_duration(self) -> Duration {
        self.0
    }

    pub fn as_duration(&self) -> Duration {
        self.0
    }

    pub fn as_secs_f64(self) -> f64 {
        self.0.as_secs_f64()
    }

    pub fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    fn parse_explicit(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("duration cannot be empty".to_string());
        }

        let mut total_nanos = 0_u128;
        let mut rest = trimmed;

        while !rest.trim_start().is_empty() {
            rest = rest.trim_start();

            let number_end = rest
                .find(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
                .unwrap_or(rest.len());
            if number_end == 0 {
                return Err(format!("expected a duration value near '{}'", rest));
            }

            let number_part = &rest[..number_end];
            if number_part.starts_with('.') || number_part.ends_with('.') {
                return Err(format!("invalid duration value '{}'", number_part));
            }
            if number_part.chars().filter(|&ch| ch == '.').count() > 1 {
                return Err(format!("invalid duration value '{}'", number_part));
            }

            let amount = number_part
                .parse::<f64>()
                .map_err(|_| format!("invalid duration value '{}'", number_part))?;
            if !amount.is_finite() || amount < 0.0 {
                return Err("duration must be a finite non-negative number".to_string());
            }

            let unit_start = number_end;
            let unit_len = rest[unit_start..]
                .find(|ch: char| !ch.is_ascii_alphabetic())
                .unwrap_or(rest.len() - unit_start);
            if unit_len == 0 {
                return Err(format!(
                    "missing duration unit after '{}' (expected units like ms, s, m, h, d)",
                    number_part
                ));
            }

            let unit = rest[unit_start..unit_start + unit_len].to_ascii_lowercase();
            let nanos_per_unit = match unit.as_str() {
                "ns" | "nanosecond" | "nanoseconds" => 1_u128,
                "us" | "microsecond" | "microseconds" => 1_000_u128,
                "ms" | "millisecond" | "milliseconds" => 1_000_000_u128,
                "s" | "sec" | "secs" | "second" | "seconds" => 1_000_000_000_u128,
                "m" | "min" | "mins" | "minute" | "minutes" => 60_u128 * 1_000_000_000,
                "h" | "hr" | "hrs" | "hour" | "hours" => 60_u128 * 60 * 1_000_000_000,
                "d" | "day" | "days" => 24_u128 * 60 * 60 * 1_000_000_000,
                _ => {
                    return Err(format!(
                        "unknown duration unit '{}' (expected ns, us, ms, s, m, h, or d)",
                        unit
                    ));
                }
            };

            let additional_nanos = (amount * nanos_per_unit as f64).round();
            if !additional_nanos.is_finite() || additional_nanos < 0.0 {
                return Err("duration overflowed supported range".to_string());
            }

            total_nanos = total_nanos
                .checked_add(additional_nanos as u128)
                .ok_or_else(|| "duration overflowed supported range".to_string())?;
            rest = &rest[unit_start + unit_len..];
        }

        let seconds = total_nanos / 1_000_000_000;
        let nanos = (total_nanos % 1_000_000_000) as u32;
        let seconds = u64::try_from(seconds)
            .map_err(|_| "duration overflowed supported range".to_string())?;

        Ok(Self(Duration::new(seconds, nanos)))
    }

    fn format_parts(&self, max_parts: usize) -> Vec<String> {
        let total_nanos = self.0.as_secs() as u128 * 1_000_000_000 + self.0.subsec_nanos() as u128;
        if total_nanos == 0 {
            return vec!["0s".to_string()];
        }

        let units = [
            (24_u128 * 60 * 60 * 1_000_000_000, "d"),
            (60_u128 * 60 * 1_000_000_000, "h"),
            (60_u128 * 1_000_000_000, "m"),
            (1_000_000_000_u128, "s"),
            (1_000_000_u128, "ms"),
            (1_000_u128, "us"),
            (1_u128, "ns"),
        ];

        let mut remaining = total_nanos;
        let mut parts = Vec::new();

        for (unit_nanos, suffix) in units {
            let value = remaining / unit_nanos;
            if value > 0 {
                parts.push(format!("{}{}", value, suffix));
                remaining %= unit_nanos;
            }
        }

        parts.truncate(max_parts);
        parts
    }
}

impl Default for DurationValue {
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<Duration> for DurationValue {
    fn from(value: Duration) -> Self {
        Self::from_duration(value)
    }
}

impl From<DurationValue> for Duration {
    fn from(value: DurationValue) -> Self {
        value.into_duration()
    }
}

impl PartialEq<Duration> for DurationValue {
    fn eq(&self, other: &Duration) -> bool {
        self.0 == *other
    }
}

impl PartialEq<DurationValue> for Duration {
    fn eq(&self, other: &DurationValue) -> bool {
        *self == other.0
    }
}

impl PartialOrd<Duration> for DurationValue {
    fn partial_cmp(&self, other: &Duration) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialOrd<DurationValue> for Duration {
    fn partial_cmp(&self, other: &DurationValue) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&other.0)
    }
}

impl FromStr for DurationValue {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::parse_explicit(input)
    }
}

impl fmt::Display for DurationValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_parts(3).join(" "))
    }
}

impl Serialize for DurationValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for DurationValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DurationVisitor;

        impl<'de> serde::de::Visitor<'de> for DurationVisitor {
            type Value = DurationValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .write_str("a human-readable duration string like 500ms, 30s, 5m, 2h, or 7d")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                parse_duration_input(value).map_err(E::custom)
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_str(DurationVisitor)
    }
}

pub fn parse_duration_input(input: &str) -> Result<DurationValue, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("duration cannot be empty".to_string());
    }

    DurationValue::from_str(trimmed)
}

#[cfg(test)]
mod tests {
    use super::{DurationValue, parse_duration_input};
    use std::time::Duration;

    #[test]
    fn parses_explicit_duration_strings() {
        assert_eq!(
            "1h 30m".parse::<DurationValue>().unwrap(),
            Duration::from_secs(5400)
        );
        assert_eq!(
            "500ms".parse::<DurationValue>().unwrap(),
            Duration::from_millis(500)
        );
        assert_eq!(
            "1.5d 2h".parse::<DurationValue>().unwrap(),
            Duration::from_secs(136_800)
        );
    }

    #[test]
    fn formats_compact_duration_strings() {
        assert_eq!(DurationValue::from_secs(95_465).to_string(), "1d 2h 31m");
        assert_eq!(DurationValue::from_millis(500).to_string(), "500ms");
        assert_eq!(DurationValue::from_secs(90).to_string(), "1m 30s");
        assert_eq!(DurationValue::from_secs(0).to_string(), "0s");
    }

    #[test]
    fn parses_duration_input_strings() {
        assert_eq!(
            parse_duration_input("2m30s").unwrap(),
            Duration::from_secs(150)
        );
    }

    #[test]
    fn serializes_and_deserializes_duration_values_as_strings() {
        let duration = DurationValue::from_millis(500);
        assert_eq!(serde_json::to_string(&duration).unwrap(), "\"500ms\"");

        let parsed: DurationValue = serde_json::from_str("\"2m30s\"").unwrap();
        assert_eq!(parsed, Duration::from_secs(150));
    }

    #[test]
    fn rejects_numeric_duration_deserialization() {
        let err = serde_json::from_str::<DurationValue>("15").unwrap_err();
        assert!(err.to_string().contains("human-readable duration"));
    }
}
