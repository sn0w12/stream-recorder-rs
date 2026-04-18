//! Unified uploader system for file hosting services.
//!
//! This module provides a common interface for uploading files to various hosting services.
//! All uploaders implement the `Uploader` trait, which provides a consistent API for:
//! - Uploading files
//! - Checking if the uploader is ready/configured
//! - Getting the uploader's name

pub mod bunkr;
pub mod error;
pub mod fileditch;
pub mod filester;
pub mod gofile;
mod http;

use async_trait::async_trait;
use error::UploadError;
use serde_json::Value;

use crate::config::Config;

/// Configuration options for uploaders.
///
/// This struct contains common configuration options that may be used by different uploaders.
/// Not all fields are used by all uploaders - each uploader uses only the fields it needs.
#[derive(Clone, Debug, Default)]
pub struct UploaderConfig {
    /// Authentication token (used by gofile, bunkr)
    pub token: Option<String>,
    /// Folder ID for organizing uploads (used by gofile)
    pub folder_id: Option<String>,
    /// Server to upload to (used by gofile)
    pub server: Option<String>,
}

/// Result of an upload operation.
///
/// Contains the URLs where the uploaded files can be accessed, and optionally
/// the raw JSON response from the service for debugging or additional processing.
#[derive(Debug, Clone)]
pub struct UploadResult {
    /// URLs where the uploaded file(s) can be accessed
    pub urls: Vec<String>,
    /// Optional raw JSON response from the upload service
    #[allow(dead_code)]
    pub raw_response: Option<Value>,
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

    /// From a human-readable string like "10MB", "5GiB", etc.
    pub fn from_str(size_str: &str) -> Result<Self, String> {
        let size_str = size_str.trim();
        let (num_part, unit_part) = size_str
            .chars()
            .partition::<String, _>(|c| c.is_digit(10) || *c == '.');
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
            "" => Self::from_bytes(num as u64), // default to bytes if no unit
            _ => return Err(format!("invalid unit: {}", unit_part)),
        };
        Ok(file_size)
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

impl Default for FileSize {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Common trait that all uploaders must implement.
///
/// This trait provides a unified interface for uploading files to different hosting services.
/// Implementations should handle all service-specific details internally and return results
/// in a standardized format.
#[async_trait]
#[allow(dead_code)]
pub trait Uploader: Send + Sync {
    /// Upload a file and return the resulting URLs.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to upload
    /// * `config` - Configuration options for the upload (authentication, settings, etc.)
    ///
    /// # Returns
    ///
    /// Returns `UploadResult` containing URLs where the file can be accessed.
    ///
    /// # Errors
    ///
    /// Returns `UploadError` if the upload fails for any reason (network, authentication, etc.)
    async fn upload_file(
        &self,
        file_path: &str,
        config: &UploaderConfig,
    ) -> Result<UploadResult, UploadError>;

    /// Optional: Get album ID by name (if the service supports albums/folders)
    ///
    /// The default implementation simply returns `Ok(None)` indicating that the
    /// service does not support albums. Uploaders which *do* support albums can
    /// override this method and provide a concrete implementation. Returning
    /// `Err` signals an unexpected failure when querying the service.
    #[allow(unused_variables)]
    async fn get_folder_id_by_name(
        &self,
        folder_name: &str,
    ) -> Result<Option<String>, UploadError> {
        Ok(None)
    }

    /// Get the name of this uploader (e.g., "gofile", "bunkr").
    ///
    /// This name is used for logging and identifying which uploader is being used.
    fn name(&self) -> &str;

    /// Get the maximum file size this uploader supports.
    fn max_file_size(&self) -> FileSize {
        FileSize::MAX
    }

    /// Check if this uploader is configured and ready to use.
    ///
    /// Returns `true` if the uploader has all necessary credentials and configuration
    /// to perform uploads, `false` otherwise.
    async fn is_ready(&self) -> bool;
}

/// A list of boxed uploaders.
pub type UploaderList = Vec<Box<dyn Uploader>>;

macro_rules! uploader_list {
    ($( $async:ident $name:ident => $module:ident => $enum:ident ),* $(,)?) => {
        /// Enum representing the different uploader types.
        #[derive(Clone, Debug)]
        pub enum UploaderType {
            $(
                $enum,
            )*
        }

        /// Get a list of available uploaders that are configured and ready to use.
        pub async fn get_uploaders() -> UploaderList {
            let mut uploaders: UploaderList = Vec::new();
            $(
                uploader_list!(@push $async, $module, $name, uploaders);
            )*
            uploaders
        }

        /// Get a list of all possible uploader types and names for display purposes.
        pub async fn get_all_uploader_types_and_names() -> Vec<(UploaderType, String)> {
            let mut list = Vec::new();
            $(
                let uploader = uploader_list!(@new $async, $module, $name);
                list.push((UploaderType::$enum, uploader.name().to_string()));
            )*
            list
        }
    };

    (@push async, $module:ident, $name:ident, $uploaders:ident) => {
        let uploader = $module::$name::new().await;
        $uploaders.push(Box::new(uploader));
    };

    (@push sync, $module:ident, $name:ident, $uploaders:ident) => {
        $uploaders.push(Box::new($module::$name::new()));
    };

    (@new async, $module:ident, $name:ident) => {
        $module::$name::new().await
    };

    (@new sync, $module:ident, $name:ident) => {
        $module::$name::new()
    };
}

uploader_list! {
    async BunkrUploader => bunkr => Bunkr,
    sync GoFileUploader => gofile => GoFile,
    sync FileditchUploader => fileditch => Fileditch,
    sync FilesterUploader => filester => Filester,
}

/// Build a list of uploaders based on user configuration and readiness.
pub async fn build_uploaders() -> Vec<(Box<dyn Uploader>, UploaderConfig)> {
    let disabled_uploaders: Vec<String> = Config::get()
        .get_disabled_uploaders()
        .into_iter()
        .map(|u| u.to_lowercase())
        .collect();
    let all_uploaders = get_uploaders().await;

    let mut uploaders: Vec<(Box<dyn Uploader>, UploaderConfig)> = Vec::new();
    for uploader in all_uploaders {
        if disabled_uploaders.contains(&uploader.name().to_string().to_lowercase()) {
            continue;
        }

        if uploader.is_ready().await {
            uploaders.push((uploader, UploaderConfig::default()));
        }
    }

    uploaders
}
