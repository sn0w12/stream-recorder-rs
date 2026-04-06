pub mod api;
pub mod encoding;
pub mod messages;
pub mod monitor;
pub mod postprocess;
pub mod recording;
pub mod types;

pub(crate) type StreamResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
