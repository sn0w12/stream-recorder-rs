use super::error::UploadError;
use super::{UploadResult, Uploader, UploaderConfig};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tokio::fs::File;

fn response_handler(response: &Value) -> Result<Value, UploadError> {
    if response["status"] == "ok" {
        Ok(response["data"].clone())
    } else if let Some(status) = response["status"].as_str() {
        if status.contains("error-") {
            let error = status.split('-').nth(1).unwrap_or("unknown").to_string();
            Err(UploadError {
                message: error,
                status_code: None,
            })
        } else {
            let status_str = status.to_string();
            Err(UploadError {
                message: format!("unexpected status: {}", status_str),
                status_code: None,
            })
        }
    } else {
        Err(UploadError {
            message: "invalid response".to_string(),
            status_code: None,
        })
    }
}

#[derive(serde::Deserialize, Clone)]
pub struct Server {
    pub name: String,
    pub zone: String,
}

#[derive(serde::Deserialize)]
struct ServersResponse {
    #[serde(rename = "serversAllZone")]
    servers_all_zone: Vec<Server>,
}

pub async fn get_server(zone: &str) -> Result<Server, UploadError> {
    let resp = reqwest::get("https://api.gofile.io/servers")
        .await
        .map_err(|e| UploadError {
            message: e.to_string(),
            status_code: e.status().map(|s| s.as_u16()),
        })?;
    let status_code = resp.status().as_u16();
    let json: Value = resp.json().await.map_err(|e| UploadError {
        message: e.to_string(),
        status_code: Some(status_code),
    })?;
    let data = response_handler(&json)?;
    let servers_resp: ServersResponse = serde_json::from_value(data).map_err(|e| UploadError {
        message: e.to_string(),
        status_code: None,
    })?;
    let servers = servers_resp.servers_all_zone;
    let available_servers: Vec<Server> =
        servers.iter().filter(|s| s.zone == zone).cloned().collect();
    if !available_servers.is_empty() {
        Ok(available_servers[0].clone())
    } else if !servers.is_empty() {
        Ok(servers[0].clone())
    } else {
        Err(UploadError {
            message: "noServersAvailable".to_string(),
            status_code: None,
        })
    }
}

pub async fn upload_file(
    file_path: &str,
    token: Option<&str>,
    folder_id: Option<&str>,
    server: Option<&str>,
) -> Result<Value, UploadError> {
    let server = if let Some(s) = server {
        s.to_string()
    } else {
        get_server("eu").await?.name
    };
    let client = Client::new();
    let file = File::open(file_path).await.map_err(|e| UploadError {
        message: e.to_string(),
        status_code: None,
    })?;
    let part = reqwest::multipart::Part::stream(file).file_name(
        std::path::Path::new(file_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    );
    let mut form = reqwest::multipart::Form::new().part("file", part);
    if let Some(fid) = folder_id {
        form = form.text("folderId", fid.to_string());
    }
    let mut request = client
        .post(format!("https://{}.gofile.io/uploadFile", server))
        .multipart(form);
    if let Some(t) = token {
        request = request.header("Authorization", format!("Bearer {}", t));
    }
    let resp = request.send().await.map_err(|e| UploadError {
        message: e.to_string(),
        status_code: e.status().map(|s| s.as_u16()),
    })?;
    let status_code = resp.status().as_u16();
    let json: Value = resp.json().await.map_err(|e| UploadError {
        message: e.to_string(),
        status_code: Some(status_code),
    })?;
    response_handler(&json)
}

/// GoFile uploader implementation
pub struct GoFileUploader;

impl GoFileUploader {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoFileUploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Uploader for GoFileUploader {
    async fn upload_file(
        &self,
        file_path: &str,
        config: &UploaderConfig,
    ) -> Result<UploadResult, UploadError> {
        let response = upload_file(
            file_path,
            config.token.as_deref(),
            config.folder_id.as_deref(),
            config.server.as_deref(),
        )
        .await?;

        let urls =
            if let Some(download_page) = response.get("downloadPage").and_then(|v| v.as_str()) {
                vec![download_page.to_string()]
            } else {
                vec![]
            };

        Ok(UploadResult {
            urls,
            raw_response: Some(response),
        })
    }

    fn name(&self) -> &str {
        "gofile"
    }

    fn max_file_size_mb(&self) -> &u64 {
        &u64::MAX
    }

    async fn is_ready(&self) -> bool {
        true // GoFile doesn't require authentication for basic uploads
    }
}
