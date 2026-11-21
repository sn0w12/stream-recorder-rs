use crate::extension::context::{
    ErrorContext, HookContext, PlatformContext, RecordingContext, SessionContext, StreamContext,
};
use crate::extension::manifest::{Hook, valid_id};
use crate::extension::{self, hooks, install, store};
use anyhow::Result;
use clap::Subcommand;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum ExtensionAction {
    /// Create a new extension scaffold
    #[clap(alias = "create")]
    New {
        /// Extension ID, also used as the folder name
        id: String,
        /// Parent directory for the new folder (defaults to the current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
        /// Human-readable extension name (defaults to the ID)
        #[arg(short, long)]
        name: Option<String>,
    },
    /// List installed extensions
    #[clap(alias = "ls")]
    List,
    /// Install an extension from a remote manifest URL
    Install {
        /// URL to extension.json, or a GitHub repository/tree URL
        url: String,
        /// Skip the confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Update an installed extension from its saved source URL
    Update {
        /// Extension ID to update (omit when using --all)
        extension_id: Option<String>,
        /// Update every installed extension
        #[arg(long, conflicts_with = "extension_id")]
        all: bool,
        /// Skip the confirmation prompt
        #[arg(long)]
        yes: bool,
    },
    /// Remove an installed extension
    #[clap(alias = "rm")]
    Remove {
        /// Extension ID to remove
        extension_id: String,
    },
    /// Run one extension hook with synthetic data
    Debug {
        /// Extension ID
        extension_id: String,
        /// Hook name, for example after_recording
        hook: String,
        /// Recording file to use for recording hooks
        #[arg(long)]
        file: Option<String>,
    },
    /// Show the extension config path and effective settings
    Config {
        /// Extension ID
        extension_id: String,
    },
}

pub async fn handle_extension_command(action: ExtensionAction) -> Result<()> {
    match action {
        ExtensionAction::New { id, path, name } => {
            create_extension(&id, path.as_deref(), name.as_deref())
        }
        ExtensionAction::List => list_extensions(),
        ExtensionAction::Install { url, yes } => {
            let manifest = install::install_from_url(&url, yes).await?;
            println!(
                "Installed extension '{}' ({}). Restart stream-recorder if it is running.",
                manifest.id, manifest.name
            );
            Ok(())
        }
        ExtensionAction::Update {
            extension_id,
            all,
            yes,
        } => {
            if all {
                let results = install::update_all(yes).await?;
                if results.is_empty() {
                    println!("No installed extensions found.");
                }
                for (id, result) in results {
                    match result {
                        Ok(manifest) => println!("Updated '{}' ({}).", id, manifest.name),
                        Err(error) => eprintln!("Failed to update '{}': {}", id, error),
                    }
                }
                println!("Restart stream-recorder if it is running.");
                Ok(())
            } else if let Some(id) = extension_id {
                let manifest = install::update_by_id(&id, yes).await?;
                println!(
                    "Updated '{}' ({}). Restart stream-recorder if it is running.",
                    manifest.id, manifest.name
                );
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Specify an extension ID to update, or pass --all."
                ))
            }
        }
        ExtensionAction::Remove { extension_id } => {
            install::remove_by_id(&extension_id)?;
            println!("Removed extension '{}'.", extension_id);
            Ok(())
        }
        ExtensionAction::Debug {
            extension_id,
            hook,
            file,
        } => debug_extension(&extension_id, &hook, file.as_deref()).await,
        ExtensionAction::Config { extension_id } => show_config(&extension_id),
    }
}

const SR_TYPES: &str = include_str!("../../extensions/sr.d.lua");

const MAIN_LUA: &str = r#"---@type HookModule
local M = {}

---@param ctx HookContext
function M.after_recording(ctx)
  local recording = ctx.recording
  sr.log.info("Recording finished: " .. (recording and recording.file or "unknown"))
  sr.template.set("example", "hello")
end

return M
"#;

const LUARC: &str = r#"{
    "runtime.version": "Lua 5.4",
    "workspace.library": ["./sr.d.lua"]
}
"#;

const GITIGNORE: &str = ".workspace/\nconfig.toml\n";

