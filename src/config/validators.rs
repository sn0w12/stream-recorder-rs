use crate::config::types::ConfigValidator;
use crate::stream::postprocess::thumb::parse_thumbnail_string;
use crate::types::DurationValue;
use anyhow::Result;

pub struct VideoQuality;
pub struct ThumbnailSize;
pub struct ThumbnailGrid;
pub struct FfmpegBitrate;
pub struct PositiveU32;
#[allow(dead_code)]
pub struct PositiveF64;
pub struct PositiveDuration;
pub struct Url;
pub struct RegexList;

impl ConfigValidator<Option<u32>> for VideoQuality {
    fn validate(value: &Option<u32>) -> Result<()> {
        let Some(value) = value else {
            return Ok(());
        };

        if (1..=51).contains(value) {
            Ok(())
        } else {
            Err(anyhow::anyhow!("video quality must be between 1 and 51"))
        }
    }
}

fn validate_thumbnail_pair(value: &Option<String>, label: &str, format_hint: &str) -> Result<()> {
    let Some(value) = value.as_deref() else {
        return Ok(());
    };

    let (first, second) = parse_thumbnail_string(value)
        .ok_or_else(|| anyhow::anyhow!("{label} must use {format_hint} format"))?;

    if first == 0 || second == 0 {
        return Err(anyhow::anyhow!("{label} values must be greater than zero"));
    }

    Ok(())
}

impl ConfigValidator<Option<String>> for ThumbnailSize {
    fn validate(value: &Option<String>) -> Result<()> {
        validate_thumbnail_pair(value, "thumbnail size", "WIDTHxHEIGHT")
    }
}

impl ConfigValidator<Option<String>> for ThumbnailGrid {
    fn validate(value: &Option<String>) -> Result<()> {
        validate_thumbnail_pair(value, "thumbnail grid", "COLSxROWS")
    }
}

impl ConfigValidator<Option<String>> for FfmpegBitrate {
    fn validate(value: &Option<String>) -> Result<()> {
        let Some(value) = value.as_deref() else {
            return Ok(());
        };

        let bitrate = value.trim();
        if bitrate.is_empty() {
            return Err(anyhow::anyhow!(
                "bitrate cannot be empty; use 'none' to clear the setting"
            ));
        }

        let split_index = bitrate
            .find(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
            .unwrap_or(bitrate.len());
        let (number_part, suffix) = bitrate.split_at(split_index);

        if number_part.is_empty() {
            return Err(anyhow::anyhow!(
                "bitrate must start with a number, e.g. 2500k or 6M"
            ));
        }

        if number_part.starts_with('.') || number_part.ends_with('.') {
            return Err(anyhow::anyhow!(
                "bitrate number must be a whole number or decimal like 2.5M"
            ));
        }

        if number_part.chars().filter(|&ch| ch == '.').count() > 1 {
            return Err(anyhow::anyhow!(
                "bitrate number must contain at most one decimal point"
            ));
        }

        let numeric_value = number_part
            .parse::<f64>()
            .map_err(|_| anyhow::anyhow!("bitrate must contain a valid positive number"))?;

        if !numeric_value.is_finite() || numeric_value <= 0.0 {
            return Err(anyhow::anyhow!("bitrate must be greater than zero"));
        }

        let suffix = suffix.to_ascii_lowercase();
        if matches!(suffix.as_str(), "" | "k" | "m" | "g" | "ki" | "mi" | "gi") {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "bitrate must use an ffmpeg-style suffix like 2500k, 6M, or 2.5Mi"
            ))
        }
    }
}

impl ConfigValidator<Option<u32>> for PositiveU32 {
    fn validate(value: &Option<u32>) -> Result<()> {
        let Some(value) = value else {
            return Ok(());
        };

        if *value > 0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("value must be greater than zero"))
        }
    }
}

impl ConfigValidator<Option<f64>> for PositiveF64 {
    fn validate(value: &Option<f64>) -> Result<()> {
        let Some(value) = value else {
            return Ok(());
        };

        if *value > 0.0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("value must be greater than zero"))
        }
    }
}

impl ConfigValidator<Option<DurationValue>> for PositiveDuration {
    fn validate(value: &Option<DurationValue>) -> Result<()> {
        let Some(value) = value else {
            return Ok(());
        };

        if value.as_duration().is_zero() {
            Err(anyhow::anyhow!("value must be greater than zero"))
        } else {
            Ok(())
        }
    }
}

impl ConfigValidator<Option<String>> for Url {
    fn validate(value: &Option<String>) -> Result<()> {
        let Some(value) = value.as_deref() else {
            return Ok(());
        };

        if value.starts_with("https://") || value.starts_with("http://") {
            Ok(())
        } else {
            Err(anyhow::anyhow!("URL must start with http:// or https://"))
        }
    }
}

impl ConfigValidator<Option<Vec<String>>> for RegexList {
    fn validate(value: &Option<Vec<String>>) -> Result<()> {
        let Some(value) = value.as_deref() else {
            return Ok(());
        };

        for regex_str in value {
            regex::Regex::new(regex_str)
                .map(|_| ())
                .map_err(|error| anyhow::anyhow!("Invalid regular expression: {}", error))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FfmpegBitrate, PositiveDuration, PositiveU32, RegexList, ThumbnailGrid, ThumbnailSize, Url,
        VideoQuality,
    };
    use crate::config::types::ConfigValidator;
    use crate::types::DurationValue;

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
    fn positive_duration_validator_rejects_zero() {
        assert!(PositiveDuration::validate(&Some(DurationValue::from_secs(1))).is_ok());

        let err = PositiveDuration::validate(&Some(DurationValue::from_secs(0)))
            .expect_err("zero duration should be rejected");
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
