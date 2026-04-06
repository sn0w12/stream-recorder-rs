use crate::platform::PlatformConfig;
use std::collections::HashMap;

/// Context struct holding initial information about a recording session.
#[derive(Clone)]
pub struct StreamInfo {
    pub username: String,
    pub platform: PlatformConfig,
    pub extracted: ExtractedStreamValues,
}

/// Contains values extracted from the platform pipeline.
#[derive(Clone)]
pub struct ExtractedStreamValues {
    pub playback_url: String,
    pub stream_title: Option<String>,
    pub avatar_url: Option<String>,
}

impl StreamInfo {
    pub fn from_pipeline(
        username: &str,
        platform: &PlatformConfig,
        vars: &HashMap<String, String>,
    ) -> Option<Self> {
        let playback_url = vars.get("playback_url")?.clone();

        Some(Self {
            username: username.to_string(),
            platform: platform.clone(),
            extracted: ExtractedStreamValues {
                playback_url,
                stream_title: vars
                    .get("stream_title")
                    .map(|title| platform.clean_title(title)),
                avatar_url: vars.get("avatar_url").cloned(),
            },
        })
    }
}
