use anyhow::Context;
use reqwest::Client;
use reqwest::multipart::{Form, Part};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Value, json};

use crate::discord::threads::ThreadStore;

const IS_COMPONENTS_V2: u64 = 1 << 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscordColor(u32);

impl DiscordColor {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
    }
}

// Serialize as plain u32
impl Serialize for DiscordColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.0)
    }
}

#[derive(Debug, Clone)]
pub enum Component {
    Container(ContainerComponent),

    Group(GroupComponent),

    Text(TextComponent),

    Media(MediaComponent),

    Image(ImageComponent),

    Divider(DividerComponent),
}

impl Component {
    fn component_type(&self) -> u8 {
        match self {
            Self::Container(_) => 17,
            Self::Group(_) => 9,
            Self::Text(_) => 10,
            Self::Media(_) => 11,
            Self::Image(_) => 12,
            Self::Divider(_) => 14,
        }
    }
}

impl Serialize for Component {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("type", &self.component_type())?;

        match self {
            Self::Container(component) => {
                map.serialize_entry("accent_color", &component.accent_color)?;
                map.serialize_entry("spoiler", &component.spoiler)?;
                map.serialize_entry("components", &component.components)?;
            }
            Self::Group(component) => {
                map.serialize_entry("components", &component.components)?;
                if let Some(accessory) = &component.accessory {
                    map.serialize_entry("accessory", accessory)?;
                }
            }
            Self::Text(component) => {
                map.serialize_entry("content", &component.content)?;
            }
            Self::Media(component) => {
                map.serialize_entry("media", &component.media)?;
                if let Some(description) = &component.description {
                    map.serialize_entry("description", description)?;
                }
                map.serialize_entry("spoiler", &component.spoiler)?;
            }
            Self::Image(component) => {
                map.serialize_entry("items", &component.items)?;
            }
            Self::Divider(component) => {
                map.serialize_entry("divider", &component.visible)?;
                map.serialize_entry("spacing", &component.spacing)?;
            }
        }

