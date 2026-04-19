use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

macro_rules! file_size_units {
    ($(($from_fn:ident, $as_fn:ident, $bytes_const:ident, $unit_label:expr)),+ $(,)?) => {
        $(
            #[doc = concat!("Create a FileSize from ", $unit_label, ".")]
            pub const fn $from_fn(value: u64) -> Self {
                Self(value.saturating_mul(Self::$bytes_const))
            }

            #[doc = concat!("Get the file size in ", $unit_label, ".")]
            pub const fn $as_fn(self) -> u64 {
                self.0 / Self::$bytes_const
            }
        )+
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileSize(u64); // stored as bytes

#[allow(dead_code)]
impl FileSize {
    const BYTES_PER_KB: u64 = 1_000;
    const BYTES_PER_KIB: u64 = 1_024;
    const BYTES_PER_MB: u64 = 1_000_000;
    const BYTES_PER_MIB: u64 = 1_048_576;
    const BYTES_PER_GB: u64 = 1_000_000_000;
    const BYTES_PER_GIB: u64 = 1_073_741_824;

    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(u64::MAX);

    /// Create a FileSize from bytes.
    pub const fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    /// Get the file size in bytes.
    pub const fn as_bytes(self) -> u64 {
        self.0
    }

    file_size_units! {
        (from_kb, as_kb, BYTES_PER_KB, "KB"),
        (from_kib, as_kib, BYTES_PER_KIB, "KiB"),
        (from_mb, as_mb, BYTES_PER_MB, "MB"),
        (from_mib, as_mib, BYTES_PER_MIB, "MiB"),
        (from_gb, as_gb, BYTES_PER_GB, "GB"),
        (from_gib, as_gib, BYTES_PER_GIB, "GiB")
    }
}

impl FromStr for FileSize {
    type Err = String;

    /// Parse a human-readable string like "10MB" or "5GiB".
    fn from_str(size_str: &str) -> Result<Self, Self::Err> {
        let size_str = size_str.trim();
        let (num_part, unit_part) = size_str
            .chars()
            .partition::<String, _>(|c| c.is_ascii_digit() || *c == '.');
        let num: f64 = num_part
            .parse()
            .map_err(|e| format!("invalid number: {}", e))?;
        let file_size = match unit_part.to_uppercase().as_str() {
            "B" => Self::from_bytes(num as u64),
            "KB" => Self::from_kb(num as u64),
            "KIB" => Self::from_kib(num as u64),
            "MB" => Self::from_mb(num as u64),
            "MIB" => Self::from_mib(num as u64),
            "GB" => Self::from_gb(num as u64),
            "GIB" => Self::from_gib(num as u64),
            "" => Self::from_bytes(num as u64),
            _ => return Err(format!("invalid unit: {}", unit_part)),
        };
        Ok(file_size)
    }
}

impl Default for FileSize {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Serialize for FileSize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.0)
    }
}

impl<'de> Deserialize<'de> for FileSize {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FileSizeVisitor;

        impl<'de> serde::de::Visitor<'de> for FileSizeVisitor {
            type Value = FileSize;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(
                    "a byte count, legacy gigabyte float, or human-readable file size string",
                )
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                Ok(FileSize::from_bytes(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let value = u64::try_from(value)
                    .map_err(|_| E::custom("file size must be non-negative"))?;
                Ok(FileSize::from_bytes(value))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if !value.is_finite() || value < 0.0 {
                    return Err(E::custom("file size must be a finite non-negative number"));
                }

                Ok(FileSize::from_bytes((value * 1_000_000_000.0) as u64))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                value.parse::<FileSize>().map_err(E::custom)
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_any(FileSizeVisitor)
    }
}

impl fmt::Display for FileSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0;

        let candidates = [
            (Self::BYTES_PER_GIB, "GiB"),
            (Self::BYTES_PER_GB, "GB"),
            (Self::BYTES_PER_MIB, "MiB"),
            (Self::BYTES_PER_MB, "MB"),
            (Self::BYTES_PER_KIB, "KiB"),
            (Self::BYTES_PER_KB, "KB"),
        ];

        let mut fallback = None;

        for (unit_bytes, suffix) in candidates {
            if bytes.is_multiple_of(unit_bytes) {
                return write!(f, "{}{}", bytes / unit_bytes, suffix);
            }

            if bytes >= unit_bytes {
                fallback.get_or_insert_with(|| {
                    let value = bytes as f64 / unit_bytes as f64;
                    let mut formatted = format!("{value:.2}");

                    while formatted.contains('.') && formatted.ends_with('0') {
                        formatted.pop();
                    }
                    if formatted.ends_with('.') {
                        formatted.pop();
                    }

                    (formatted, suffix)
                });
            }
        }

        if let Some((formatted, suffix)) = fallback {
            return write!(f, "{}{}", formatted, suffix);
        }

        write!(f, "{}B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_human_readable_sizes() {
        assert_eq!("42".parse::<FileSize>().unwrap(), FileSize::from_bytes(42));
        assert_eq!("10KB".parse::<FileSize>().unwrap(), FileSize::from_kb(10));
        assert_eq!("5MiB".parse::<FileSize>().unwrap(), FileSize::from_mib(5));
        assert_eq!(
            "  1.5gb  ".parse::<FileSize>().unwrap(),
            FileSize::from_gb(1)
        );
    }

    #[test]
    fn rejects_invalid_sizes() {
        assert!("10XB".parse::<FileSize>().is_err());
        assert!("not-a-size".parse::<FileSize>().is_err());
    }

    #[test]
    fn formats_human_readable_sizes() {
        assert_eq!(FileSize::from_bytes(1_024).to_string(), "1KiB");
        assert_eq!(FileSize::from_bytes(1_000_000).to_string(), "1MB");
        assert_eq!(FileSize::from_bytes(20_000_000_000).to_string(), "20GB");
        assert_eq!(FileSize::from_bytes(1536).to_string(), "1.5KiB");
        assert_eq!(FileSize::from_bytes(4_673_595_901).to_string(), "4.35GiB");
    }

    #[test]
    fn serializes_and_deserializes_byte_counts() {
        let size = FileSize::from_mib(2);

        let serialized = serde_json::to_string(&size).unwrap();
        assert_eq!(serialized, "2097152");

        let deserialized: FileSize = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, size);
    }

    #[test]
    fn deserializes_legacy_gigabyte_floats() {
        let size: FileSize = serde_json::from_str("1.5").unwrap();

        assert_eq!(size.as_bytes(), 1_500_000_000);
    }
}
