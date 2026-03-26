use crate::{
    discord::webhook::{
        Component, ContainerComponent, DiscordColor, DividerComponent, GroupComponent, Identity,
        ImageComponent, Media, MediaComponent, MediaGalleryItem, TextComponent, WebhookClient,
    },
    stream::monitor::StreamInfo,
};
use anyhow::Result;

fn sanitize_avatar_url(avatar_url: Option<String>) -> Option<String> {
    avatar_url.and_then(|url| {
        let trimmed = url.trim();
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            Some(trimmed.to_string())
        } else {
            None
        }
    })
}

fn accessory_media_component(avatar_url: Option<String>) -> Option<Box<Component>> {
    sanitize_avatar_url(avatar_url).map(|url| {
        Box::new(Component::Media(MediaComponent {
            media: Media { url },
            description: None,
            spoiler: false,
        }))
    })
}

fn stream_header_components(
    username: &str,
    avatar_url: Option<String>,
    title: &str,
) -> Vec<Component> {
    let header_text = vec![
        Component::Text(TextComponent {
            content: username.into(),
        }),
        Component::Text(TextComponent {
            content: format!("**{}**", title),
        }),
    ];

    if let Some(accessory) = accessory_media_component(avatar_url) {
        vec![Component::Group(GroupComponent {
            components: header_text,
            accessory,
        })]
    } else {
        header_text
    }
}

fn platform_to_identity(stream_info: &StreamInfo) -> Identity {
    Identity {
        username: stream_info.platform.name.clone(),
        avatar_url: sanitize_avatar_url(stream_info.platform.icon.clone()),
    }
}

/// Sends a Discord webhook notification for recording start.
pub async fn send_recording_start_webhook(
    webhook_url: Option<&str>,
    stream_info: &StreamInfo,
) -> Result<()> {
    let mut client = WebhookClient::new(webhook_url.unwrap_or_default());
    let component = Component::Container(ContainerComponent {
        accent_color: DiscordColor::rgb(255, 255, 0),
        spoiler: false,
        components: stream_header_components(
            &stream_info.username,
            stream_info.avatar_url.clone(),
            "Stream Recording Started",
        ),
    });
    let identity = platform_to_identity(stream_info);

    client
        .send_to_thread(
            &stream_info.username,
            None,
            Some(vec![component]),
            None,
            identity,
        )
        .await?;
    Ok(())
}

/// Sends a Discord webhook notification for recorded stream completion.
pub async fn send_recording_complete_webhook(
    webhook_url: Option<&str>,
    stream_info: &StreamInfo,
    duration_str: &str,
    size_str: &str,
) -> Result<()> {
    let mut client = WebhookClient::new(webhook_url.unwrap_or_default());
    let component = Component::Container(ContainerComponent {
        accent_color: DiscordColor::rgb(0, 255, 0),
        spoiler: false,
        components: {
            let mut components = stream_header_components(
                &stream_info.username,
                stream_info.avatar_url.clone(),
                "Stream Recording Completed",
            );
            components.push(Component::Divider(DividerComponent {
                visible: true,
                spacing: 1,
            }));
            components.push(Component::Text(TextComponent {
                content: format!("**Title**\n{}", stream_info.stream_title.clone()),
            }));
            components.push(Component::Text(TextComponent {
                content: format!("**Duration**\n{}", duration_str),
            }));
            components.push(Component::Text(TextComponent {
                content: format!("**Size**\n{}", size_str),
            }));
            components
        },
    });
    let identity = platform_to_identity(stream_info);

    client
        .send_to_thread(
            &stream_info.username,
            None,
            Some(vec![component]),
            None,
            identity,
        )
        .await?;
    Ok(())
}

pub async fn send_template_webhook(
    webhook_url: Option<&str>,
    stream_info: &StreamInfo,
    message: &str,
    attachment: String,
) -> Result<()> {
    let mut client = WebhookClient::new(webhook_url.unwrap_or_default());
    let header_component = Component::Container(ContainerComponent {
        accent_color: DiscordColor::rgb(0, 0, 255),
        spoiler: false,
        components: stream_header_components(
            &stream_info.username,
            stream_info.avatar_url.clone(),
            "Stream Processing Completed",
        ),
    });
    let message_component = Component::Text(TextComponent {
        content: message.into(),
    });

    let part = reqwest::multipart::Part::file(&attachment).await?;
    let filename = std::path::Path::new(&attachment)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("attachment")
        .to_string();

    let image_component = Component::Image(ImageComponent {
        items: vec![MediaGalleryItem {
            media: Media {
                url: format!("attachment://{}", filename),
            },
            description: None,
            spoiler: false,
        }],
    });
    let identity = platform_to_identity(stream_info);

    client
        .send_to_thread(
            &stream_info.username,
            None,
            Some(vec![header_component, message_component, image_component]),
            Some(vec![(filename, part)]),
            identity,
        )
        .await?;
    Ok(())
}

pub async fn send_minimum_duration_webhook(
    webhook_url: Option<&str>,
    stream_info: &StreamInfo,
) -> Result<()> {
    let mut client = WebhookClient::new(webhook_url.unwrap_or_default());
    let header_component = Component::Container(ContainerComponent {
        accent_color: DiscordColor::rgb(255, 0, 0),
        spoiler: false,
        components: stream_header_components(
            &stream_info.username,
            stream_info.avatar_url.clone(),
            "Stream Too Short",
        ),
    });
    let identity = platform_to_identity(stream_info);

    client
        .send_to_thread(
            &stream_info.username,
            None,
            Some(vec![header_component]),
            None,
            identity,
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_header_components_use_plain_text_when_avatar_missing() {
        let components = stream_header_components("alice", None, "Started");

        assert_eq!(components.len(), 2);
        assert!(matches!(components[0], Component::Text(_)));
        assert!(matches!(components[1], Component::Text(_)));
    }

    #[test]
    fn sanitize_avatar_url_rejects_non_http_urls() {
        assert_eq!(
            sanitize_avatar_url(Some("ftp://example.com/avatar.png".into())),
            None
        );
        assert_eq!(
            sanitize_avatar_url(Some("https://example.com/avatar.png".into())),
            Some("https://example.com/avatar.png".into())
        );
    }
}
