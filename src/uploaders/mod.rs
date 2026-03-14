//! Unified uploader system for file hosting services.
//!
//! This module provides a common interface for uploading files to various hosting services.
//! All uploaders implement the `Uploader` trait, which provides a consistent API for:
//! - Uploading files
//! - Checking if the uploader is ready/configured
//! - Getting the uploader's name
//!
//! # Adding a New Uploader
//!
//! To add a new uploader:
//!
//! 1. Create a new module file (e.g., `newservice.rs`)
//! 2. Implement the `Uploader` trait for your service
//! 3. Add the module to this file and export it
//! 4. Use it in `monitor.rs` like the existing uploaders
//!
//! ## Example
//!
//! ```rust,ignore
//! use async_trait::async_trait;
//! use super::{Uploader, UploaderConfig, UploadResult, error::UploadError};
//!
//! pub struct MyUploader {
//!     api_key: String,
//! }
//!
//! impl MyUploader {
//!     pub fn new(api_key: String) -> Self {
//!         Self { api_key }
//!     }
//! }
//!
//! #[async_trait]
//! impl Uploader for MyUploader {
//!     async fn upload_file(&self, file_path: &str, config: &UploaderConfig)
//!         -> Result<UploadResult, UploadError>
//!     {
//!         // Upload implementation here
//!         Ok(UploadResult {
//!             urls: vec!["https://example.com/file".to_string()],
//!             raw_response: None,
//!         })
//!     }
//!
//!     fn name(&self) -> &str {
//!         "myuploader"
//!     }
//!
//!     async fn is_ready(&self) -> bool {
//!         !self.api_key.is_empty()
//!     }
//! }
//! ```

pub mod bunkr;
pub mod error;
pub mod fileditch;
pub mod filester;
pub mod gofile;

use async_trait::async_trait;
use error::UploadError;
use serde_json::Value;

/// Configuration options for uploaders.
///
/// This struct contains common configuration options that may be used by different uploaders.
/// Not all fields are used by all uploaders - each uploader uses only the fields it needs.
#[derive(Clone, Debug)]
#[derive(Default)]
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

    /// Check if this uploader is configured and ready to use.
    ///
    /// Returns `true` if the uploader has all necessary credentials and configuration
    /// to perform uploads, `false` otherwise.
    async fn is_ready(&self) -> bool;
}