fn display_name(id: &str) -> String {
    id.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn create_extension(id: &str, parent: Option<&Path>, name: Option<&str>) -> Result<()> {
    if !valid_id(id) {
        return Err(anyhow::anyhow!(
            "Invalid extension id '{}'. Use ^[a-z][a-z0-9_]{{0,63}}$",
            id
        ));
    }

    let parent = parent.unwrap_or_else(|| Path::new("."));
    let dir = parent.join(id);
    if dir.exists() {
        return Err(anyhow::anyhow!(
            "Extension directory already exists: {}",
            dir.display()
        ));
    }

    fs::create_dir_all(&dir)?;

    let display = name
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| display_name(id));

    let manifest = json!({
        "$schema": "https://raw.githubusercontent.com/sn0w12/stream-recorder-rs/refs/heads/main/extensions/schema.json",
        "id": id,
        "name": display,
        "version": "1.0.0",
        "api": 1,
        "stream_recorder_version": "^0.2",
        "main": "main.lua",
        "files": ["main.lua", "sr.d.lua", ".luarc.json"],
        "hooks": ["after_recording"],
        "permissions": [],
        "limits": {
            "timeout_ms": 30000,
            "memory_mb": 64
        }
    });

    fs::write(
        dir.join("extension.json"),
        serde_json::to_string_pretty(&manifest)? + "\n",
    )?;
    fs::write(dir.join("main.lua"), MAIN_LUA)?;
    fs::write(dir.join("sr.d.lua"), SR_TYPES)?;
    fs::write(dir.join(".luarc.json"), LUARC)?;
    fs::write(dir.join(".gitignore"), GITIGNORE)?;

    println!("Created extension '{}' at {}", id, dir.display());
    println!("  extension.json");
    println!("  main.lua");
    println!("  sr.d.lua");
    println!("  .luarc.json");
    println!("  .gitignore");
    println!();
    println!(
        "Copy the folder to {} to install it.",
        extension::extensions_dir().display()
    );
    println!("Then run:");
    println!(
        "  stream-recorder extension debug {} after_recording --file <recording>",
        id
    );

    Ok(())
}

