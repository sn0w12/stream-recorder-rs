use crate::extension::context::{
    ErrorContext, HookContext, PlatformContext, RecordingContext, SessionContext, StreamContext,
    uploads_value,
};
use crate::extension::manifest::Hook;
use crate::extension::runtime::{AbortRequest, HookOutcome, HookRequest};
use crate::extension::{self, LoadedExtension, find_hook_extensions, runtime, store};
use crate::stream::types::StreamInfo;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::sync::{Semaphore, oneshot};

static HOOK_PERMITS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(4)));

async fn invoke(request: HookRequest) -> HookOutcome {
    let permit = match HOOK_PERMITS.clone().acquire_owned().await {
        Ok(permit) => permit,
        Err(_) => {
            return HookOutcome {
                logs: Vec::new(),
                error: Some("extension worker semaphore closed".to_string()),
                aborted: None,
                duration: Duration::ZERO,
            };
        }
    };

    let (sender, receiver) = oneshot::channel();
    let spawned = std::thread::Builder::new()
        .name(format!("ext-{}", request.manifest.id))
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let outcome = runtime::execute(request);
            let _ = sender.send(outcome);
            drop(permit);
        });

    match spawned {
        Ok(_) => receiver.await.unwrap_or_else(|_| HookOutcome {
            logs: Vec::new(),
            error: Some("extension worker stopped unexpectedly".to_string()),
            aborted: None,
            duration: Duration::ZERO,
        }),
        Err(error) => HookOutcome {
            logs: Vec::new(),
            error: Some(format!("failed to start extension worker: {}", error)),
            aborted: None,
            duration: Duration::ZERO,
        },
    }
}

fn session_context(session_id: &str, stream_info: &StreamInfo) -> SessionContext {
    SessionContext {
        id: session_id.to_string(),
        started_at: store::session_started_at(session_id),
        username: stream_info.username.clone(),
        platform: PlatformContext {
            id: stream_info.platform.id.clone(),
            name: stream_info.platform.name.clone(),
        },
        stream: StreamContext {
            title: stream_info.extracted.stream_title.clone(),
            avatar_url: stream_info.extracted.avatar_url.clone(),
            user_id: stream_info.extracted.user_id.clone(),
        },
    }
}

fn base_context(
    extension: &LoadedExtension,
    hook: Hook,
    session: Option<SessionContext>,
) -> HookContext {
    let mut context = HookContext::new(&extension.manifest, extension.settings.clone(), hook);
    context.session = session;
    context
}

fn request_for(
    extension: &LoadedExtension,
    hook: Hook,
    context: HookContext,
    session_id: &str,
    session_dir: Option<PathBuf>,
) -> HookRequest {
    HookRequest {
        manifest: extension.manifest.clone(),
        extension_dir: extension.dir.clone(),
        hook,
        context,
        session_id: session_id.to_string(),
        session_dir,
        workspace_dir: extension.workspace_dir(session_id),
        quiet: false,
    }
}

fn parent_dir(path: Option<&str>) -> Option<PathBuf> {
    path.and_then(|path| Path::new(path).parent().map(Path::to_path_buf))
}

pub async fn before_recording(
    session_id: &str,
    stream_info: &StreamInfo,
    output_path: &str,
    is_continuation: bool,
) -> Option<AbortRequest> {
    let session = session_context(session_id, stream_info);
    let session_dir = parent_dir(Some(output_path));

    for extension in find_hook_extensions(Hook::BeforeRecording) {
        let mut context = base_context(&extension, Hook::BeforeRecording, Some(session.clone()));
        context.recording = Some(RecordingContext {
            output_path: Some(output_path.to_string()),
            is_continuation: Some(is_continuation),
            playback_url: Some(stream_info.extracted.playback_url.clone()),
            ..RecordingContext::default()
        });

        let outcome = invoke(request_for(
            &extension,
            Hook::BeforeRecording,
            context,
            session_id,
            session_dir.clone(),
        ))
        .await;

        if let Some(abort) = outcome.aborted {
            let reason = if abort.reason.trim().is_empty() {
                "no reason given".to_string()
            } else {
                abort.reason.clone()
            };
            println!(
                "Extension '{}' skipped recording for {}: {}",
                extension.manifest.id, stream_info.username, reason
            );
            return Some(abort);
        }
    }

    None
}

