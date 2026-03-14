use std::collections::HashMap;
use anyhow::Result;
use crate::stream::monitor::{build_uploaders, try_upload};

pub async fn handle_upload_command(file: String, uploader: Option<String>) -> Result<()> {
    if !std::path::Path::new(&file).is_file() {
        return Err(anyhow::anyhow!("File not found or is not a regular file: {}", file));
    }

    let config = crate::config::Config::load()?;
    let max_retries = config.get_max_upload_retries();
    let uploaders = build_uploaders().await;

    let mut matched = false;
    let mut upload_results: HashMap<String, Vec<String>> = HashMap::new();

    for (up, up_config) in &uploaders {
        if let Some(ref name) = uploader {
            if !up.name().eq_ignore_ascii_case(name) {
                continue;
            }
        }
        matched = true;
        try_upload(up.as_ref(), &file, up_config, &mut upload_results, max_retries).await;
    }

    if !matched {
        if let Some(name) = uploader {
            return Err(anyhow::anyhow!("No uploader named '{}' is configured", name));
        }
        return Err(anyhow::anyhow!("No uploaders are configured"));
    }

    for (name, urls) in &upload_results {
        for url in urls {
            println!("{}: {}", name, url);
        }
    }

    Ok(())
}