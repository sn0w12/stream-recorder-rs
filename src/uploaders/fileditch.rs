use async_trait::async_trait;
use reqwest::Client;
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::Value;
use tokio_util::io::ReaderStream;

use super::error::UploadError;
use super::{UploadResult, Uploader, UploaderConfig};

#[derive(Deserialize)]
struct FileditchResponse {
    success: bool,
    url: String,
}

#[derive(Deserialize)]
struct FileditchErrorResponse {
    error: String,
}

fn parse_upload_response(
    raw_response: Value,
    status_code: u16,
) -> Result<UploadResult, UploadError> {
    if let Ok(error_response) =
        serde_json::from_value::<FileditchErrorResponse>(raw_response.clone())
    {
        return Err(UploadError {
            message: error_response.error,
            status_code: Some(status_code),
        });
    }

    let parsed: FileditchResponse =
        serde_json::from_value(raw_response.clone()).map_err(|e| UploadError {
            message: format!("invalid fileditch response: {}", e),
            status_code: Some(status_code),
        })?;

    if !parsed.success {
        return Err(UploadError {
            message: "fileditch returned unsuccessful response".to_string(),
            status_code: Some(status_code),
        });
    }

    if parsed.url.trim().is_empty() {
        return Err(UploadError {
            message: "fileditch upload succeeded but no file URL was returned".to_string(),
            status_code: Some(status_code),
        });
    }

    Ok(UploadResult {
        urls: vec![parsed.url],
        raw_response: Some(raw_response),
    })
}

/// Fileditch uploader implementation
pub struct FileditchUploader {
    client: Client,
}

impl FileditchUploader {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for FileditchUploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Uploader for FileditchUploader {
    async fn upload_file(
        &self,
        file_path: &str,
        _config: &UploaderConfig,
    ) -> Result<UploadResult, UploadError> {
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let file = tokio::fs::File::open(file_path)
            .await
            .map_err(|e| UploadError {
                message: e.to_string(),
                status_code: None,
            })?;
        let file_size = file.metadata().await.map_err(|e| UploadError {
            message: e.to_string(),
            status_code: None,
        })?;

        if file_size.len() == 0 {
            return Err(UploadError {
                message: "empty files are not accepted".to_string(),
                status_code: Some(400),
            });
        }

        let body = reqwest::Body::wrap_stream(ReaderStream::new(file));

        let response = self
            .client
            .post("https://new.fileditch.com/upload.php")
            .query(&[("filename", file_name.as_str())])
            .header(CONTENT_TYPE, "application/octet-stream")
            .header(CONTENT_LENGTH, file_size.len().to_string())
            .body(body)
            .send()
            .await
            .map_err(|e| UploadError {
                message: e.to_string(),
                status_code: e.status().map(|s| s.as_u16()),
            })?;

        let status_code = response.status().as_u16();
        let raw_response = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| UploadError {
                message: e.to_string(),
                status_code: Some(status_code),
            })?;

        parse_upload_response(raw_response, status_code)
    }

    fn name(&self) -> &str {
        "fileditch"
    }

    fn max_file_size_mb(&self) -> &u64 {
        &25000
    }

    async fn is_ready(&self) -> bool {
        true
    }
}