        map.end()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerComponent {
    pub accent_color: DiscordColor,
    pub spoiler: bool,
    pub components: Vec<Component>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupComponent {
    pub components: Vec<Component>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessory: Option<Box<Component>>, // typically a Media component
}

#[derive(Debug, Clone, Serialize)]
pub struct TextComponent {
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DividerComponent {
    #[serde(rename = "divider")]
    pub visible: bool, // serialized as `divider` to match Discord's API
    pub spacing: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaComponent {
    pub media: Media,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub spoiler: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageComponent {
    pub items: Vec<MediaGalleryItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaGalleryItem {
    pub media: Media,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub spoiler: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Media {
    pub url: String,
}

/// Response from Discord when we ask to wait for the message.
#[derive(Debug, Deserialize)]
pub struct Message {
    pub channel_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct AttachmentMetadata {
    pub id: u64,
    pub filename: String,
}

/// Options for executing a webhook.
#[derive(Debug, Default)]
pub struct ExecuteWebhookOptions {
    pub content: Option<String>,
    pub components: Option<Vec<Component>>,
    pub files: Option<Vec<(String, Part)>>,
    pub thread_id: Option<u64>,
    pub thread_name: Option<String>,
    pub identity: Option<Identity>,
}

#[derive(Debug)]
pub struct Identity {
    pub username: String,
    pub avatar_url: Option<String>,
}

pub struct WebhookClient {
    http: Client,
    // `None` when no webhook URL was provided; client becomes a no-op.
    base_url: Option<String>,
    store: ThreadStore,
}

fn append_query_param(url: &mut String, key: &str, value: &str) {
    if url.contains('?') {
        url.push('&');
    } else {
        url.push('?');
    }
    url.push_str(key);
    url.push('=');
    url.push_str(value);
}

fn add_components_v2_payload(payload: &mut Value, components: &[Component]) -> anyhow::Result<()> {
    payload["components"] = serde_json::to_value(components)?;
    payload["flags"] = json!(IS_COMPONENTS_V2);
    Ok(())
}

fn add_attachment_payload(payload: &mut Value, files: &[(String, Part)]) -> anyhow::Result<()> {
    let attachments = files
        .iter()
        .enumerate()
        .map(|(idx, (filename, _))| AttachmentMetadata {
            id: idx as u64,
            filename: filename.clone(),
        })
        .collect::<Vec<_>>();

    payload["attachments"] = serde_json::to_value(attachments)?;
    Ok(())
}

fn multipart_file_field_name(index: usize) -> String {
    format!("files[{index}]")
}

impl WebhookClient {
    /// Create a new client from a full Discord webhook URL.
    /// Example: "https://discord.com/api/webhooks/123456/abc-def"
    pub fn new(webhook_url: &str) -> Self {
        Self {
            http: Client::new(),
            base_url: if webhook_url.trim().is_empty() {
                None
            } else {
                Some(webhook_url.trim_end_matches('/').to_string())
            },
            store: ThreadStore::load(),
        }
    }

    /// Execute the webhook with the given options. returns the created `Message`.
    async fn execute(&self, options: ExecuteWebhookOptions) -> anyhow::Result<Option<Message>> {
        // If no base URL is configured, skip executing webhooks.
        let mut url = match &self.base_url {
            Some(u) => u.clone(),
            None => return Ok(None),
        };

        if let Some(tid) = options.thread_id {
            append_query_param(&mut url, "thread_id", &tid.to_string());
        }
        append_query_param(&mut url, "wait", "true");
        if options.components.is_some() {
            append_query_param(&mut url, "with_components", "true");
        }

        // Build the JSON payload (without files)
        let mut payload = json!({});
        if let Some(identity) = &options.identity {
            payload["username"] = Value::String(identity.username.clone());
            if let Some(avatar_url) = &identity.avatar_url {
                payload["avatar_url"] = Value::String(avatar_url.clone());
            }
        }
        if let Some(content) = &options.content {
            payload["content"] = Value::String(content.clone());
        }
        if let Some(components) = &options.components {
            add_components_v2_payload(&mut payload, components)?;
        }
        if let Some(files) = &options.files {
            add_attachment_payload(&mut payload, files)?;
        }
        if let Some(thread_name) = &options.thread_name {
            payload["thread_name"] = Value::String(thread_name.clone());
        }

        // Decide whether to use multipart (if files are present)
        let response = if let Some(files) = options.files {
            // Multipart request
            let mut form = Form::new();
            // Add the JSON part
            let json_str = serde_json::to_string(&payload)?;
            form = form.part("payload_json", Part::text(json_str));

            // Add each file part with the field names Discord expects.
            for (idx, (filename, part)) in files.into_iter().enumerate() {
                let field_name = multipart_file_field_name(idx);
                form = form.part(field_name, part.file_name(filename));
            }

            self.http.post(&url).multipart(form).send().await?
        } else {
            // JSON only
            self.http.post(&url).json(&payload).send().await?
        };

        // Check for HTTP errors
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Discord API error {status}: {text}");
        }

        let message = response.json::<Message>().await?;
        Ok(Some(message))
    }

    /// Send a message to a thread identified by name.
    /// If the thread does not exist, it is created in the forum channel
    /// and its ID is stored for future use.
    pub async fn send_to_thread(
        &mut self,
        thread_name: &str,
        content: Option<String>,
        components: Option<Vec<Component>>,
        files: Option<Vec<(String, Part)>>,
        identity: Identity,
    ) -> anyhow::Result<()> {
        // No-op when webhook URL not configured.
        if self.base_url.is_none() {
            return Ok(());
        }

        let mut opts = ExecuteWebhookOptions {
            content,
            components,
            files,
            identity: Some(identity),
            ..Default::default()
        };

        if let Some(thread_id) = self.store.get(thread_name) {
            // Thread exists – send message to it
            opts.thread_id = Some(thread_id);
            self.execute(opts).await?;
        } else {
            // Thread does not exist – create it and capture the new thread ID
            opts.thread_name = Some(thread_name.to_string());
            let maybe_msg = self.execute(opts).await?;
            let msg =
                maybe_msg.context("discord webhook response did not include a thread message")?;
            let new_thread_id = msg
                .channel_id
                .parse::<u64>()
                .context("discord webhook response returned an invalid thread ID")?;
            self.store.insert(thread_name.to_string(), new_thread_id)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discord_color_rgb_and_serialize() {
        let c = DiscordColor::rgb(1, 2, 3);
        let serialized = serde_json::to_string(&c).unwrap();
        assert_eq!(serialized, "66051"); // 0x010203 == 66051

        let red = DiscordColor::rgb(255, 0, 0);
        let red_ser = serde_json::to_string(&red).unwrap();
        assert_eq!(red_ser, "16711680"); // 0xFF0000 == 16711680
    }

    #[test]
    fn append_query_param_appends_multiple_parameters() {
        let mut url = "https://discord.com/api/webhooks/1/2".to_string();
        append_query_param(&mut url, "wait", "true");
        append_query_param(&mut url, "with_components", "true");

        assert_eq!(
            url,
            "https://discord.com/api/webhooks/1/2?wait=true&with_components=true"
        );
    }

    #[test]
    fn add_components_v2_payload_sets_components_and_flags() {
        let mut payload = json!({});
        let components = vec![Component::Text(TextComponent {
            content: "hello".to_string(),
        })];

        add_components_v2_payload(&mut payload, &components).expect("payload update");

        assert_eq!(payload["flags"], json!(IS_COMPONENTS_V2));
        assert_eq!(payload["components"][0]["type"], json!(10));
        assert_eq!(payload["components"][0]["content"], "hello");
    }

    #[test]
    fn add_attachment_payload_sets_attachment_metadata() {
        let mut payload = json!({});
        let files = vec![
            (
                "thumb.png".to_string(),
                Part::text("thumbnail-bytes").file_name("ignored.png"),
            ),
            (
                "info.txt".to_string(),
                Part::text("details").file_name("ignored.txt"),
            ),
        ];

        add_attachment_payload(&mut payload, &files).expect("attachment payload update");

        assert_eq!(
            payload["attachments"],
            json!([
                { "id": 0, "filename": "thumb.png" },
                { "id": 1, "filename": "info.txt" }
            ])
        );
    }

    #[test]
    fn image_component_serializes_as_media_gallery() {
        let component = Component::Image(ImageComponent {
            items: vec![MediaGalleryItem {
                media: Media {
                    url: "attachment://thumb.png".to_string(),
                },
                description: None,
                spoiler: false,
            }],
        });

        let serialized = serde_json::to_value(&component).expect("component serialization");

        assert_eq!(serialized["type"], json!(12));
        assert_eq!(
            serialized["items"][0]["media"]["url"],
            "attachment://thumb.png"
        );
    }

    #[test]
    fn multipart_file_field_name_matches_discord_format() {
        assert_eq!(multipart_file_field_name(0), "files[0]");
        assert_eq!(multipart_file_field_name(1), "files[1]");
    }

    #[test]
    fn message_deserializes_string_channel_id() {
        let message: Message = serde_json::from_value(json!({
            "channel_id": "123456789012345678"
        }))
        .expect("message deserialization");

        assert_eq!(message.channel_id, "123456789012345678");
    }
}
