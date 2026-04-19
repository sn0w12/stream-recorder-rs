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
pub mod jpg6;

use async_trait::async_trait;
use error::UploadError;
use serde_json::Value;

use crate::{config::Config, types::FileSize};

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

/// Broad uploader category used for filtering uploaders.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UploaderKind {
    Video,
    Image,
}

/// Filter for selecting which uploader kinds should be included.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UploaderKindFilter {
    All,
    Video,
    Image,
}

impl UploaderKindFilter {
    fn matches(self, kind: UploaderKind) -> bool {
        match self {
            UploaderKindFilter::All => true,
            UploaderKindFilter::Video => kind == UploaderKind::Video,
            UploaderKindFilter::Image => kind == UploaderKind::Image,
        }
    }
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

    /// Get the broad category of this uploader.
    fn kind(&self) -> UploaderKind;

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
    sync Jpg6Uploader => jpg6 => Jpg6,
}

/// Build a list of uploaders based on user configuration and readiness.
pub async fn build_uploaders(
    filter: UploaderKindFilter,
) -> Vec<(Box<dyn Uploader>, UploaderConfig)> {
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

        if uploader.is_ready().await && filter.matches(uploader.kind()) {
            uploaders.push((uploader, UploaderConfig::default()));
        }
    }

    uploaders
}

#[cfg(test)]
mod tests {
    use super::{UploaderKind, UploaderKindFilter};

    #[test]
    fn uploader_kind_filter_matches_expected_kinds() {
        assert!(UploaderKindFilter::All.matches(UploaderKind::Video));
        assert!(UploaderKindFilter::All.matches(UploaderKind::Image));
        assert!(UploaderKindFilter::Video.matches(UploaderKind::Video));
        assert!(!UploaderKindFilter::Video.matches(UploaderKind::Image));
        assert!(UploaderKindFilter::Image.matches(UploaderKind::Image));
        assert!(!UploaderKindFilter::Image.matches(UploaderKind::Video));
    }
}
