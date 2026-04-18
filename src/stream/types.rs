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
    fn update_optional_field(
        target: &mut Option<String>,
        incoming: Option<String>,
        field_name: &'static str,
        updated_fields: &mut Vec<&'static str>,
    ) {
        if let Some(value) = incoming
            && target.as_ref() != Some(&value)
        {
            *target = Some(value);
            updated_fields.push(field_name);
        }
    }

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

    /// Refresh extracted values using the latest pipeline output.
    ///
    /// Missing optional values are ignored so we do not overwrite previously
    /// captured metadata with absent fields from a partial pipeline response.
    /// Does not refresh the playback_url.
    pub fn refresh_from_pipeline(&mut self, vars: &HashMap<String, String>) -> Vec<&'static str> {
        let mut updated_fields = Vec::new();

        Self::update_optional_field(
            &mut self.extracted.stream_title,
            vars.get("stream_title")
                .map(|title| self.platform.clean_title(title)),
            "stream_title",
            &mut updated_fields,
        );
        Self::update_optional_field(
            &mut self.extracted.avatar_url,
            vars.get("avatar_url").cloned(),
            "avatar_url",
            &mut updated_fields,
        );

        updated_fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{LiveCheck, PipelineStep};

    fn test_platform() -> PlatformConfig {
        PlatformConfig {
            id: "test-platform".to_string(),
            name: "Test Platform".to_string(),
            base_url: "https://example.com/".to_string(),
            icon: None,
            token_name: None,
            headers: HashMap::new(),
            steps: vec![PipelineStep {
                endpoint: "/stream".to_string(),
                live_check: Some(LiveCheck::Path("data.live".to_string())),
                extract: HashMap::new(),
            }],
            source_url: None,
            version: "1.0.0".to_string(),
            stream_recorder_version: None,
            title_clean_regex: None,
        }
    }

    #[test]
    fn refresh_from_pipeline_updates_changed_fields() {
        let platform = test_platform();
        let mut initial = HashMap::new();
        initial.insert(
            "playback_url".to_string(),
            "https://example.com/live.m3u8".to_string(),
        );
        initial.insert("stream_title".to_string(), "Original Title".to_string());
        initial.insert(
            "avatar_url".to_string(),
            "https://example.com/original.jpg".to_string(),
        );

        let mut stream_info =
            StreamInfo::from_pipeline("example_user", &platform, &initial).expect("stream info");

        let mut refreshed = HashMap::new();
        refreshed.insert(
            "playback_url".to_string(),
            "https://example.com/live-updated.m3u8".to_string(),
        );
        refreshed.insert("stream_title".to_string(), "Updated Title".to_string());
        refreshed.insert(
            "avatar_url".to_string(),
            "https://example.com/updated.jpg".to_string(),
        );

        let updated_fields = stream_info.refresh_from_pipeline(&refreshed);

        assert_eq!(updated_fields, vec!["stream_title", "avatar_url"]);
        assert_eq!(
            stream_info.extracted.playback_url,
            "https://example.com/live.m3u8"
        );
        assert_eq!(
            stream_info.extracted.stream_title.as_deref(),
            Some("Updated Title")
        );
        assert_eq!(
            stream_info.extracted.avatar_url.as_deref(),
            Some("https://example.com/updated.jpg")
        );
    }

    #[test]
    fn refresh_from_pipeline_preserves_existing_optional_fields_when_missing() {
        let platform = test_platform();
        let mut initial = HashMap::new();
        initial.insert(
            "playback_url".to_string(),
            "https://example.com/live.m3u8".to_string(),
        );
        initial.insert("stream_title".to_string(), "Original Title".to_string());
        initial.insert(
            "avatar_url".to_string(),
            "https://example.com/original.jpg".to_string(),
        );

        let mut stream_info =
            StreamInfo::from_pipeline("example_user", &platform, &initial).expect("stream info");

        let mut refreshed = HashMap::new();
        refreshed.insert(
            "playback_url".to_string(),
            "https://example.com/live.m3u8".to_string(),
        );

        let updated_fields = stream_info.refresh_from_pipeline(&refreshed);

        assert!(updated_fields.is_empty());
        assert_eq!(
            stream_info.extracted.stream_title.as_deref(),
            Some("Original Title")
        );
        assert_eq!(
            stream_info.extracted.avatar_url.as_deref(),
            Some("https://example.com/original.jpg")
        );
    }
}
