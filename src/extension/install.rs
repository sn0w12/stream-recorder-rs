use crate::extension::manifest::{
    ExtensionManifest, MAX_SCRIPT_BYTES, Permission, validate_relative,
};
use anyhow::Result;
use reqwest::Client;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_FILE_BYTES: usize = 10 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 25 * 1024 * 1024;

pub fn resolve_manifest_url(url: &str) -> Result<String> {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(anyhow::anyhow!("Extension URL must not be empty"));
    }

    if let Some(index) = trimmed.find("github.com/") {
        let path = &trimmed[index + "github.com/".len()..];
        let segments: Vec<&str> = path.split('/').collect();
        if segments.len() >= 2 && !segments[0].is_empty() && !segments[1].is_empty() {
            let owner = segments[0];
            let repo = segments[1];
            if segments.len() == 2 {
                return Ok(format!(
                    "https://raw.githubusercontent.com/{}/{}/HEAD/extension.json",
                    owner, repo
                ));
            }
            if segments[2] == "tree" && segments.len() >= 4 {
                let branch = segments[3];
                let subpath = segments[4..].join("/");
                let base = if subpath.is_empty() {
                    format!(
                        "https://raw.githubusercontent.com/{}/{}/{}",
                        owner, repo, branch
                    )
                } else {
                    format!(
                        "https://raw.githubusercontent.com/{}/{}/{}/{}",
                        owner, repo, branch, subpath
                    )
                };
                return Ok(format!("{}/extension.json", base.trim_end_matches('/')));
            }
            if segments[2] == "blob" && segments.len() >= 4 {
                let branch = segments[3];
                let file = segments[4..].join("/");
                return Ok(format!(
                    "https://raw.githubusercontent.com/{}/{}/{}/{}",
                    owner, repo, branch, file
                ));
            }
        }
    }

    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        if trimmed.ends_with(".json") {
            return Ok(trimmed.to_string());
        }
        return Ok(format!("{}/extension.json", trimmed));
    }

    Err(anyhow::anyhow!(
        "Extension URL must be a JSON URL or a GitHub repository/tree URL, got '{}'",
        url
    ))
}

fn base_url(manifest_url: &str) -> Result<String> {
    manifest_url
        .rsplit_once('/')
        .map(|(base, _)| format!("{}/", base))
        .ok_or_else(|| anyhow::anyhow!("Invalid manifest URL '{}'", manifest_url))
}

fn encode_path(path: &str) -> String {
    let mut encoded = String::new();
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

fn join_url(base: &str, path: &str) -> String {
    format!("{}{}", base, encode_path(path))
}

fn client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))
}

async fn fetch_bytes(client: &Client, url: &str, max: usize) -> Result<Vec<u8>> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to download '{}': {}", url, e))?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to download '{}': HTTP {}",
            url,
            response.status()
        ));
    }

    if let Some(length) = response.content_length()
        && length > max as u64
    {
        return Err(anyhow::anyhow!(
            "Download from '{}' is {} bytes, maximum is {}",
            url,
            length,
            max
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", url, e))?;

    if bytes.len() > max {
        return Err(anyhow::anyhow!(
            "Download from '{}' is {} bytes, maximum is {}",
            url,
            bytes.len(),
            max
        ));
    }

    Ok(bytes.to_vec())
}

fn write_relative(root: &Path, relative: &str, bytes: &[u8]) -> Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, bytes).map_err(|e| anyhow::anyhow!("Failed to write {:?}: {}", path, e))
}

