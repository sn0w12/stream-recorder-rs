use anyhow::{Context, Result};
use reqwest::header::ACCEPT_ENCODING;
use semver::Version;
use serde::Deserialize;

const CRATES_IO_VERSIONS_URL: &str =
    "https://crates.io/api/v1/crates/stream-recorder/versions?per_page=1&sort=date";

#[derive(Debug, Clone, Deserialize)]
struct CargoVersionResponse {
    versions: Vec<CargoVersion>,
}

#[derive(Debug, Clone, Deserialize)]
struct CargoVersion {
    num: String,
}

fn parse_latest_version(response_body: &str) -> Result<Version> {
    let response = serde_json::from_str::<CargoVersionResponse>(response_body)
        .context("failed to parse crates.io version response")?;

    let latest = response
        .versions
        .first()
        .context("no versions found for stream-recorder")?;

    Version::parse(&latest.num).context("crates.io returned an invalid version")
}

async fn get_latest_version() -> Result<Version> {
    let client = reqwest::Client::builder()
        .user_agent(format!("stream-recorder/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .context("failed to build crates.io version client")?;

    let response_body = client
        .get(CRATES_IO_VERSIONS_URL)
        .header(ACCEPT_ENCODING, "identity")
        .send()
        .await
        .context("failed to query crates.io for versions")?
        .error_for_status()
        .context("crates.io returned an error status for version check")?
        .text()
        .await
        .context("failed to read crates.io version response body")?;

    parse_latest_version(&response_body)
}

pub enum VersionCheckResult {
    UpToDate,
    Outdated { latest_version: Version },
    Error(String),
}

pub async fn check_version() -> VersionCheckResult {
    let current_version = Version::parse(env!("CARGO_PKG_VERSION"))
        .expect("CARGO_PKG_VERSION is always valid semver");

    match get_latest_version().await {
        Ok(latest_version) => {
            if current_version >= latest_version {
                VersionCheckResult::UpToDate
            } else {
                VersionCheckResult::Outdated { latest_version }
            }
        }
        Err(e) => VersionCheckResult::Error(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_latest_version_from_crates_io_response() {
        let response = r#"{
            "versions": [
                {
                    "id": 2172203,
                    "crate": "stream-recorder",
                    "num": "0.1.3",
                    "features": {},
                    "yanked": false,
                    "published_by": {
                        "id": 367252,
                        "login": "sn0w12"
                    },
                    "audit_actions": [
                        {
                            "action": "publish",
                            "user": {
                                "id": 367252,
                                "login": "sn0w12"
                            },
                            "time": "2026-03-26T19:11:28.963031Z"
                        }
                    ],
                    "linecounts": {
                        "languages": {
                            "Rust": {
                                "code_lines": 5827,
                                "comment_lines": 174,
                                "files": 27
                            }
                        },
                        "total_code_lines": 5827,
                        "total_comment_lines": 174
                    }
                }
            ],
            "meta": {
                "total": 4,
                "next_page": "?per_page=1&sort=date&seek=example"
            }
        }"#;

        let version = parse_latest_version(response).expect("response should parse");

        assert_eq!(version, Version::new(0, 1, 3));
    }

    #[test]
    fn errors_when_no_versions_are_returned() {
        let error = parse_latest_version(r#"{"versions":[],"meta":{"total":0}}"#)
            .expect_err("missing versions should fail");

        assert!(
            error
                .to_string()
                .contains("no versions found for stream-recorder")
        );
    }
}
