use crate::stream::postprocess::thumb::parse_thumbnail_string;
use apto::ConfigValidator;

pub struct VideoQuality;
pub struct ThumbnailSize;
pub struct ThumbnailGrid;
pub struct FfmpegBitrate;
pub struct PositiveU32;
#[allow(dead_code)]
pub struct PositiveF64;
pub struct Url;
pub struct RegexList;

impl ConfigValidator<Option<u32>> for VideoQuality {
    fn validate(value: &Option<u32>) -> Result<(), apto::ConfigError> {
        let Some(value) = value else {
            return Ok(());
        };

        if (1..=51).contains(value) {
            Ok(())
        } else {
            Err(apto::ConfigError::InvalidValue(
                "video quality must be between 1 and 51".to_string(),
            ))
        }
    }
}

fn validate_thumbnail_pair(
    value: &Option<String>,
    label: &str,
    format_hint: &str,
) -> Result<(), apto::ConfigError> {
    let Some(value) = value.as_deref() else {
        return Ok(());
    };

    let (first, second) = parse_thumbnail_string(value).ok_or_else(|| {
        apto::ConfigError::InvalidValue(format!("{label} must use {format_hint} format"))
    })?;

    if first == 0 || second == 0 {
        return Err(apto::ConfigError::InvalidValue(format!(
            "{label} values must be greater than zero"
        )));
    }

    Ok(())
}

impl ConfigValidator<Option<String>> for ThumbnailSize {
    fn validate(value: &Option<String>) -> Result<(), apto::ConfigError> {
        validate_thumbnail_pair(value, "thumbnail size", "WIDTHxHEIGHT")
    }
}

impl ConfigValidator<Option<String>> for ThumbnailGrid {
    fn validate(value: &Option<String>) -> Result<(), apto::ConfigError> {
        validate_thumbnail_pair(value, "thumbnail grid", "COLSxROWS")
    }
}

