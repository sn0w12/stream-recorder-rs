//! Uploader selection built on the `upio` upload library.
//!
//! Every service, its HTTP client, its credential handling and its response
//! parsing live in `upio`. This module only decides which services run, what
//! they are told about the destination folder, and how their readiness is
//! reported at startup.

use upio::registry::{UploaderId, build_uploader};
use upio::{UploaderCapabilities, UploaderEndpointConfig};

pub use upio::{UploadError, Uploader};

use crate::{config::Config, utils};

/// A service paired with the credentials it should be driven with.
pub type UploaderList = Vec<(upio::BoxedUploader, UploaderEndpointConfig)>;

/// Which pass of a recording session an upload belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UploaderKindFilter {
    All,
    Video,
    Image,
}

impl UploaderKindFilter {
    fn accepts(self, capabilities: UploaderCapabilities) -> bool {
        match self {
            Self::All => true,
            Self::Video => capabilities.video,
            // A service that already receives the recording must not also
            // receive the thumbnail. Both passes write the same
            // `<service>_urls` template key, so the second pass would silently
            // replace the first pass's URLs instead of adding to them.
            Self::Image => capabilities.image && !capabilities.video,
        }
    }
}

/// GoonBox signs in with an account rather than a token, so it needs two
/// separate secrets. Every other service reads `<service>_token`, which is what
/// makes this table-free.
fn endpoint_config(id: UploaderId) -> UploaderEndpointConfig {
    let name = id.name();
    let mut config = UploaderEndpointConfig {
        token: utils::get_token_by_name(&format!("{name}_token")),
        ..Default::default()
    };

    if name == UploaderId::Goonbox.name() {
        config.username = utils::get_token_by_name(crate::consts::GOONBOX_USERNAME_KEY);
        config.password = utils::get_token_by_name(crate::consts::GOONBOX_PASSWORD_KEY);
    }

    config
}

/// Every enabled service that accepts the requested kind of file and has the
/// credentials it needs. A service missing credentials is skipped, because
/// `upio` reports that at construction time rather than on the first upload.
pub fn build_uploaders(filter: UploaderKindFilter) -> UploaderList {
    let disabled: Vec<String> = Config::get()
        .get_disabled_uploaders()
        .into_iter()
        .map(|name| name.to_lowercase())
        .collect();

    let mut uploaders = UploaderList::new();
    for id in UploaderId::ALL {
        if disabled.contains(&id.name().to_lowercase()) {
            continue;
        }
        if !filter.accepts(id.capabilities()) {
            continue;
        }

        let config = endpoint_config(*id);
        match build_uploader(*id, &config) {
            Ok(uploader) => uploaders.push((uploader, config)),
            Err(error) => eprintln!("{} skipped: {}", id.name(), error),
        }
    }

    uploaders
}

/// How a service's configuration will read on the startup banner.
pub enum UploaderStatus {
    Ready(String),
    MissingCredentials(String),
}

/// Every known service with its credential state. Read-only: unlike
/// `build_uploaders` this never builds an uploader, so printing the banner
/// cannot fail and never contacts a service.
pub fn uploader_statuses() -> Vec<(&'static str, UploaderStatus)> {
    UploaderId::ALL
        .iter()
        .map(|id| (id.name(), status_for(*id, &endpoint_config(*id))))
        .collect()
}

fn status_for(id: UploaderId, config: &UploaderEndpointConfig) -> UploaderStatus {
    let has_token = config.token.is_some();
    match id.name() {
        "goonbox" => match (config.username.is_some(), config.password.is_some()) {
            (true, true) => UploaderStatus::Ready("username and password configured".to_string()),
            (true, false) => UploaderStatus::MissingCredentials("password required".to_string()),
            _ => UploaderStatus::MissingCredentials("username and password required".to_string()),
        },
        "gofile" | "filester" if has_token => UploaderStatus::Ready("token configured".to_string()),
        "gofile" | "filester" => UploaderStatus::Ready("no token, public limits".to_string()),
        _ if has_token => UploaderStatus::Ready("token configured".to_string()),
        _ if id == UploaderId::Fileditch => {
            UploaderStatus::Ready("no credentials required".to_string())
        }
        _ => UploaderStatus::MissingCredentials("token required".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use upio::registry::UploaderId;

    #[test]
    fn image_only_services_serve_the_thumbnail_pass() {
        let goonbox = UploaderId::Goonbox.capabilities();
        let bunkr = UploaderId::Bunkr.capabilities();

        assert!(UploaderKindFilter::Image.accepts(goonbox));
        assert!(!UploaderKindFilter::Image.accepts(bunkr));
    }

    #[test]
    fn video_pass_excludes_image_only_services() {
        let goonbox = UploaderId::Goonbox.capabilities();
        let bunkr = UploaderId::Bunkr.capabilities();

        assert!(UploaderKindFilter::Video.accepts(bunkr));
        assert!(!UploaderKindFilter::Video.accepts(goonbox));
    }

    #[test]
    fn the_all_filter_accepts_every_service() {
        for id in UploaderId::ALL {
            assert!(UploaderKindFilter::All.accepts(id.capabilities()));
        }
    }

    #[test]
    fn service_names_are_unique_and_lower_case() {
        let mut names: Vec<&str> = UploaderId::ALL.iter().map(|id| id.name()).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count);
        assert!(
            names
                .iter()
                .all(|name| name.chars().all(|c| !c.is_uppercase()))
        );
    }

    #[test]
    fn a_service_missing_its_token_is_reported_as_unconfigured() {
        let config = UploaderEndpointConfig::default();
        assert!(matches!(
            status_for(UploaderId::Bunkr, &config),
            UploaderStatus::MissingCredentials(_)
        ));
        assert!(matches!(
            status_for(UploaderId::Fileditch, &config),
            UploaderStatus::Ready(_)
        ));
    }

    #[test]
    fn an_optional_token_upgrade_changes_the_reported_note() {
        let anonymous = UploaderEndpointConfig::default();
        assert!(matches!(
            status_for(UploaderId::Gofile, &anonymous),
            UploaderStatus::Ready(note) if note.contains("public limits")
        ));

        let authenticated = UploaderEndpointConfig {
            token: Some("t".to_string()),
            ..Default::default()
        };
        assert!(matches!(
            status_for(UploaderId::Gofile, &authenticated),
            UploaderStatus::Ready(note) if note.contains("token configured")
        ));
    }

    #[test]
    fn goonbox_needs_both_halves_of_the_account() {
        let only_username = UploaderEndpointConfig {
            username: Some("u".to_string()),
            ..Default::default()
        };
        assert!(matches!(
            status_for(UploaderId::Goonbox, &only_username),
            UploaderStatus::MissingCredentials(note) if note.contains("password")
        ));

        let complete = UploaderEndpointConfig {
            username: Some("u".to_string()),
            password: Some("p".to_string()),
            ..Default::default()
        };
        assert!(matches!(
            status_for(UploaderId::Goonbox, &complete),
            UploaderStatus::Ready(_)
        ));
    }

    #[test]
    fn every_known_service_has_a_credential_state() {
        for (name, _) in uploader_statuses() {
            assert!(!name.is_empty());
        }
    }
}
