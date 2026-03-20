use crate::{
    discord::webhook::{
        Component, ContainerComponent, DiscordColor, DividerComponent, GroupComponent, Media,
        MediaComponent, TextComponent, WebhookClient,
    },
    stream::monitor::StreamInfo,
};

fn stream_header_component(username: &str, avatar_url: Option<String>, title: &str) -> Component {
    Component::Group(GroupComponent {
        components: vec![
            Component::Text(TextComponent {
                content: username.into(),
            }),
            Component::Text(TextComponent {
                content: format!("**{}**", title),
            }),
        ],
        accessory: Some(Box::new(Component::Media(MediaComponent {
            media: Media {
                url: avatar_url.unwrap_or_default(),
            },
            description: None,
            spoiler: false,
        }))),
    })
}

/// Sends a Discord webhook notification for recording start.
pub async fn send_recording_start_webhook(
    webhook_url: Option<&str>,
    username: &str,
    avatar_url: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client = WebhookClient::new(webhook_url.unwrap_or_default());
    let component = Component::Container(ContainerComponent {
        accent_color: DiscordColor::rgb(255, 255, 0),
        spoiler: false,
        components: vec![stream_header_component(
            username,
            avatar_url,
            "Stream Recording Started",
        )],
    });
    client
        .send_to_thread(username, None, Some(vec![component]), None)
        .await?;
    Ok(())
}

/// Sends a Discord webhook notification for recorded stream completion.
pub async fn send_recording_complete_webhook(
    webhook_url: Option<&str>,
    stream_info: &StreamInfo,
    duration_str: &str,
    size_str: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client = WebhookClient::new(webhook_url.unwrap_or_default());
    let component = Component::Container(ContainerComponent {
        accent_color: DiscordColor::rgb(0, 255, 0),
        spoiler: false,
        components: vec![
            stream_header_component(
                &stream_info.username,
                stream_info.avatar_url.clone(),
                "Stream Recording Completed",
            ),
            Component::Divider(DividerComponent {
                visible: true,
                spacing: 1,
            }),
            Component::Text(TextComponent {
                content: format!("**Title**\n{}", stream_info.stream_title.clone()),
            }),
            Component::Text(TextComponent {
                content: format!("**Duration**\n{}", duration_str),
            }),
            Component::Text(TextComponent {
                content: format!("**Size**\n{}", size_str),
            }),
        ],
    });
    client
        .send_to_thread(&stream_info.username, None, Some(vec![component]), None)
        .await?;
    Ok(())
}

pub async fn send_template_webhook(
    webhook_url: Option<&str>,
    stream_info: &StreamInfo,
    message: &str,
    attachment: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client = WebhookClient::new(webhook_url.unwrap_or_default());
    let component = Component::Container(ContainerComponent {
        accent_color: DiscordColor::rgb(0, 0, 255),
        spoiler: false,
        components: vec![
            stream_header_component(
                &stream_info.username,
                stream_info.avatar_url.clone(),
                "Stream Processing Completed",
            ),
            Component::Text(TextComponent {
                content: message.into(),
            }),
        ],
    });

    let part = reqwest::multipart::Part::file(&attachment)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    let filename = std::path::Path::new(&attachment)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("attachment")
        .to_string();

    client
        .send_to_thread(
            &stream_info.username,
            None,
            Some(vec![component]),
            Some(vec![(filename, part)]),
        )
        .await?;
    Ok(())
}
