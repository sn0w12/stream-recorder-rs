use reqwest::Method;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::io::Read;
use std::time::Duration;

pub const MAX_REQUEST_BODY: usize = 10 * 1024 * 1024;
pub const MAX_RESPONSE_BODY: usize = 5 * 1024 * 1024;

#[derive(Debug)]
pub struct HttpRequest {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub timeout_ms: u64,
}

#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub fn execute(request: HttpRequest) -> Result<HttpResponse, String> {
    if !(request.url.starts_with("http://") || request.url.starts_with("https://")) {
        return Err("URL must start with http:// or https://".to_string());
    }
    if let Some(body) = &request.body
        && body.len() > MAX_REQUEST_BODY
    {
        return Err(format!(
            "request body is {} bytes, maximum is {}",
            body.len(),
            MAX_REQUEST_BODY
        ));
    }

    let method = Method::from_bytes(request.method.as_bytes())
        .map_err(|e| format!("invalid HTTP method '{}': {}", request.method, e))?;

    let mut headers = HeaderMap::new();
    for (name, value) in &request.headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| format!("invalid header name '{}': {}", name, e))?;
        let value = HeaderValue::from_str(value)
            .map_err(|e| format!("invalid header value for '{}': {}", name, e))?;
        headers.insert(name, value);
    }

    let timeout = Duration::from_millis(request.timeout_ms.max(1));
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::limited(3))
        .timeout(timeout)
        .build()
        .map_err(|e| format!("failed to build HTTP client: {}", e))?;

    let mut builder = client.request(method, &request.url).headers(headers);
    if let Some(body) = request.body {
        builder = builder.body(body);
    }

    let response = builder
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status().as_u16();
    let response_headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();

    let mut body = Vec::new();
    response
        .take((MAX_RESPONSE_BODY + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|e| format!("failed to read HTTP response: {}", e))?;

    if body.len() > MAX_RESPONSE_BODY {
        return Err(format!(
            "HTTP response body exceeds the {} byte limit",
            MAX_RESPONSE_BODY
        ));
    }

    Ok(HttpResponse {
        status,
        headers: response_headers,
        body,
    })
}