pub async fn after_recording(
    session_id: &str,
    stream_info: &StreamInfo,
    file: &str,
    thumbnail: &str,
    duration_seconds: f64,
    size_bytes: u64,
    segments: usize,
) {
    let session = session_context(session_id, stream_info);
    let session_dir = parent_dir(Some(file));
    let recording = RecordingContext {
        file: Some(file.to_string()),
        thumbnail: Some(thumbnail.to_string()),
        duration_seconds: Some(duration_seconds),
        size_bytes: Some(size_bytes),
        segments: Some(segments),
        ..RecordingContext::default()
    };

    for extension in find_hook_extensions(Hook::AfterRecording) {
        let mut context = base_context(&extension, Hook::AfterRecording, Some(session.clone()));
        context.recording = Some(recording.clone());
        let _ = invoke(request_for(
            &extension,
            Hook::AfterRecording,
            context,
            session_id,
            session_dir.clone(),
        ))
        .await;
    }
}

pub async fn before_upload(
    session_id: &str,
    stream_info: &StreamInfo,
    recording: &RecordingContext,
    video: Vec<String>,
    image: Vec<String>,
) {
    let session = session_context(session_id, stream_info);
    let session_dir = parent_dir(recording.file.as_deref());
    let uploads = json!({
        "video": video.iter().map(|name| json!({"uploader": name, "urls": []})).collect::<Vec<Value>>(),
        "image": image.iter().map(|name| json!({"uploader": name, "urls": []})).collect::<Vec<Value>>(),
    });

    for extension in find_hook_extensions(Hook::BeforeUpload) {
        let mut context = base_context(&extension, Hook::BeforeUpload, Some(session.clone()));
        context.recording = Some(recording.clone());
        context.uploads = Some(uploads.clone());
        let _ = invoke(request_for(
            &extension,
            Hook::BeforeUpload,
            context,
            session_id,
            session_dir.clone(),
        ))
        .await;
    }
}

pub async fn after_upload(
    session_id: &str,
    stream_info: &StreamInfo,
    recording: &RecordingContext,
    video_results: &HashMap<String, Vec<String>>,
    image_results: &HashMap<String, Vec<String>>,
) {
    let session = session_context(session_id, stream_info);
    let session_dir = parent_dir(recording.file.as_deref());
    let uploads = uploads_value(video_results, image_results);

    for extension in find_hook_extensions(Hook::AfterUpload) {
        let mut context = base_context(&extension, Hook::AfterUpload, Some(session.clone()));
        context.recording = Some(recording.clone());
        context.uploads = Some(uploads.clone());
        let _ = invoke(request_for(
            &extension,
            Hook::AfterUpload,
            context,
            session_id,
            session_dir.clone(),
        ))
        .await;
    }
}

pub async fn on_error(
    session_id: Option<&str>,
    stream_info: Option<&StreamInfo>,
    error: ErrorContext,
    recording: Option<RecordingContext>,
) {
    let extensions = find_hook_extensions(Hook::OnError);
    if extensions.is_empty() {
        return;
    }

    let (session_id, transient) = match session_id {
        Some(id) => (id.to_string(), false),
        None => (format!("error-{}", uuid::Uuid::new_v4()), true),
    };
    store::ensure_session(&session_id);

    let session = stream_info.map(|info| session_context(&session_id, info));
    let session_dir = recording
        .as_ref()
        .and_then(|recording| parent_dir(recording.file.as_deref()));

    for extension in extensions {
        let mut context = base_context(&extension, Hook::OnError, session.clone());
        context.error = Some(error.clone());
        context.recording = recording.clone();
        let _ = invoke(request_for(
            &extension,
            Hook::OnError,
            context,
            &session_id,
            session_dir.clone(),
        ))
        .await;
    }

    if transient {
        extension::cleanup_session(&session_id);
    }
}

pub async fn run_debug(
    extension: &LoadedExtension,
    hook: Hook,
    context: HookContext,
    session_id: String,
    session_dir: Option<PathBuf>,
) -> HookOutcome {
    store::ensure_session(&session_id);
    let request = HookRequest {
        manifest: extension.manifest.clone(),
        extension_dir: extension.dir.clone(),
        hook,
        context,
        session_id: session_id.clone(),
        session_dir,
        workspace_dir: extension.workspace_dir(&session_id),
        quiet: true,
    };
    invoke(request).await
}

pub fn cleanup_session(session_id: &str) {
    extension::cleanup_session(session_id);
}
