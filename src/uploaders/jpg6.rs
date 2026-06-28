use async_trait::async_trait;
use reqwest::Client;
use reqwest::header::{COOKIE, SET_COOKIE};
use reqwest::multipart::Form;
use reqwest::redirect::Policy;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next().and_then(|c| (c as char).to_digit(16));
            let lo = chars.next().and_then(|c| (c as char).to_digit(16));
            match hi.zip(lo) {
                Some((h, l)) => result.push(char::from((h * 16 + l) as u8)),
                None => result.push('%'),
            }
        } else {
            result.push(b as char);
        }
    }
    result
}

use crate::stream::messages::send_program_error_webhook;
use crate::types::FileSize;
use crate::utils::jpg6_cookies_path;

use super::error::UploadError;
use super::http::{make_file_part, map_reqwest_error};
use super::{UploadResult, Uploader, UploaderConfig, UploaderKind};

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
struct CookieJar {
    values: HashMap<String, String>,
    #[serde(skip)]
    max_ages: HashMap<String, u64>,
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

            let name = name.trim().to_string();
            if name.is_empty() {
                continue;
            }

            self.values.insert(name.clone(), value.trim().to_string());

            for part in cookie.split(';').skip(1) {
                let part = part.trim();
                if let Some(age_str) = part
                    .strip_prefix("Max-Age=")
                    .or_else(|| part.strip_prefix("max-age="))
                    && let Ok(age) = age_str.trim().parse::<u64>()
                {
                    self.max_ages.insert(name.clone(), age);
                }
            }
        }
    }

    fn min_refresh_interval(&self) -> Duration {
        self.max_ages
            .values()
            .min()
            .map(|&age| Duration::from_secs(age) / 2)
            .unwrap_or(Duration::from_secs(300))
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

    fn save_to_file(&self) {
        let cookies: Vec<serde_json::Value> = self
            .values
            .iter()
            .map(|(name, value)| serde_json::json!({"name": name, "value": value}))
            .collect();
        if let Ok(json) = serde_json::to_string_pretty(&cookies) {
            let _ = std::fs::write(jpg6_cookies_path(), json);
        }
    }

    fn load_from_file() -> Self {
        let path = jpg6_cookies_path();
        if !path.exists() {
            return CookieJar::default();
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            return CookieJar::default();
        };

        #[derive(serde::Deserialize)]
        struct ExportCookie {
            name: String,
            value: String,
        }
        let Ok(cookies) = serde_json::from_str::<Vec<ExportCookie>>(&content) else {
            return CookieJar::default();
        };

        let mut jar = CookieJar::default();
        for c in cookies {
            jar.values.insert(c.name, url_decode(&c.value));
        }
        jar
    }
}

#[derive(Deserialize)]
struct Jpg6Image {
    medium_url: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Jpg6UploadResponse {
    image: Jpg6Image,
}

#[derive(Deserialize)]
struct AuthMeResponse {
    user: Option<Value>,
}

struct CookieJarState {
    jar: CookieJar,
    last_refresh: Instant,
}

pub struct Jpg6Uploader {
    client: Client,
    cookies: Mutex<CookieJarState>,
}

/// Check with the server whether the cookies in the jar represent a valid session.
/// Merges any Set-Cookie headers from the response into the jar.
async fn check_session(client: &Client, jar: &mut CookieJar) -> Result<bool, UploadError> {
    let cookie_header = jar.header_value().unwrap_or_default();
    let xsrf_token = jar.values.get("XSRF-TOKEN").cloned();

    let mut request = client
        .get("https://goonbox.cr/api/auth/me")
        .header(COOKIE, cookie_header)
        .header("Accept", "application/json")
        .header("Referer", "https://goonbox.cr/profile");
    if let Some(token) = &xsrf_token {
        request = request.header("X-XSRF-TOKEN", token);
    }
    let response = request.send().await.map_err(map_reqwest_error)?;

    let status = response.status().as_u16();
    let headers = response.headers().clone();
    jar.merge_headers(&headers);

    let body = response.text().await.map_err(|error| UploadError {
        message: error.to_string(),
        status_code: Some(status),
    })?;

    let parsed: AuthMeResponse = serde_json::from_str(&body).map_err(|error| UploadError {
        message: format!("invalid auth/me response: {}", error),
        status_code: Some(status),
    })?;

    Ok(parsed.user.is_some())
}

impl Jpg6Uploader {
    pub fn new() -> Self {
        let jar = CookieJar::load_from_file();

        Self {
            client: Client::builder()
                .redirect(Policy::none())
                .build()
                .unwrap_or_else(|_| Client::new()),
            cookies: Mutex::new(CookieJarState {
                jar,
                last_refresh: Instant::now(),
            }),
        }
    }