fn list_extensions() -> Result<()> {
    let report = extension::load_report_including_disabled();
    let disabled: Vec<String> = crate::config::Config::get()
        .get_disabled_extensions()
        .into_iter()
        .map(|value| value.to_lowercase())
        .collect();
    if report.extensions.is_empty() {
        println!("No extensions installed.");
        println!("Install one with: extension install <url>");
    } else {
        println!("Installed extensions:");
        for extension in &report.extensions {
            let permissions = if extension.manifest.permissions.is_empty() {
                "none".to_string()
            } else {
                extension
                    .manifest
                    .permissions
                    .iter()
                    .map(|permission| permission.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let update_status = if extension.manifest.source_url.is_some() {
                "updatable"
            } else {
                "no source URL"
            };
            let enabled = if disabled.contains(&extension.manifest.id.to_lowercase()) {
                ", disabled"
            } else {
                ""
            };
            println!(
                "  {} ({}) v{} - {} hooks, permissions: {}, {}{}",
                extension.manifest.id,
                extension.manifest.name,
                extension.manifest.version,
                extension.hooks().join(", "),
                permissions,
                update_status,
                enabled
            );
        }
    }

    if !report.errors.is_empty() {
        eprintln!("\nIgnored invalid extensions:");
        for error in report.errors {
            eprintln!("  - {}", error);
        }
    }

    Ok(())
}

fn show_config(id: &str) -> Result<()> {
    let extension = extension::load_extension_by_id(id).map_err(|e| anyhow::anyhow!(e))?;
    let path = install::config_file(id);
    println!(
        "Extension: {} ({})",
        extension.manifest.id, extension.manifest.name
    );
    println!("Config: {}", path.display());
    if extension.manifest.settings.is_empty() {
        println!("This extension declares no settings.");
        return Ok(());
    }

    println!("\nEffective settings:");
    for (key, def) in &extension.manifest.settings {
        let value = extension
            .settings
            .get(key)
            .map(format_setting_value)
            .unwrap_or_else(|| "<unset>".to_string());
        let description = def.description.as_deref().unwrap_or("");
        if description.is_empty() {
            println!("  {} = {}", key, value);
        } else {
            println!("  {} = {}  ({})", key, value, description);
        }
    }

    if path.exists() {
        println!("\nFile contents:");
        print!("{}", std::fs::read_to_string(&path)?);
    } else {
        println!("\nNo config.toml exists. Create it to override defaults.");
    }

    Ok(())
}

fn format_setting_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn debug_session(id: &str) -> SessionContext {
    SessionContext {
        id: id.to_string(),
        started_at: store::session_started_at(id),
        username: "debug_user".to_string(),
        platform: PlatformContext {
            id: "debug".to_string(),
            name: "Debug".to_string(),
        },
        stream: StreamContext {
            title: Some("Debug stream".to_string()),
            avatar_url: None,
            user_id: Some("debug-user".to_string()),
        },
    }
}

async fn debug_extension(id: &str, hook_name: &str, file: Option<&str>) -> Result<()> {
    let hook = Hook::parse(hook_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown hook '{}'. Expected one of: {}",
            hook_name,
            Hook::ALL
                .iter()
                .map(|hook| hook.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    let extension = extension::load_extension_by_id(id).map_err(|e| anyhow::anyhow!(e))?;
    if !extension.manifest.wants(hook) {
        return Err(anyhow::anyhow!(
            "Extension '{}' does not declare hook '{}'",
            id,
            hook.as_str()
        ));
    }

    let session_id = format!("debug-{}", uuid::Uuid::new_v4());
    store::ensure_session(&session_id);

    let mut context = HookContext::new(&extension.manifest, extension.settings.clone(), hook);
    context.session = Some(debug_session(&session_id));

    let size = file
        .and_then(|file| std::fs::metadata(file).ok())
        .map(|metadata| metadata.len());
    let mut recording = RecordingContext {
        file: file.map(str::to_string),
        size_bytes: size,
        ..RecordingContext::default()
    };

    match hook {
        Hook::BeforeRecording => {
            recording.output_path = Some(file.unwrap_or("debug-recording.mp4").to_string());
            recording.is_continuation = Some(false);
            recording.playback_url = Some("https://example.com/debug.m3u8".to_string());
            context.recording = Some(recording);
        }
        Hook::AfterRecording => {
            recording.duration_seconds = Some(60.0);
            recording.segments = Some(1);
            recording.thumbnail = Some("debug-thumbnail.jpg".to_string());
            context.recording = Some(recording);
        }
        Hook::BeforeUpload => {
            context.recording = Some(recording);
            context.uploads = Some(json!({
                "video": [{"uploader": "debug", "urls": []}],
                "image": [],
            }));
        }
        Hook::AfterUpload => {
            context.recording = Some(recording);
            context.uploads = Some(json!({
                "video": [{"uploader": "debug", "urls": ["https://example.com/video"]}],
                "image": [],
            }));
        }
        Hook::OnError => {
            context.recording = Some(recording);
            context.error = Some(ErrorContext::new("debug", "synthetic error"));
        }
    }

    let session_dir = file.and_then(|file| Path::new(file).parent().map(Path::to_path_buf));

    println!("Running hook '{}' for extension '{}'...", hook.as_str(), id);
    let outcome =
        hooks::run_debug(&extension, hook, context, session_id.clone(), session_dir).await;

    for log in &outcome.logs {
        println!("[{}] {}", log.level, log.message);
    }
    println!("Hook finished in {:.3}s", outcome.duration.as_secs_f64());

    if let Some(error) = &outcome.error {
        eprintln!("Hook failed: {}", error);
    }
    if let Some(abort) = &outcome.aborted {
        println!(
            "Abort requested: {} (notify: {})",
            abort.reason, abort.notify
        );
    }

    let values = store::template_values(&session_id);
    if values.is_empty() {
        println!("Template values: none");
    } else {
        println!(
            "Template values: {}",
            serde_json::to_string_pretty(&values)?
        );
    }

    hooks::cleanup_session(&session_id);
    let _ = std::fs::remove_dir_all(extension.workspace_dir(&session_id));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::manifest::ExtensionManifest;

    #[test]
    fn scaffold_creates_expected_files() {
        let temp = tempfile::tempdir().expect("temp dir");
        create_extension("my_ext", Some(temp.path()), None).expect("scaffold");

        let dir = temp.path().join("my_ext");
        for file in [
            "extension.json",
            "main.lua",
            "sr.d.lua",
            ".luarc.json",
            ".gitignore",
        ] {
            assert!(dir.join(file).is_file(), "missing {}", file);
        }

        let types = fs::read_to_string(dir.join("sr.d.lua")).expect("types");
        assert!(types.contains("---@meta"));

        let manifest = ExtensionManifest::load_from_dir(&dir).expect("manifest");
        assert_eq!(manifest.id, "my_ext");
        assert_eq!(manifest.name, "My Ext");
        assert_eq!(manifest.hooks, vec![Hook::AfterRecording]);
        assert_eq!(
            manifest.files,
            vec![
                "main.lua".to_string(),
                "sr.d.lua".to_string(),
                ".luarc.json".to_string()
            ]
        );
    }

    #[test]
    fn scaffold_rejects_invalid_id() {
        let temp = tempfile::tempdir().expect("temp dir");
        assert!(create_extension("Bad-Id", Some(temp.path()), None).is_err());
        assert!(!temp.path().join("Bad-Id").exists());
    }

    #[test]
    fn scaffold_rejects_existing_directory() {
        let temp = tempfile::tempdir().expect("temp dir");
        fs::create_dir_all(temp.path().join("my_ext")).expect("existing dir");
        assert!(create_extension("my_ext", Some(temp.path()), None).is_err());
    }

    #[test]
    fn scaffold_uses_custom_name() {
        let temp = tempfile::tempdir().expect("temp dir");
        create_extension("my_ext", Some(temp.path()), Some("Custom Name")).expect("scaffold");

        let manifest =
            ExtensionManifest::load_from_dir(&temp.path().join("my_ext")).expect("manifest");
        assert_eq!(manifest.name, "Custom Name");
    }
}
