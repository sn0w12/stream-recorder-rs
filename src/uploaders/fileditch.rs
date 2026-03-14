use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use super::error::UploadError;
use super::{UploadResult, Uploader, UploaderConfig};

#[derive(Deserialize)]
struct FileditchFile {
    url: Option<String>,
}

#[derive(Deserialize)]
struct FileditchResponse {
    success: bool,
    files: Vec<FileditchFile>,
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

        let part = reqwest::multipart::Part::stream(file).file_name(file_name);
        let form = reqwest::multipart::Form::new().part("files[]", part);

        let response = self
            .client
            .post("https://up1.fileditch.com/upload.php")
            .multipart(form)
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

        let urls: Vec<String> = parsed
            .files
            .into_iter()
            .filter_map(|file| file.url)
            .collect();

        if urls.is_empty() {
            return Err(UploadError {
                message: "fileditch upload succeeded but no file URL was returned".to_string(),
                status_code: Some(status_code),
            });
        }

        Ok(UploadResult {
            urls,
            raw_response: Some(raw_response),
        })
    }

    fn name(&self) -> &str {
        "fileditch"
    }

    async fn is_ready(&self) -> bool {
        true
    }
}
