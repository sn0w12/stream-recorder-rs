use async_trait::async_trait;
use reqwest::Client;
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::Value;
use tokio_util::io::ReaderStream;

use crate::types::FileSize;

use super::error::UploadError;
use super::http::{file_name_from_path, map_io_error, map_reqwest_error, parse_json_response};
use super::{UploadResult, Uploader, UploaderConfig, UploaderKind};

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
        let file_name = file_name_from_path(file_path);

        let file = tokio::fs::File::open(file_path)
            .await
            .map_err(map_io_error)?;
        let file_size = file.metadata().await.map_err(map_io_error)?;

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
            .map_err(map_reqwest_error)?;

        let (status_code, raw_response) = parse_json_response(response).await?;

        parse_upload_response(raw_response, status_code)
    }

    fn name(&self) -> &str {
        "fileditch"
    }

    fn kind(&self) -> UploaderKind {
        UploaderKind::Video
    }

    fn max_file_size(&self) -> FileSize {
        FileSize::from_gb(25)
    }

    async fn is_ready(&self) -> bool {
        true
    }
}