fn copy_dir(source: &Path, destination: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name();
        let target = destination.join(&name);
        if file_type.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

fn permission_lines(manifest: &ExtensionManifest) -> Vec<String> {
    manifest
        .permissions
        .iter()
        .map(|permission| format!("  {:<14} {}", permission.as_str(), permission.description()))
        .collect()
}

fn print_summary(manifest: &ExtensionManifest, old: Option<&ExtensionManifest>) {
    println!(
        "Extension '{}' ({}) v{}",
        manifest.id, manifest.name, manifest.version
    );
    if let Some(description) = &manifest.description {
        println!("{}", description);
    }
    println!(
        "Hooks: {}",
        manifest
            .hooks
            .iter()
            .map(|hook| hook.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    if manifest.permissions.is_empty() {
        println!("Permissions: none");
    } else {
        println!("Permissions:");
        for line in permission_lines(manifest) {
            println!("{}", line);
        }
    }

    if let Some(old) = old {
        let added: Vec<&Permission> = manifest
            .permissions
            .iter()
            .filter(|permission| !old.permissions.contains(permission))
            .collect();
        if !added.is_empty() {
            println!(
                "This update adds permissions: {}",
                added
                    .iter()
                    .map(|permission| permission.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    if manifest.permissions.contains(&Permission::Process) {
        println!();
        println!("WARNING: process extensions run programs as your user.");
        println!("They can read files your user can read, including .env and the system keyring.");
        println!("Only install extensions you trust.");
    }
}

fn confirm() -> Result<()> {
    print!("Continue? [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim().to_lowercase();
    if answer == "y" || answer == "yes" {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Installation cancelled"))
    }
}

pub async fn install_from_url(url: &str, yes: bool) -> Result<ExtensionManifest> {
    install_from_url_inner(url, yes, None).await
}

async fn install_from_url_inner(
    url: &str,
    yes: bool,
    expected_id: Option<&str>,
) -> Result<ExtensionManifest> {
    let manifest_url = resolve_manifest_url(url)?;
    let client = client()?;
    let bytes = fetch_bytes(&client, &manifest_url, MAX_MANIFEST_BYTES).await?;
    let content = String::from_utf8(bytes)
        .map_err(|e| anyhow::anyhow!("Manifest is not valid UTF-8: {}", e))?;

    let mut manifest: ExtensionManifest = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Downloaded file is not a valid extension manifest: {}", e))?;
    manifest.validate(&manifest_url)?;
    if let Some(expected_id) = expected_id
        && manifest.id != expected_id
    {
        return Err(anyhow::anyhow!(
            "Updated manifest declares id '{}' but expected '{}'",
            manifest.id,
            expected_id
        ));
    }
    manifest.source_url = Some(url.to_string());

    let target = crate::extension::extension_dir(&manifest.id);
    let previous = ExtensionManifest::load_from_dir(&target).ok();
    print_summary(&manifest, previous.as_ref());
    if !yes {
        confirm()?;
    }

    let base = base_url(&manifest_url)?;
    let parent = crate::extension::extensions_dir();
    fs::create_dir_all(&parent)?;
    let temp = parent.join(format!(".tmp-{}-{}", manifest.id, uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp)?;

    let result = async {
        let mut total = 0_usize;
        for file in manifest.files_to_install() {
            validate_relative(&file, &manifest_url, "`files`")?;
            let url = join_url(&base, &file);
            let bytes = fetch_bytes(&client, &url, MAX_FILE_BYTES).await?;
            if file == manifest.main && bytes.len() as u64 > MAX_SCRIPT_BYTES {
                return Err(anyhow::anyhow!(
                    "Main script '{}' is {} bytes, maximum is {}",
                    file,
                    bytes.len(),
                    MAX_SCRIPT_BYTES
                ));
            }
            total = total.saturating_add(bytes.len());
            if total > MAX_TOTAL_BYTES {
                return Err(anyhow::anyhow!(
                    "Extension download exceeds the {} byte limit",
                    MAX_TOTAL_BYTES
                ));
            }
            write_relative(&temp, &file, &bytes)?;
        }

        let serialized = serde_json::to_string_pretty(&manifest)?;
        fs::write(temp.join("extension.json"), serialized)?;

        if target.exists() {
            let config = target.join("config.toml");
            if config.exists() {
                fs::copy(&config, temp.join("config.toml"))?;
            }
            let workspace = target.join(".workspace");
            if workspace.exists() {
                copy_dir(&workspace, &temp.join(".workspace"))?;
            }
        }

        let backup = parent.join(format!(".old-{}-{}", manifest.id, uuid::Uuid::new_v4()));
        let had_previous = target.exists();
        if had_previous {
            fs::rename(&target, &backup)?;
        }

        if let Err(error) = fs::rename(&temp, &target) {
            if had_previous {
                let _ = fs::rename(&backup, &target);
            }
            return Err(anyhow::anyhow!(
                "Failed to install extension into {:?}: {}",
                target,
                error
            ));
        }

        if had_previous {
            let _ = fs::remove_dir_all(&backup);
        }

        Ok(())
    }
    .await;

    if let Err(error) = result {
        let _ = fs::remove_dir_all(&temp);
        return Err(error);
    }

    Ok(manifest)
}

pub async fn update_by_id(id: &str, yes: bool) -> Result<ExtensionManifest> {
    let dir = crate::extension::extension_dir(id);
    if !dir.is_dir() {
        return Err(anyhow::anyhow!(
            "Extension '{}' is not installed (expected {:?})",
            id,
            dir
        ));
    }
    let manifest =
        ExtensionManifest::load_from_dir(&dir).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let source_url = manifest.source_url.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "Extension '{}' has no source URL saved. Re-install it with: extension install <url>",
            id
        )
    })?;
    install_from_url_inner(&source_url, yes, Some(id)).await
}

fn installed_ids() -> Result<Vec<String>> {
    let dir = crate::extension::extensions_dir();
    let mut ids = Vec::new();
    if !dir.exists() {
        return Ok(ids);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with('.') || !path.is_dir() {
            continue;
        }
        if path.join("extension.json").is_file() {
            ids.push(name.to_string());
        }
    }
    ids.sort();
    Ok(ids)
}

pub async fn update_all(yes: bool) -> Result<Vec<(String, Result<ExtensionManifest>)>> {
    let mut results = Vec::new();
    for id in installed_ids()? {
        let result = update_by_id(&id, yes).await;
        results.push((id, result));
    }
    Ok(results)
}

pub fn remove_by_id(id: &str) -> Result<()> {
    if !crate::extension::manifest::valid_id(id) {
        return Err(anyhow::anyhow!("Invalid extension id '{}'", id));
    }
    let dir = crate::extension::extension_dir(id);
    if !dir.exists() {
        return Err(anyhow::anyhow!(
            "Extension '{}' is not installed (expected {:?})",
            id,
            dir
        ));
    }
    fs::remove_dir_all(&dir)
        .map_err(|e| anyhow::anyhow!("Failed to remove extension {:?}: {}", dir, e))?;
    Ok(())
}

pub fn config_file(id: &str) -> PathBuf {
    crate::extension::config_path(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_github_repo_urls() {
        assert_eq!(
            resolve_manifest_url("https://github.com/owner/repo").unwrap(),
            "https://raw.githubusercontent.com/owner/repo/HEAD/extension.json"
        );
        assert_eq!(
            resolve_manifest_url("https://github.com/owner/repo/tree/main/ext").unwrap(),
            "https://raw.githubusercontent.com/owner/repo/main/ext/extension.json"
        );
    }

    #[test]
    fn resolves_direct_json_urls() {
        assert_eq!(
            resolve_manifest_url("https://example.com/ext/extension.json").unwrap(),
            "https://example.com/ext/extension.json"
        );
        assert_eq!(
            resolve_manifest_url("https://example.com/ext").unwrap(),
            "https://example.com/ext/extension.json"
        );
    }

    #[test]
    fn rejects_unknown_urls() {
        assert!(resolve_manifest_url("not-a-url").is_err());
    }

    #[test]
    fn encodes_paths_for_urls() {
        assert_eq!(encode_path("assets/my file.png"), "assets/my%20file.png");
    }
}
