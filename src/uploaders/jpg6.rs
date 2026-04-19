use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use reqwest::Client;
use reqwest::header::{COOKIE, SET_COOKIE};
use reqwest::multipart::Form;
use reqwest::redirect::Policy;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use crate::types::FileSize;
use crate::utils::{SplitMonitorReferenceError, get_jpg6_token, split_monitor_reference};

use super::error::UploadError;
use super::http::{make_file_part, map_reqwest_error};
use super::{UploadResult, Uploader, UploaderConfig, UploaderKind};

#[derive(Clone, Default)]
struct CookieJar {
    values: HashMap<String, String>,
}

impl CookieJar {
    fn merge_headers(&mut self, headers: &reqwest::header::HeaderMap) {
        for value in headers.get_all(SET_COOKIE).iter() {
            let Ok(cookie) = value.to_str() else {
                continue;
            };

            let Some(cookie_pair) = cookie.split(';').next().map(str::trim) else {
                continue;
            };

            let Some((name, value)) = cookie_pair.split_once('=') else {
                continue;
            };

            if !name.trim().is_empty() {
                self.values
                    .insert(name.trim().to_string(), value.trim().to_string());
            }
        }
    }

    fn header_value(&self) -> Option<String> {
        if self.values.is_empty() {
            return None;
        }

        Some(
            self.values
                .iter()
                .map(|(name, value)| format!("{}={}", name, value))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}

#[derive(Debug)]
struct Jpg6Credentials {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct Jpg6Success {
    message: String,
    code: u16,
}

#[derive(Deserialize)]
struct Jpg6NestedImage {
    url: Option<String>,
}

#[derive(Deserialize)]
struct Jpg6Image {
    display_url: Option<String>,
    medium: Option<Jpg6NestedImage>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Jpg6UploadResponse {
    success: Option<Jpg6Success>,
    status_txt: Option<String>,
    image: Option<Jpg6Image>,
}

fn parse_credentials(token: &str) -> Result<Jpg6Credentials, UploadError> {
    let (username, password) = split_monitor_reference(token).map_err(|error| {
        let message = match error {
            SplitMonitorReferenceError::InvalidFormat => {
                "jpg6 token must use the format USERNAME:PASSWORD"
            }
            SplitMonitorReferenceError::EmptyPlatform => {
                "jpg6 token must include a username before the ':'"
            }
            SplitMonitorReferenceError::EmptyUsername => {
                "jpg6 token must include a password after the ':'"
            }
        };

        UploadError {
            message: message.to_string(),
            status_code: None,
        }
    })?;

    Ok(Jpg6Credentials {
        username: username.to_string(),
        password: password.to_string(),
    })
}

fn extract_upload_auth_token(html: &str) -> Option<String> {
    let regex = Regex::new(r#"PF\.obj\.config\.auth_token\s*=\s*"([^"]+)""#).ok()?;
    regex
        .captures(html)
        .and_then(|captures| captures.get(1))
        .map(|match_| match_.as_str().to_string())
}

fn ensure_success(status_code: u16, context: &str) -> Result<(), UploadError> {
    if status_code >= 400 {
        return Err(UploadError {
            message: format!("{} returned HTTP {}", context, status_code),
            status_code: Some(status_code),
        });
    }

    Ok(())
}

fn parse_upload_response(
    raw_response: Value,
    status_code: u16,
) -> Result<UploadResult, UploadError> {
    let parsed: Jpg6UploadResponse =
        serde_json::from_value(raw_response.clone()).map_err(|error| UploadError {
            message: format!("invalid jpg6 response: {}", error),
            status_code: Some(status_code),
        })?;

    if let Some(success) = parsed.success
        && success.code >= 400 {
            return Err(UploadError {
                message: success.message,
                status_code: Some(status_code),
            });
        }

    if let Some(status_txt) = parsed.status_txt.as_deref()
        && !status_txt.eq_ignore_ascii_case("ok") {
            return Err(UploadError {
                message: format!("jpg6 returned status {}", status_txt),
                status_code: Some(status_code),
            });
        }

    if !(200..300).contains(&status_code) {
        return Err(UploadError {
            message: format!("jpg6 upload failed with HTTP {}", status_code),
            status_code: Some(status_code),
        });
    }

    let url = parsed
        .image
        .and_then(|image| {
            image
                .display_url
                .or(image.medium.and_then(|nested_image| nested_image.url))
        })
        .ok_or_else(|| UploadError {
            message: "jpg6 upload succeeded but no URL was returned".to_string(),
            status_code: Some(status_code),
        })?;

    Ok(UploadResult {
        urls: vec![url],
        raw_response: Some(raw_response),
    })
}

async fn get_page_with_cookies(
    client: &Client,
    url: &str,
    cookies: &CookieJar,
) -> Result<(u16, String, CookieJar), UploadError> {
    let mut request = client.get(url);
    if let Some(cookie_header) = cookies.header_value() {
        request = request.header(COOKIE, cookie_header);
    }

    let response = request.send().await.map_err(map_reqwest_error)?;
    let status_code = response.status().as_u16();
    let mut updated_cookies = cookies.clone();
    updated_cookies.merge_headers(response.headers());
    let body = response.text().await.map_err(|error| UploadError {
        message: error.to_string(),
        status_code: Some(status_code),
    })?;

    Ok((status_code, body, updated_cookies))
}

/// Jpg6 uploader implementation
pub struct Jpg6Uploader {
    client: Client,
    token: Option<String>,
}

impl Jpg6Uploader {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .redirect(Policy::none())
                .build()
                .unwrap_or_else(|_| Client::new()),
            token: get_jpg6_token(),
        }
    }
}

impl Default for Jpg6Uploader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Uploader for Jpg6Uploader {
    async fn upload_file(
        &self,
        file_path: &str,
        _config: &UploaderConfig,
    ) -> Result<UploadResult, UploadError> {
        let credentials = parse_credentials(self.token.as_deref().ok_or_else(|| UploadError {
            message: "jpg6 token not configured".to_string(),
            status_code: None,
        })?)?;

        let mut cookies = CookieJar::default();

        let (status_code, login_html, updated_cookies) =
            get_page_with_cookies(&self.client, "https://jpg6.su/login", &cookies).await?;
        ensure_success(status_code, "loading jpg6 login page")?;
        cookies = updated_cookies;

        let login_token = extract_upload_auth_token(&login_html).ok_or_else(|| UploadError {
            message: "jpg6 login page did not contain an auth_token".to_string(),
            status_code: Some(status_code),
        })?;

        let mut login_request = self.client.post("https://jpg6.su/login");
        if let Some(cookie_header) = cookies.header_value() {
            login_request = login_request.header(COOKIE, cookie_header);
        }

        let login_response = login_request
            .form(&[
                ("login-subject", credentials.username.as_str()),
                ("password", credentials.password.as_str()),
                ("auth_token", login_token.as_str()),
            ])
            .send()
            .await
            .map_err(map_reqwest_error)?;
        let login_status = login_response.status().as_u16();
        cookies.merge_headers(login_response.headers());
        ensure_success(login_status, "logging in to jpg6")?;

        let (status_code, upload_html, updated_cookies) =
            get_page_with_cookies(&self.client, "https://jpg6.su/upload", &cookies).await?;
        ensure_success(status_code, "loading jpg6 upload page")?;
        cookies = updated_cookies;

        let upload_token = extract_upload_auth_token(&upload_html).ok_or_else(|| UploadError {
            message: "jpg6 upload page did not contain PF.obj.config.auth_token".to_string(),
            status_code: Some(status_code),
        })?;

        let file = make_file_part(file_path).await?;
        let timestamp = Utc::now().timestamp_millis().to_string();
        let form = Form::new()
            .part("source", file)
            .text("type", "file")
            .text("action", "upload")
            .text("timestamp", timestamp)
            .text("auth_token", upload_token)
            .text("nsfw", "0");

        let mut upload_request = self.client.post("https://jpg6.su/json").multipart(form);
        if let Some(cookie_header) = cookies.header_value() {
            upload_request = upload_request.header(COOKIE, cookie_header);
        }

        let upload_response = upload_request.send().await.map_err(map_reqwest_error)?;
        let upload_status = upload_response.status().as_u16();
        cookies.merge_headers(upload_response.headers());
        let raw_response = upload_response
            .json::<Value>()
            .await
            .map_err(|error| UploadError {
                message: error.to_string(),
                status_code: Some(upload_status),
            })?;

        parse_upload_response(raw_response, upload_status)
    }

    fn name(&self) -> &str {
        "jpg6"
    }

    fn kind(&self) -> UploaderKind {
        UploaderKind::Image
    }

    fn max_file_size(&self) -> FileSize {
        FileSize::from_mb(15)
    }

    async fn is_ready(&self) -> bool {
        self.token.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_credentials_supports_username_and_password() {
        let credentials = parse_credentials("user;pass").unwrap();
        assert_eq!(credentials.username, "user");
        assert_eq!(credentials.password, "pass");
    }

    #[test]
    fn parse_credentials_keeps_additional_semicolons_in_password() {
        let credentials = parse_credentials("user;pa;ss").unwrap();
        assert_eq!(credentials.username, "user");
        assert_eq!(credentials.password, "pa;ss");
    }

    #[test]
    fn extract_upload_auth_token_finds_script_assignment() {
        let html = r#"<script>PF.obj.config.auth_token = "abc123";</script>"#;
        assert_eq!(extract_upload_auth_token(html).as_deref(), Some("abc123"));
    }

    #[test]
    fn parse_upload_response_uses_viewer_url() {
        let raw_response = serde_json::json!({
            "status_code": 200,
            "success": {"message": "image uploaded", "code": 200},
            "image": {
                "url_viewer": "https://jpg6.su/img/example"
            },
            "status_txt": "OK"
        });

        let result = parse_upload_response(raw_response.clone(), 200).unwrap();
        assert_eq!(result.urls, vec!["https://jpg6.su/img/example"]);
        assert_eq!(result.raw_response, Some(raw_response));
    }
}
