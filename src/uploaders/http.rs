use reqwest::multipart::Part;
use serde_json::Value;

use super::error::UploadError;

/// Map a reqwest error to an UploadError, extracting the status code if available.
pub fn map_reqwest_error(error: reqwest::Error) -> UploadError {
    UploadError {
        message: error.to_string(),
        status_code: error.status().map(|status| status.as_u16()),
    }
}

/// Map a std::io::Error to an UploadError.
pub fn map_io_error(error: std::io::Error) -> UploadError {
    UploadError {
        message: error.to_string(),
        status_code: None,
    }
}

/// Extract the file name from a file path.
pub fn file_name_from_path(file_path: &str) -> String {
    std::path::Path::new(file_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

/// Create a multipart file part from a file path.
pub async fn make_file_part(file_path: &str) -> Result<Part, UploadError> {
    let file = tokio::fs::File::open(file_path)
        .await
        .map_err(map_io_error)?;
    Ok(Part::stream(file).file_name(file_name_from_path(file_path)))
}

/// Parse a JSON response from reqwest, returning the status code and the parsed JSON value.
pub async fn parse_json_response(response: reqwest::Response) -> Result<(u16, Value), UploadError> {
    let status_code = response.status().as_u16();
    let json = response
        .json::<Value>()
        .await
        .map_err(|error| UploadError {
            message: error.to_string(),
            status_code: Some(status_code),
        })?;
    Ok((status_code, json))
}
