use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

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

    /// From kilobytes (decimal, 1 KB = 1000 bytes)
    pub const fn from_kb(kb: u64) -> Self {
        Self(kb.saturating_mul(Self::BYTES_PER_KB))
    }

    /// From kibibytes (binary, 1 KiB = 1024 bytes)
    pub const fn from_kib(kib: u64) -> Self {
        Self(kib.saturating_mul(Self::BYTES_PER_KIB))
    }

    /// From megabytes (decimal, 1 MB = 1,000,000 bytes)
    pub const fn from_mb(mb: u64) -> Self {
        Self(mb.saturating_mul(Self::BYTES_PER_MB))
    }

    /// From mebibytes (binary, 1 MiB = 1,048,576 bytes)
    pub const fn from_mib(mib: u64) -> Self {
        Self(mib.saturating_mul(Self::BYTES_PER_MIB))
    }

    /// From gigabytes (decimal, 1 GB = 1,000,000,000 bytes)
    pub const fn from_gb(gb: u64) -> Self {
        Self(gb.saturating_mul(Self::BYTES_PER_GB))
    }

    /// From gibibytes (binary, 1 GiB = 1,073,741,824 bytes)
    pub const fn from_gib(gib: u64) -> Self {
        Self(gib.saturating_mul(Self::BYTES_PER_GIB))
    }

    /// Get the file size in bytes.
    pub const fn as_bytes(self) -> u64 {
        self.0
    }

    /// Get the file size in kilobytes (decimal, 1 KB = 1000 bytes).
    pub const fn as_kb(self) -> u64 {
        self.0 / Self::BYTES_PER_KB
    }

    /// Get the file size in kibibytes (binary, 1 KiB = 1024 bytes).
    pub const fn as_kib(self) -> u64 {
        self.0 / Self::BYTES_PER_KIB
    }

    /// Get the file size in megabytes (decimal, 1 MB = 1,000,000 bytes).
    pub const fn as_mb(self) -> u64 {
        self.0 / Self::BYTES_PER_MB
    }

    /// Get the file size in mebibytes (binary, 1 MiB = 1,048,576 bytes).
    pub const fn as_mib(self) -> u64 {
        self.0 / Self::BYTES_PER_MIB
    }

    /// Get the file size in gigabytes (decimal, 1 GB = 1,000,000,000 bytes).
    pub const fn as_gb(self) -> u64 {
        self.0 / Self::BYTES_PER_GB
    }

    /// Get the file size in gibibytes (binary, 1 GiB = 1,073,741,824 bytes).
    pub const fn as_gib(self) -> u64 {
        self.0 / Self::BYTES_PER_GIB
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

        for (unit_bytes, suffix) in candidates {
            if bytes.is_multiple_of(unit_bytes) {
                return write!(f, "{}{}", bytes / unit_bytes, suffix);
            }
        }

        write!(f, "{}B", bytes)
    }
}