impl ConfigValidator<Option<String>> for FfmpegBitrate {
    fn validate(value: &Option<String>) -> Result<(), apto::ConfigError> {
        let Some(value) = value.as_deref() else {
            return Ok(());
        };

        let bitrate = value.trim();
        if bitrate.is_empty() {
            return Err(apto::ConfigError::InvalidValue(
                "bitrate cannot be empty; use 'none' to clear the setting".to_string(),
            ));
        }

        let split_index = bitrate
            .find(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
            .unwrap_or(bitrate.len());
        let (number_part, suffix) = bitrate.split_at(split_index);

        if number_part.is_empty() {
            return Err(apto::ConfigError::InvalidValue(
                "bitrate must start with a number, e.g. 2500k or 6M".to_string(),
            ));
        }

        if number_part.starts_with('.') || number_part.ends_with('.') {
            return Err(apto::ConfigError::InvalidValue(
                "bitrate number must be a whole number or decimal like 2.5M".to_string(),
            ));
        }

        if number_part.chars().filter(|&ch| ch == '.').count() > 1 {
            return Err(apto::ConfigError::InvalidValue(
                "bitrate number must contain at most one decimal point".to_string(),
            ));
        }

        let numeric_value = number_part.parse::<f64>().map_err(|_| {
            apto::ConfigError::InvalidValue(
                "bitrate must contain a valid positive number".to_string(),
            )
        })?;

        if !numeric_value.is_finite() || numeric_value <= 0.0 {
            return Err(apto::ConfigError::InvalidValue(
                "bitrate must be greater than zero".to_string(),
            ));
        }

        let suffix = suffix.to_ascii_lowercase();
        if matches!(suffix.as_str(), "" | "k" | "m" | "g" | "ki" | "mi" | "gi") {
            Ok(())
        } else {
            Err(apto::ConfigError::InvalidValue(
                "bitrate must use an ffmpeg-style suffix like 2500k, 6M, or 2.5Mi".to_string(),
            ))
        }
    }
}

impl ConfigValidator<Option<u32>> for PositiveU32 {
    fn validate(value: &Option<u32>) -> Result<(), apto::ConfigError> {
        let Some(value) = value else {
            return Ok(());
        };

        if *value > 0 {
            Ok(())
        } else {
            Err(apto::ConfigError::InvalidValue(
                "value must be greater than zero".to_string(),
            ))
        }
    }
}

impl ConfigValidator<Option<f64>> for PositiveF64 {
    fn validate(value: &Option<f64>) -> Result<(), apto::ConfigError> {
        let Some(value) = value else {
            return Ok(());
        };

        if *value > 0.0 {
            Ok(())
        } else {
            Err(apto::ConfigError::InvalidValue(
                "value must be greater than zero".to_string(),
            ))
        }
    }
}

impl ConfigValidator<Option<String>> for Url {
    fn validate(value: &Option<String>) -> Result<(), apto::ConfigError> {
        let Some(value) = value.as_deref() else {
            return Ok(());
        };

        if value.starts_with("https://") || value.starts_with("http://") {
            Ok(())
        } else {
            Err(apto::ConfigError::InvalidValue(
                "URL must start with http:// or https://".to_string(),
            ))
        }
    }
}

impl ConfigValidator<Option<Vec<String>>> for RegexList {
    fn validate(value: &Option<Vec<String>>) -> Result<(), apto::ConfigError> {
        let Some(value) = value.as_deref() else {
            return Ok(());
        };

        for regex_str in value {
            regex::Regex::new(regex_str).map(|_| ()).map_err(|error| {
                apto::ConfigError::InvalidValue(format!("Invalid regular expression: {}", error))
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_quality_validator_enforces_range() {
        assert!(VideoQuality::validate(&Some(1)).is_ok());
        assert!(VideoQuality::validate(&Some(51)).is_ok());

        let err = VideoQuality::validate(&Some(0)).expect_err("0 should be rejected");
        assert!(err.to_string().contains("between 1 and 51"));
    }

    #[test]
    fn thumbnail_size_validator_checks_format_and_zero_values() {
        assert!(ThumbnailSize::validate(&Some("320x180".to_string())).is_ok());

        let format_err = ThumbnailSize::validate(&Some("320".to_string()))
            .expect_err("missing separator should be rejected");
        assert!(format_err.to_string().contains("WIDTHxHEIGHT"));

        let zero_err = ThumbnailSize::validate(&Some("0x180".to_string()))
            .expect_err("zero values should be rejected");
        assert!(zero_err.to_string().contains("greater than zero"));
    }

    #[test]
    fn thumbnail_grid_validator_checks_format_and_zero_values() {
        assert!(ThumbnailGrid::validate(&Some("3x3".to_string())).is_ok());

        let format_err = ThumbnailGrid::validate(&Some("3".to_string()))
            .expect_err("missing separator should be rejected");
        assert!(format_err.to_string().contains("COLSxROWS"));

        let zero_err = ThumbnailGrid::validate(&Some("3x0".to_string()))
            .expect_err("zero values should be rejected");
        assert!(zero_err.to_string().contains("greater than zero"));
    }

    #[test]
    fn ffmpeg_bitrate_validator_checks_common_inputs() {
        assert!(FfmpegBitrate::validate(&Some("2.5M".to_string())).is_ok());

        let err = FfmpegBitrate::validate(&Some("fast".to_string()))
            .expect_err("nonsense bitrate should be rejected");
        assert!(err.to_string().contains("start with a number"));
    }

    #[test]
    fn positive_u32_validator_rejects_zero() {
        assert!(PositiveU32::validate(&Some(1)).is_ok());

        let err = PositiveU32::validate(&Some(0)).expect_err("0 should be rejected");
        assert!(err.to_string().contains("greater than zero"));
    }

    #[test]
    fn url_validator_accepts_http_and_https_only() {
        assert!(Url::validate(&Some("https://example.com".to_string())).is_ok());

        let err = Url::validate(&Some("ftp://example.com".to_string()))
            .expect_err("non-http URL should be rejected");
        assert!(err.to_string().contains("http:// or https://"));
    }

    #[test]
    fn regex_list_validator_rejects_invalid_expressions() {
        assert!(
            RegexList::validate(&Some(vec![r"foo.*".to_string(), r"^bar$".to_string()])).is_ok()
        );

        let err = RegexList::validate(&Some(vec!["[".to_string()]))
            .expect_err("invalid regex should be rejected");
        assert!(err.to_string().contains("Invalid regular expression"));
    }
}
