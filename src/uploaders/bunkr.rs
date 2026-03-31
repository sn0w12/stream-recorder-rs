use super::error::UploadError;
use super::{UploadResult, Uploader, UploaderConfig};
use async_trait::async_trait;
use bunkr_client::{BunkrUploader as BunkrClient, Config as BunkrConfig};

/// Bunkr uploader wrapper
pub struct BunkrUploader {
    client: Option<BunkrClient>,
}

impl BunkrUploader {
    pub async fn new() -> Self {
        let client = if let Some(token) = crate::utils::get_bunkr_token() {
            BunkrClient::new(token).await.ok()
        } else {
            None
        };
        Self { client }
    }
}

#[async_trait]
impl Uploader for BunkrUploader {
    async fn upload_file(
        &self,
        file_path: &str,
        config: &UploaderConfig,
    ) -> Result<UploadResult, UploadError> {
        let client = self.client.as_ref().ok_or(UploadError {
            message: "Bunkr not configured".to_string(),
            status_code: None,
        })?;
        let bunkr_config = BunkrConfig::default();
        let files = vec![file_path.to_string()];

        // convert optional String to Option<&str> as required by bunkr-client
        let folder_id_opt: Option<&str> = config.folder_id.as_deref();

        // Upload with concurrency=1 since we're uploading a single file
        // If multiple files need to be uploaded, this should be called multiple times
        let (urls, failures) = client
            .upload_files(files, folder_id_opt, 1, None, Some(&bunkr_config))
            .await
            .map_err(|e| UploadError {
                message: e.to_string(),
                status_code: None,
            })?;

        if !failures.is_empty() {
            // Return the first failure as an error
            let failure = &failures[0];
            return Err(UploadError {
                message: failure.error.clone(),
                status_code: failure.status_code,
            });
        }

        // Split comma-separated URLs returned by bunkr-client
        // The bunkr-client library returns URLs as comma-separated strings in a Vec
        // We split them to get individual URLs for consistency with other uploaders
        let urls: Vec<String> = urls
            .into_iter()
            .flat_map(|u| {
                u.split(',')
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .collect();

        Ok(UploadResult {
            urls,
            raw_response: None,
        })
    }

    async fn get_folder_id_by_name(
        &self,
        folder_name: &str,
    ) -> Result<Option<String>, UploadError> {
        let client = self.client.as_ref().ok_or(UploadError {
            message: "Bunkr not configured".to_string(),
            status_code: None,
        })?;
        match client.get_album_by_name(folder_name).await? {
            Some(id) => Ok(Some(id.to_string())),
            None => Err(UploadError {
                message: format!("folder '{}' not found", folder_name),
                status_code: None,
            }),
        }
    }

    fn name(&self) -> &str {
        "bunkr"
    }

    fn max_file_size_mb(&self) -> &u64 {
        &u64::MAX // Bunkr splits videos by itself based on data from its API.
    }

    async fn is_ready(&self) -> bool {
        self.client.is_some()
    }
}
