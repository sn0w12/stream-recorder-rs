use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::types::FileSize;

use super::error::UploadError;
use super::http::{make_file_part, map_reqwest_error, parse_json_response};
use super::{UploadResult, Uploader, UploaderConfig, UploaderKind};

#[derive(Deserialize)]
#[allow(dead_code)]
struct FilesterResponse {
    success: bool,
    message: String,
    slug: String,
    url: String,
    file_id: i32,
    thumbnail_url: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct FilesterFolderListResponse {
    success: bool,
    data: Option<Vec<FilesterFolder>>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct FilesterFolder {
    id: String,
    name: String,
    public: bool,
    file_count: i32,
    created_at: String,
}

/// Filester uploader implementation
pub struct FilesterUploader {
    client: Client,
    api_key: Option<String>,
}

impl FilesterUploader {
    pub fn new() -> Self {
        let api_key = crate::utils::get_filester_token();
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

impl Default for FilesterUploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Uploader for FilesterUploader {
    async fn upload_file(
        &self,
        file_path: &str,
        config: &UploaderConfig,
    ) -> Result<UploadResult, UploadError> {
        let part = make_file_part(file_path).await?;
        let form = reqwest::multipart::Form::new().part("file", part);
        let mut req = self
            .client
            .post("https://u1.filester.me/api/v1/upload")
            .multipart(form);

        if let Some(folder_id) = &config.folder_id {
            req = req.header("X-Folder-ID", folder_id);
        }
        if let Some(api_key) = &self.api_key {
            req = req.header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", api_key),
            );
        }

        let response = req.send().await.map_err(map_reqwest_error)?;
        let (status_code, raw_response) = parse_json_response(response).await?;

        let parsed: FilesterResponse =
            serde_json::from_value(raw_response.clone()).map_err(|e| UploadError {
                message: format!("invalid Filester response: {}", e),
                status_code: Some(status_code),
            })?;

        if !parsed.success {
            return Err(UploadError {
                message: format!(
                    "Filester returned unsuccessful response: {}",
                    parsed.message
                ),
                status_code: Some(status_code),
            });
        }

        let urls = vec![parsed.url];

        Ok(UploadResult {
            urls,
            raw_response: Some(raw_response),
        })
    }

    async fn get_folder_id_by_name(
        &self,
        folder_name: &str,
    ) -> Result<Option<String>, UploadError> {
        let api_key = self.api_key.as_ref().ok_or_else(|| UploadError {
            message: "no filester API key set".to_string(),
            status_code: None,
        })?;

        let resp = self
            .client
            .get("https://u1.filester.me/api/v1/folders")
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", api_key),
            )
            .send()
            .await
            .map_err(map_reqwest_error)?;

        let status_code = resp.status().as_u16();
        let folder_resp: FilesterFolderListResponse =
            resp.json().await.map_err(|e| UploadError {
                message: e.to_string(),
                status_code: Some(status_code),
            })?;

        if !folder_resp.success {
            return Err(UploadError {
                message: "Filester returned unsuccessful response when listing folders".to_string(),
                status_code: Some(status_code),
            });
        }

        if let Some(folders) = folder_resp.data {
            for f in folders {
                if f.name.eq_ignore_ascii_case(folder_name) {
                    return Ok(Some(f.id));
                }
            }
        }

        Err(UploadError {
            message: format!("folder '{}' not found", folder_name),
            status_code: None,
        })
    }

    fn name(&self) -> &str {
        "filester"
    }

    fn kind(&self) -> UploaderKind {
        UploaderKind::Video
    }

    fn max_file_size(&self) -> FileSize {
        FileSize::from_gb(10)
    }

    async fn is_ready(&self) -> bool {
        true
    }
}
