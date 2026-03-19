use reqwest::Client;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};

use crate::discord::threads::ThreadStore;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum Component {
    #[serde(rename = "17")]
    Container(ContainerComponent),

    #[serde(rename = "9")]
    Group(GroupComponent),

    #[serde(rename = "10")]
    Text(TextComponent),

    #[serde(rename = "11")]
    Media(MediaComponent),

    #[serde(rename = "14")]
    Divider(DividerComponent),
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerComponent {
    pub accent_color: u32,
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
    pub divider: bool, // always true, just to differentiate from other components
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
pub struct Media {
    pub url: String,
}

/// Response from Discord when we ask to wait for the message.
#[derive(Debug, Deserialize)]
pub struct Message {
    pub channel_id: u64, // this is the thread ID when a thread is created
}

/// Options for executing a webhook.
#[derive(Debug, Default)]
pub struct ExecuteWebhookOptions {
    pub content: Option<String>,
    pub components: Option<Vec<Component>>,
    pub files: Option<Vec<(String, Part)>>,
    pub thread_id: Option<u64>,
    pub thread_name: Option<String>,
}

pub struct WebhookClient {
    http: Client,
    // `None` when no webhook URL was provided; client becomes a no-op.
    base_url: Option<String>,
    store: ThreadStore,
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
            url.push_str(&format!("?thread_id={}", tid));
        }
        if url.contains('?') {
            url.push_str("&wait=true");
        } else {
            url.push_str("?wait=true");
        }

        // Build the JSON payload (without files)
        let mut payload = serde_json::json!({});
        if let Some(content) = &options.content {
            payload["content"] = serde_json::Value::String(content.clone());
        }
        if let Some(components) = &options.components {
            payload["components"] = serde_json::to_value(components)?;
        }
        if let Some(thread_name) = &options.thread_name {
            payload["thread_name"] = serde_json::Value::String(thread_name.clone());
        }

        // Decide whether to use multipart (if files are present)
        let response = if let Some(files) = options.files {
            // Multipart request
            let mut form = Form::new();
            // Add the JSON part
            let json_str = serde_json::to_string(&payload)?;
            form = form.part("payload_json", Part::text(json_str));

            // Add each file part with part name "files[index]"
            for (idx, (_filename, part)) in files.into_iter().enumerate() {
                form = form.part(format!("files[{}]", idx), part);
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
            anyhow::bail!("Discord API error {}: {}", status, text);
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
    ) -> anyhow::Result<()> {
        // No-op when webhook URL not configured.
        if self.base_url.is_none() {
            return Ok(());
        }

        let mut opts = ExecuteWebhookOptions {
            content,
            components,
            files,
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
            let msg = maybe_msg.expect("wait=true should return a message");
            let new_thread_id = msg.channel_id;
            self.store.insert(thread_name.to_string(), new_thread_id)?;
        }
        Ok(())
    }
}