    async fn ensure_fresh_cookies(&self) -> Result<(), UploadError> {
        let needs_refresh = {
            let state = self.cookies.lock().unwrap();
            state.last_refresh.elapsed() >= state.jar.min_refresh_interval()
        };

        if !needs_refresh {
            return Ok(());
        }

        self.refresh_cookies().await
    }

    async fn refresh_cookies(&self) -> Result<(), UploadError> {
        let mut jar = {
            let state = self.cookies.lock().unwrap();
            state.jar.clone()
        };

        let valid = check_session(&self.client, &mut jar).await?;

        {
            let mut state = self.cookies.lock().unwrap();
            state.jar = jar;
            state.jar.save_to_file();
        }

        if !valid {
            send_program_error_webhook(
                None,
                "jpg6 session expired",
                "The jpg6 session cookies have expired. Please provide new cookies by editing the jpg6_cookies.json file in the config directory, then restart the recorder.",
            )
            .await;

            return Err(UploadError {
                message: "jpg6 session expired".to_string(),
                status_code: None,
            });
        }

        {
            let mut state = self.cookies.lock().unwrap();
            state.last_refresh = Instant::now();
        }

        Ok(())
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
        self.ensure_fresh_cookies().await?;

        let (cookie_header, xsrf_token) = {
            let state = self.cookies.lock().unwrap();
            (state.jar.header_value(), state.jar.values.get("XSRF-TOKEN").cloned())
        };

        let file = make_file_part(file_path).await?;
        let form = Form::new().part("file", file);

        let mut upload_request = self.client.post("https://goonbox.cr/api/upload").multipart(form);
        if let Some(ch) = &cookie_header {
            upload_request = upload_request.header(COOKIE, ch);
        }
        if let Some(token) = &xsrf_token {
            upload_request = upload_request.header("X-XSRF-TOKEN", token);
        }
        upload_request = upload_request
            .header("Accept", "application/json")
            .header("Referer", "https://goonbox.cr/upload");

        let upload_response = upload_request.send().await.map_err(map_reqwest_error)?;
        let upload_status = upload_response.status().as_u16();
        let headers = upload_response.headers().clone();

        {
            let mut state = self.cookies.lock().unwrap();
            state.jar.merge_headers(&headers);
            state.jar.save_to_file();
        }

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
        let state = self.cookies.lock().unwrap();
        state.jar.values.contains_key("goonbox_session")
            && state.jar.values.contains_key("XSRF-TOKEN")
    }
}

/// Check if stored jpg6 cookies are valid by making a request to the API.
pub async fn validate_cookies() -> bool {
    let mut jar = CookieJar::load_from_file();
    if jar.values.is_empty() {
        return false;
    }

    let client = Client::builder()
        .redirect(Policy::none())
        .build()
        .unwrap_or_else(|_| Client::new());

    check_session(&client, &mut jar).await.unwrap_or(false)
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

    if !(200..300).contains(&status_code) {
        return Err(UploadError {
            message: format!("jpg6 upload failed with HTTP {}", status_code),
            status_code: Some(status_code),
        });
    }

    let url = parsed.image.medium_url.ok_or_else(|| UploadError {
        message: "jpg6 upload succeeded but no URL was returned".to_string(),
        status_code: Some(status_code),
    })?;

    Ok(UploadResult {
        urls: vec![url],
        raw_response: Some(raw_response),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_upload_response_uses_viewer_url() {
        let raw_response = serde_json::json!({
            "image": {
                "medium_url": "https://goonbox.cr/img/example"
            },
        });

        let result = parse_upload_response(raw_response.clone(), 200).unwrap();
        assert_eq!(result.urls, vec!["https://goonbox.cr/img/example"]);
        assert_eq!(result.raw_response, Some(raw_response));
    }
}
