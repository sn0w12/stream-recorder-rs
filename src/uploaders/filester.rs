use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use super::error::UploadError;
use super::{UploadResult, Uploader, UploaderConfig};

#[derive(Deserialize)]
#[allow(dead_code)]
struct FilesterResponse {
    success: bool,
    message: String,
    slug: String,
    file_id: i32,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct FilesterFolder {
    id: i32,
    identifier: String,
    name: String,
    parent_id: i32,
    path: String,
    public: i32,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct FilesterFolderResponse {
    success: bool,
    folders: Option<Vec<FilesterFolder>>,
    hierarchical: Option<Vec<FilesterFolder>>,
}

/// Filester uploader implementation
pub struct FilesterUploader {
    client: Client,
    #[allow(dead_code)]
    token: Option<String>,
}

impl FilesterUploader {
    pub fn new() -> Self {
        let token = crate::utils::get_filester_token();
        Self {
            client: Client::new(),
            token,
        }
    }
}

#[async_trait]
impl Uploader for FilesterUploader {
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
        let form = reqwest::multipart::Form::new().part("file", part);
        let mut req = self
            .client
            .post("https://api.filester.me/upload")
            .multipart(form);

        if let Some(folder_id) = &_config.folder_id {
            req = req.header("X-Folder-ID", folder_id);
        }
        if let Some(token) = &self.token {
            req = req.header(reqwest::header::COOKIE, format!("auth_token={}", token));
        }

        let response = req.send().await.map_err(|e| UploadError {
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

        let parsed: FilesterResponse =
            serde_json::from_value(raw_response.clone()).map_err(|e| UploadError {
                message: format!("invalid Filester response: {}", e),
                status_code: Some(status_code),
            })?;

        if !parsed.success {
            return Err(UploadError {
                message: "Filester returned unsuccessful response".to_string(),
                status_code: Some(status_code),
            });
        }

        let urls: Vec<String> = vec![format!("https://filester.me/d/{}", parsed.slug)];

        if urls.is_empty() {
            return Err(UploadError {
                message: "Filester upload succeeded but no file URL was returned".to_string(),
                status_code: Some(status_code),
            });
        }

        Ok(UploadResult {
            urls,
            raw_response: Some(raw_response),
        })
    }

    async fn get_folder_id_by_name(
        &self,
        folder_name: &str,
    ) -> Result<Option<String>, UploadError> {
        // Token is required for this endpoint
        let token = match &self.token {
            Some(t) => t,
            None => {
                return Err(UploadError {
                    message: "no filester token set".to_string(),
                    status_code: None,
                });
            }
        };

        let resp = self
            .client
            .get("https://filester.me/api/user/folders")
            .header(reqwest::header::COOKIE, format!("auth_token={}", token))
            .send()
            .await
            .map_err(|e| UploadError {
                message: e.to_string(),
                status_code: e.status().map(|s| s.as_u16()),
            })?;

        let status_code = resp.status().as_u16();
        let folder_resp: FilesterFolderResponse = resp.json().await.map_err(|e| UploadError {
            message: e.to_string(),
            status_code: Some(status_code),
        })?;

        if let Some(folders) = folder_resp.folders {
            for f in folders {
                if f.name.eq_ignore_ascii_case(folder_name) {
                    return Ok(Some(f.identifier));
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

    fn max_file_size_mb(&self) -> &u64 {
        &10000
    }

    async fn is_ready(&self) -> bool {
        true
    }
}
