use crate::extension::context::HookContext;
use crate::extension::http;
use crate::extension::manifest::{ExtensionManifest, Hook, MAX_SCRIPT_BYTES, Permission};
use crate::extension::process;
use crate::extension::store;
use crate::extension::store::MAX_VALUE_DEPTH;
use mlua::{
    Function, HookTriggers, Lua, LuaOptions, LuaSerdeExt, StdLib, Table, Value as LuaValue, VmState,
};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const MAX_LOG_ENTRIES: usize = 200;
pub const MAX_LOG_BYTES: usize = 64 * 1024;
pub const MAX_LOG_MESSAGE_BYTES: usize = 8 * 1024;
pub const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024;
pub const MAX_TEXT_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct AbortRequest {
    pub reason: String,
    pub notify: bool,
}

#[derive(Debug)]
pub struct HookOutcome {
    pub logs: Vec<LogEntry>,
    pub error: Option<String>,
    pub aborted: Option<AbortRequest>,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct HookRequest {
    pub manifest: ExtensionManifest,
    pub extension_dir: PathBuf,
    pub hook: Hook,
    pub context: HookContext,
    pub session_id: String,
    pub session_dir: Option<PathBuf>,
    pub workspace_dir: PathBuf,
    pub quiet: bool,
}

#[derive(Debug, Default)]
struct LogState {
    entries: Vec<LogEntry>,
    bytes: usize,
    overflowed: bool,
}

impl LogState {
    fn push(&mut self, level: &str, message: &str) {
        if self.overflowed {
            return;
        }
        let message = truncate(message, MAX_LOG_MESSAGE_BYTES);
        if self.entries.len() >= MAX_LOG_ENTRIES || self.bytes + message.len() > MAX_LOG_BYTES {
            self.overflowed = true;
            return;
        }
        self.bytes += message.len();
        self.entries.push(LogEntry {
            level: level.to_string(),
            message,
        });
    }
}

fn truncate(value: &str, max: usize) -> String {
    if value.len() <= max {
        return value.to_string();
    }
    let mut end = max;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &value[..end])
}

fn log_message(
    level: &str,
    extension_id: &str,
    logs: &Arc<Mutex<LogState>>,
    quiet: bool,
    message: String,
) {
    let message = truncate(&message, MAX_LOG_MESSAGE_BYTES);
    if !quiet {
        match level {
            "warn" => eprintln!("[ext:{}] warn: {}", extension_id, message),
            "error" => eprintln!("[ext:{}] error: {}", extension_id, message),
            _ => println!("[ext:{}] {}", extension_id, message),
        }
    }
    if let Ok(mut state) = logs.lock() {
        state.push(level, &message);
    }
}

fn runtime_error(message: impl Into<String>) -> mlua::Error {
    mlua::Error::RuntimeError(message.into())
}

fn remaining_ms(deadline: Instant) -> Option<u64> {
    let now = Instant::now();
    if now >= deadline {
        return None;
    }
    Some(
        deadline
            .duration_since(now)
            .as_millis()
            .min(u128::from(u64::MAX)) as u64,
    )
}

fn validate_lua_value(value: &LuaValue, depth: usize) -> Result<(), String> {
    match value {
        LuaValue::Nil | LuaValue::Boolean(_) | LuaValue::Integer(_) | LuaValue::Number(_) => Ok(()),
        LuaValue::String(text) => {
            if text.as_bytes().len() > store::MAX_TEMPLATE_VALUE_BYTES {
                Err("string value is too large".to_string())
            } else {
                Ok(())
            }
        }
        LuaValue::Table(table) => {
            if depth >= MAX_VALUE_DEPTH {
                return Err(format!("value nesting exceeds {} levels", MAX_VALUE_DEPTH));
            }
            let mut count = 0;
            for pair in table.clone().pairs::<LuaValue, LuaValue>() {
                let (key, value) = pair.map_err(|e| format!("cannot read table value: {}", e))?;
                count += 1;
                if count > 256 {
                    return Err("table has too many entries".to_string());
                }
                match key {
                    LuaValue::String(_) | LuaValue::Integer(_) => {}
                    _ => {
                        return Err("table keys must be strings or integers".to_string());
                    }
                }
                validate_lua_value(&value, depth + 1)?;
            }
            Ok(())
        }
        _ => Err("only nil, booleans, numbers, strings, and tables can be used".to_string()),
    }
}

fn lua_to_json(lua: &Lua, value: LuaValue) -> Result<Value, String> {
    validate_lua_value(&value, 0)?;
    lua.from_value(value)
        .map_err(|e| format!("value cannot be converted to JSON: {}", e))
}

#[derive(Debug)]
struct PathPolicy {
    extension_dir: PathBuf,
    workspace_dir: PathBuf,
    session_dir: Option<PathBuf>,
    session_read: bool,
}

impl PathPolicy {
    fn new(
        extension_dir: &Path,
        workspace_dir: &Path,
        session_dir: Option<&Path>,
        session_read: bool,
    ) -> Result<Self, String> {
        fs::create_dir_all(workspace_dir)
            .map_err(|e| format!("failed to create workspace {:?}: {}", workspace_dir, e))?;

        let extension_dir = fs::canonicalize(extension_dir)
            .map_err(|e| format!("failed to resolve extension directory: {}", e))?;
        let workspace_dir = fs::canonicalize(workspace_dir)
            .map_err(|e| format!("failed to resolve workspace directory: {}", e))?;
        let session_dir = session_dir.and_then(|dir| fs::canonicalize(dir).ok());

        Ok(Self {
            extension_dir,
            workspace_dir,
            session_dir,
            session_read,
        })
    }

    fn read_roots(&self) -> Vec<&Path> {
        let mut roots = vec![self.extension_dir.as_path(), self.workspace_dir.as_path()];
        if self.session_read
            && let Some(dir) = &self.session_dir
        {
            roots.push(dir.as_path());
        }
        roots
    }

    fn is_allowed(&self, path: &Path, roots: &[&Path]) -> bool {
        roots.iter().any(|root| path.starts_with(root))
    }

    fn resolve_read(&self, raw: &str) -> Result<PathBuf, String> {
        reject_root_relative(raw)?;
        let roots = self.read_roots();
        let candidate = if Path::new(raw).is_absolute() {
            PathBuf::from(raw)
        } else {
            let mut found = None;
            for root in &roots {
                let candidate = root.join(raw);
                if candidate.exists() {
                    found = Some(candidate);
                    break;
                }
            }
            found.ok_or_else(|| format!("file not found in an allowed root: {}", raw))?
        };

        let canonical = fs::canonicalize(&candidate)
            .map_err(|e| format!("failed to resolve {:?}: {}", candidate, e))?;
        if !self.is_allowed(&canonical, &roots) {
            return Err(format!("path is outside the allowed roots: {}", raw));
        }
        Ok(canonical)
    }

    fn resolve_write(&self, raw: &str) -> Result<PathBuf, String> {
        reject_root_relative(raw)?;
        let path = Path::new(raw);
        if path.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err("path must not contain '..'".to_string());
        }

        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_dir.join(path)
        };

        let parent = candidate
            .parent()
            .ok_or_else(|| "path has no parent directory".to_string())?;
        fs::create_dir_all(parent).map_err(|e| format!("failed to create {:?}: {}", parent, e))?;
        let parent = fs::canonicalize(parent)
            .map_err(|e| format!("failed to resolve {:?}: {}", parent, e))?;

        if !parent.starts_with(&self.workspace_dir) {
            return Err("writes are only allowed inside the extension workspace".to_string());
        }

        let file_name = candidate
            .file_name()
            .ok_or_else(|| "path has no file name".to_string())?;
        let target = parent.join(file_name);
        if target.exists() {
            let canonical = fs::canonicalize(&target)
                .map_err(|e| format!("failed to resolve {:?}: {}", target, e))?;
            if !canonical.starts_with(&self.workspace_dir) {
                return Err("writes are only allowed inside the extension workspace".to_string());
            }
            return Ok(canonical);
        }
        Ok(target)
    }

    fn resolve_dir(&self, raw: &str) -> Result<PathBuf, String> {
        reject_root_relative(raw)?;
        let candidate = if Path::new(raw).is_absolute() {
            PathBuf::from(raw)
        } else {
            self.workspace_dir.join(raw)
        };
        let canonical = fs::canonicalize(&candidate)
            .map_err(|e| format!("failed to resolve directory {:?}: {}", candidate, e))?;
        if canonical.starts_with(&self.extension_dir) || canonical.starts_with(&self.workspace_dir)
        {
            Ok(canonical)
        } else {
            Err("working directory must be inside the extension or workspace directory".to_string())
        }
    }

    fn list(&self, scope: &str) -> Result<Vec<String>, String> {
        let root = match scope {
            "extension" => self.extension_dir.as_path(),
            "workspace" => self.workspace_dir.as_path(),
            "session" => {
                if !self.session_read {
                    return Err("extension does not have the 'session.read' permission".to_string());
                }
                self.session_dir
                    .as_deref()
                    .ok_or_else(|| "no session directory is available".to_string())?
            }
            _ => {
                return Err(
                    "scope must be one of 'extension', 'workspace', or 'session'".to_string(),
                );
            }
        };

        let mut entries = Vec::new();
        for entry in fs::read_dir(root).map_err(|e| format!("failed to list {:?}: {}", root, e))? {
            let entry = entry.map_err(|e| format!("failed to read directory entry: {}", e))?;
            if entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false)
            {
                entries.push(entry.path().to_string_lossy().into_owned());
            }
            if entries.len() >= 500 {
                break;
            }
        }
        entries.sort();
        Ok(entries)
    }
}

fn reject_root_relative(raw: &str) -> Result<(), String> {
    let path = Path::new(raw);
    if !path.is_absolute()
        && path
            .components()
            .any(|component| matches!(component, Component::Prefix(_) | Component::RootDir))
    {
        return Err("path must be relative to an allowed root".to_string());
    }
    Ok(())
}

const MAX_READ_BYTES: u64 = MAX_FILE_BYTES;

fn read_file_text(path: &Path, max: u64) -> Result<String, String> {
    let len = fs::metadata(path)
        .map_err(|e| format!("failed to read metadata for {:?}: {}", path, e))?
        .len();
    if len > max {
        return Err(format!("file is {} bytes, maximum is {}", len, max));
    }
    fs::read_to_string(path).map_err(|e| format!("failed to read {:?}: {}", path, e))
}

fn read_file_bytes(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let len = fs::metadata(path)
        .map_err(|e| format!("failed to read metadata for {:?}: {}", path, e))?
        .len();
    if len > max {
        return Err(format!("file is {} bytes, maximum is {}", len, max));
    }
    fs::read(path).map_err(|e| format!("failed to read {:?}: {}", path, e))
}

fn create_log_function(
    lua: &Lua,
    level: &'static str,
    extension_id: String,
    logs: Arc<Mutex<LogState>>,
    quiet: bool,
) -> mlua::Result<Function> {
    lua.create_function(move |lua, value: LuaValue| {
        let message = lua
            .coerce_string(value)
            .ok()
            .flatten()
            .map(|text| text.to_string_lossy())
            .unwrap_or_else(|| "<unprintable>".to_string());
        log_message(level, &extension_id, &logs, quiet, message);
        Ok(())
    })
}

fn register_api(
    lua: &Lua,
    request: &HookRequest,
    policy: Arc<PathPolicy>,
    logs: Arc<Mutex<LogState>>,
    deadline: Instant,
) -> mlua::Result<()> {
    let extension_id = request.manifest.id.clone();
    let session_id = request.session_id.clone();
    let permissions: HashSet<Permission> = request.manifest.permissions.iter().copied().collect();
    let quiet = request.quiet;

    let sr = lua.create_table()?;

    let log = lua.create_table()?;
    let info = create_log_function(lua, "info", extension_id.clone(), Arc::clone(&logs), quiet)?;
    let warn = create_log_function(lua, "warn", extension_id.clone(), Arc::clone(&logs), quiet)?;
    let error = create_log_function(lua, "error", extension_id.clone(), Arc::clone(&logs), quiet)?;
    log.set("info", info.clone())?;
    log.set("warn", warn.clone())?;
    log.set("error", error)?;
    sr.set("log", log)?;
    lua.globals().set("print", info)?;
    lua.globals().set("warn", warn)?;

    let json = lua.create_table()?;
    json.set(
        "encode",
        lua.create_function(|lua, value: LuaValue| {
            let value = lua_to_json(lua, value).map_err(runtime_error)?;
            serde_json::to_string(&value)
                .map_err(|e| runtime_error(format!("failed to encode JSON: {}", e)))
        })?,
    )?;
    json.set(
        "decode",
        lua.create_function(|lua, text: String| {
            let value: Value = serde_json::from_str(&text)
                .map_err(|e| runtime_error(format!("failed to decode JSON: {}", e)))?;
            lua.to_value(&value)
                .map_err(|e| runtime_error(format!("failed to convert JSON: {}", e)))
        })?,
    )?;
    sr.set("json", json)?;

    let template = lua.create_table()?;
    let template_session = session_id.clone();
    let template_extension = request.manifest.id.clone();
    template.set(
        "set",
        lua.create_function(move |lua, (key, value): (String, LuaValue)| {
            let value = lua_to_json(lua, value).map_err(runtime_error)?;
            store::set_template_value(&template_session, &template_extension, &key, value)
                .map_err(runtime_error)
        })?,
    )?;
    sr.set("template", template)?;

    let state = lua.create_table()?;
    let state_session = session_id.clone();
    let state_extension = request.manifest.id.clone();
    state.set(
        "get",
        lua.create_function(move |lua, key: String| {
            let value =
                store::state_get(&state_session, &state_extension, &key).unwrap_or(Value::Null);
            lua.to_value(&value)
                .map_err(|e| runtime_error(format!("failed to convert state: {}", e)))
        })?,
    )?;
    let state_session = session_id.clone();
    let state_extension = request.manifest.id.clone();
    state.set(
        "set",
        lua.create_function(move |lua, (key, value): (String, LuaValue)| {
            let value = lua_to_json(lua, value).map_err(runtime_error)?;
            store::state_set(&state_session, &state_extension, &key, value).map_err(runtime_error)
        })?,
    )?;
    sr.set("state", state)?;

    let files = lua.create_table()?;
    let files_policy = Arc::clone(&policy);
    files.set(
        "read_text",
        lua.create_function(move |_, path: String| {
            let path = files_policy.resolve_read(&path).map_err(runtime_error)?;
            read_file_text(&path, MAX_TEXT_BYTES).map_err(runtime_error)
        })?,
    )?;
    let files_policy = Arc::clone(&policy);
    files.set(
        "read_bytes",
        lua.create_function(move |lua, path: String| {
            let path = files_policy.resolve_read(&path).map_err(runtime_error)?;
            let bytes = read_file_bytes(&path, MAX_READ_BYTES).map_err(runtime_error)?;
            lua.create_string(&bytes)
        })?,
    )?;
    let files_policy = Arc::clone(&policy);
    files.set(
        "read_json",
        lua.create_function(move |lua, path: String| {
            let path = files_policy.resolve_read(&path).map_err(runtime_error)?;
            let text = read_file_text(&path, MAX_TEXT_BYTES).map_err(runtime_error)?;
            let value: Value = serde_json::from_str(&text)
                .map_err(|e| runtime_error(format!("failed to decode JSON: {}", e)))?;
            lua.to_value(&value)
                .map_err(|e| runtime_error(format!("failed to convert JSON: {}", e)))
        })?,
    )?;
    let files_policy = Arc::clone(&policy);
    files.set(
        "write",
        lua.create_function(move |_, (path, data): (String, mlua::LuaString)| {
            let bytes = data.as_bytes().to_vec();
            if bytes.len() > MAX_READ_BYTES as usize {
                return Err(runtime_error(format!(
                    "file write exceeds the {} byte limit",
                    MAX_READ_BYTES
                )));
            }
            let path = files_policy.resolve_write(&path).map_err(runtime_error)?;
            fs::write(&path, &bytes)
                .map_err(|e| runtime_error(format!("failed to write {:?}: {}", path, e)))?;
            Ok(true)
        })?,
    )?;
    let files_policy = Arc::clone(&policy);
    files.set(
        "exists",
        lua.create_function(move |_, path: String| Ok(files_policy.resolve_read(&path).is_ok()))?,
    )?;
    let files_policy = Arc::clone(&policy);
    files.set(
        "list",
        lua.create_function(move |_, scope: String| {
            files_policy.list(&scope).map_err(runtime_error)
        })?,
    )?;
    sr.set("files", files)?;

    let workspace = lua.create_table()?;
    let workspace_policy = Arc::clone(&policy);
    workspace.set(
        "path",
        lua.create_function(move |_, name: String| {
            workspace_policy
                .resolve_write(&name)
                .map(|path| path.to_string_lossy().into_owned())
                .map_err(runtime_error)
        })?,
    )?;
    let workspace_policy = Arc::clone(&policy);
    workspace.set(
        "clear",
        lua.create_function(move |_, ()| {
            if let Err(error) = fs::remove_dir_all(&workspace_policy.workspace_dir)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                return Err(runtime_error(format!(
                    "failed to clear workspace: {}",
                    error
                )));
            }
            fs::create_dir_all(&workspace_policy.workspace_dir)
                .map_err(|e| runtime_error(format!("failed to create workspace: {}", e)))?;
            Ok(())
        })?,
    )?;
    sr.set("workspace", workspace)?;

    let time = lua.create_table()?;
    time.set(
        "now_iso",
        lua.create_function(|_, ()| Ok(chrono::Utc::now().to_rfc3339()))?,
    )?;
    time.set(
        "unix",
        lua.create_function(|_, ()| Ok(chrono::Utc::now().timestamp()))?,
    )?;
    sr.set("time", time)?;

    if permissions.contains(&Permission::Network) {
        let http = lua.create_table()?;
        http.set(
            "request",
            lua.create_function(move |lua, options: Table| {
                let remaining = remaining_ms(deadline)
                    .ok_or_else(|| runtime_error("extension hook timed out"))?;
                let url: String = options.get("url")?;
                let method: Option<String> = options.get("method")?;
                let timeout: Option<u64> = options.get("timeout_ms")?;
                let max_timeout = remaining.min(30_000);
                let timeout_ms = timeout.unwrap_or(10_000).clamp(1, max_timeout);

                let mut headers = Vec::new();
                if let Some(table) = options.get::<Option<Table>>("headers")? {
                    for pair in table.pairs::<String, String>() {
                        let (name, value) = pair?;
                        headers.push((name, value));
                    }
                }

                let body = options
                    .get::<Option<mlua::LuaString>>("body")?
                    .map(|body| body.as_bytes().to_vec());

                let response = http::execute(http::HttpRequest {
                    url,
                    method: method.unwrap_or_else(|| "GET".to_string()),
                    headers,
                    body,
                    timeout_ms,
                })
                .map_err(runtime_error)?;

                let result = lua.create_table()?;
                result.set("status", response.status)?;
                let header_table = lua.create_table()?;
                for (name, value) in response.headers {
                    header_table.set(name, value)?;
                }
                result.set("headers", header_table)?;
                result.set("body", lua.create_string(&response.body)?)?;
                Ok(result)
            })?,
        )?;
        sr.set("http", http)?;
    }

    if permissions.contains(&Permission::Process) {
        let process = lua.create_table()?;
        let process_policy = Arc::clone(&policy);
        process.set(
            "run",
            lua.create_function(move |lua, options: Table| {
                let remaining = remaining_ms(deadline)
                    .ok_or_else(|| runtime_error("extension hook timed out"))?;
                let program: String = options.get("program")?;
                let timeout: Option<u64> = options.get("timeout_ms")?;
                let max_output: Option<usize> = options.get("max_output_bytes")?;
                let max_timeout = remaining.min(300_000);
                let timeout_ms = timeout.unwrap_or(30_000).clamp(1, max_timeout);

                let mut args = Vec::new();
                if let Some(table) = options.get::<Option<Table>>("args")? {
                    for value in table.sequence_values::<String>() {
                        args.push(value?);
                    }
                }

                let stdin = options
                    .get::<Option<mlua::LuaString>>("stdin")?
                    .map(|data| data.as_bytes().to_vec());

                let mut env = Vec::new();
                if let Some(table) = options.get::<Option<Table>>("env")? {
                    for pair in table.pairs::<String, String>() {
                        let (key, value) = pair?;
                        env.push((key, value));
                    }
                }

                let cwd = match options.get::<Option<String>>("cwd")? {
                    Some(cwd) => Some(process_policy.resolve_dir(&cwd).map_err(runtime_error)?),
                    None => Some(process_policy.workspace_dir.clone()),
                };

                let output = process::execute(process::ProcessRequest {
                    program,
                    args,
                    stdin,
                    timeout_ms,
                    max_output_bytes: max_output
                        .unwrap_or(process::MAX_OUTPUT_BYTES)
                        .clamp(1024, process::MAX_OUTPUT_BYTES),
                    cwd,
                    env,
                })
                .map_err(runtime_error)?;

                let result = lua.create_table()?;
                result.set("status", output.status)?;
                result.set("success", output.success)?;
                result.set("stdout", lua.create_string(&output.stdout)?)?;
                result.set("stderr", lua.create_string(&output.stderr)?)?;
                result.set("timed_out", output.timed_out)?;
                result.set("truncated", output.truncated)?;
                Ok(result)
            })?,
        )?;
        sr.set("process", process)?;
    }

    lua.globals().set("sr", sr)?;
    Ok(())
}

fn parse_abort(value: &LuaValue) -> Option<AbortRequest> {
    let LuaValue::Table(table) = value else {
        return None;
    };
    let abort = table
        .get::<Option<bool>>("abort")
        .ok()
        .flatten()
        .unwrap_or(false);
    if !abort {
        return None;
    }
    Some(AbortRequest {
        reason: table
            .get::<Option<String>>("reason")
            .ok()
            .flatten()
            .unwrap_or_default(),
        notify: table
            .get::<Option<bool>>("notify")
            .ok()
            .flatten()
            .unwrap_or(false),
    })
}

fn execute_inner(
    request: &HookRequest,
    logs: Arc<Mutex<LogState>>,
) -> Result<Option<AbortRequest>, String> {
    store::ensure_session(&request.session_id);

    let permissions: HashSet<Permission> = request.manifest.permissions.iter().copied().collect();
    let policy = Arc::new(PathPolicy::new(
        &request.extension_dir,
        &request.workspace_dir,
        request.session_dir.as_deref(),
        permissions.contains(&Permission::SessionRead),
    )?);

    let deadline =
        Instant::now() + Duration::from_millis(request.manifest.limits.effective_timeout_ms());

    let libs = StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8;
    let lua = Lua::new_with(libs, LuaOptions::new())
        .map_err(|e| format!("failed to create Lua state: {}", e))?;
    lua.set_memory_limit(request.manifest.limits.effective_memory_bytes())
        .map_err(|e| format!("failed to set memory limit: {}", e))?;

    let expired = Arc::new(AtomicBool::new(false));
    let hook_expired = Arc::clone(&expired);
    let hook_deadline = deadline;
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(10_000),
        move |_, _| {
            if Instant::now() >= hook_deadline {
                hook_expired.store(true, Ordering::SeqCst);
                Err(runtime_error("extension hook timed out"))
            } else {
                Ok(VmState::Continue)
            }
        },
    )
    .map_err(|e| format!("failed to install instruction hook: {}", e))?;

    register_api(
        &lua,
        request,
        Arc::clone(&policy),
        Arc::clone(&logs),
        deadline,
    )
    .map_err(|e| format!("failed to register extension API: {}", e))?;

    let globals = lua.globals();
    for name in [
        "dofile",
        "loadfile",
        "load",
        "collectgarbage",
        "require",
        "package",
        "io",
        "os",
        "debug",
    ] {
        globals
            .set(name, LuaValue::Nil)
            .map_err(|e| format!("failed to remove global '{}': {}", name, e))?;
    }

    let main_path = request.manifest.main_path(&request.extension_dir);
    let metadata = fs::metadata(&main_path)
        .map_err(|e| format!("failed to read main script {:?}: {}", main_path, e))?;
    if metadata.len() > MAX_SCRIPT_BYTES {
        return Err(format!(
            "main script is {} bytes, maximum is {}",
            metadata.len(),
            MAX_SCRIPT_BYTES
        ));
    }
    let source = fs::read_to_string(&main_path)
        .map_err(|e| format!("failed to read main script {:?}: {}", main_path, e))?;

    let module: Table = lua
        .load(&source)
        .set_name(request.manifest.id.clone())
        .eval()
        .map_err(|e| format!("failed to load main script: {}", e))?;

    let function: Function = module
        .get(request.hook.as_str())
        .map_err(|_| format!("hook '{}' is not defined", request.hook.as_str()))?;

    let context = request
        .context
        .to_value()
        .map_err(|e| format!("failed to build hook context: {}", e))?;
    let context = lua
        .to_value(&context)
        .map_err(|e| format!("failed to convert hook context: {}", e))?;

    let result = function.call(context);

    if expired.load(Ordering::SeqCst) {
        return Err("extension hook timed out".to_string());
    }

    let result = result.map_err(|e| format!("hook failed: {}", e))?;

    if request.hook == Hook::BeforeRecording {
        Ok(parse_abort(&result))
    } else {
        Ok(None)
    }
}

pub fn execute(request: HookRequest) -> HookOutcome {
    let started = Instant::now();
    let logs = Arc::new(Mutex::new(LogState::default()));
    let result = execute_inner(&request, Arc::clone(&logs));

    let entries = logs
        .lock()
        .map(|state| state.entries.clone())
        .unwrap_or_default();

    let (error, aborted) = match result {
        Ok(aborted) => (None, aborted),
        Err(error) => {
            let message = truncate(&error, MAX_LOG_MESSAGE_BYTES);
            if !request.quiet {
                eprintln!("[ext:{}] {}", request.manifest.id, message);
            }
            (Some(error), None)
        }
    };

    HookOutcome {
        logs: entries,
        error,
        aborted,
        duration: started.elapsed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::context::HookContext;
    use crate::extension::manifest::{Limits, SettingDef, SettingKind};
    use serde_json::{Map, json};
    use std::collections::HashMap;

    fn test_manifest(permissions: Vec<Permission>) -> ExtensionManifest {
        ExtensionManifest {
            schema: None,
            id: "test_ext".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            icon: None,
            stream_recorder_version: None,
            api: 1,
            main: "main.lua".to_string(),
            files: Vec::new(),
            hooks: Hook::ALL.to_vec(),
            permissions,
            limits: Limits {
                timeout_ms: 5_000,
                memory_mb: 64,
            },
            settings: HashMap::new(),
            source_url: None,
        }
    }

    fn make_request(
        manifest: ExtensionManifest,
        source: &str,
        permissions: Vec<Permission>,
    ) -> (HookRequest, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("main.lua"), source).expect("write script");
        let context = HookContext::new(&manifest, Map::new(), Hook::AfterRecording);
        let request = HookRequest {
            manifest,
            extension_dir: dir.path().to_path_buf(),
            hook: Hook::AfterRecording,
            context,
            session_id: format!("test-{}", uuid::Uuid::new_v4()),
            session_dir: None,
            workspace_dir: dir.path().join(".workspace"),
            quiet: true,
        };
        let _ = permissions;
        (request, dir)
    }

    #[test]
    fn script_can_write_template_values() {
        let manifest = test_manifest(Vec::new());
        let (request, _dir) = make_request(
            manifest,
            r#"
            return {
              after_recording = function(ctx)
                sr.template.set("deaths", 12)
                sr.log.info("done")
                sr.log.info(123)
              end,
            }
            "#,
            Vec::new(),
        );
        let session = request.session_id.clone();
        let outcome = execute(request);
        assert!(outcome.error.is_none(), "{:?}", outcome.error);
        assert_eq!(
            store::template_values(&session)["test_ext"]["deaths"],
            json!(12)
        );
        assert!(outcome.logs.iter().any(|log| log.message == "done"));
        assert!(outcome.logs.iter().any(|log| log.message == "123"));
        store::remove_session(&session);
    }

    #[test]
    fn before_recording_can_abort() {
        let manifest = test_manifest(Vec::new());
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            dir.path().join("main.lua"),
            r#"
            return {
              before_recording = function(ctx)
                return { abort = true, reason = "not interested", notify = true }
              end,
            }
            "#,
        )
        .expect("write script");
        let context = HookContext::new(&manifest, Map::new(), Hook::BeforeRecording);
        let request = HookRequest {
            manifest,
            extension_dir: dir.path().to_path_buf(),
            hook: Hook::BeforeRecording,
            context,
            session_id: format!("test-{}", uuid::Uuid::new_v4()),
            session_dir: None,
            workspace_dir: dir.path().join(".workspace"),
            quiet: true,
        };
        let outcome = execute(request);
        let abort = outcome.aborted.expect("abort request");
        assert_eq!(abort.reason, "not interested");
        assert!(abort.notify);
    }

    #[test]
    fn sandbox_blocks_filesystem_globals() {
        let manifest = test_manifest(Vec::new());
        let (request, _dir) = make_request(
            manifest,
            r#"
            return {
              after_recording = function(ctx)
                local ok = pcall(function() return io.open("/etc/passwd") end)
                error("io exposed: " .. tostring(ok))
              end,
            }
            "#,
            Vec::new(),
        );
        let outcome = execute(request);
        assert!(outcome.error.is_some());
        assert!(outcome.error.unwrap().contains("hook failed"));
    }

    #[test]
    fn process_and_http_apis_require_permissions() {
        let manifest = test_manifest(Vec::new());
        let (request, _dir) = make_request(
            manifest,
            r#"
            return {
              after_recording = function(ctx)
                sr.process.run({ program = "echo" })
              end,
            }
            "#,
            Vec::new(),
        );
        let outcome = execute(request);
        assert!(outcome.error.is_some());

        let manifest = test_manifest(Vec::new());
        let (request, _dir) = make_request(
            manifest,
            r#"
            return {
              after_recording = function(ctx)
                sr.http.request({ url = "https://example.com" })
              end,
            }
            "#,
            Vec::new(),
        );
        let outcome = execute(request);
        assert!(outcome.error.is_some());
    }

    #[test]
    fn infinite_loop_is_stopped_by_timeout() {
        let manifest = test_manifest(Vec::new());
        let (mut request, _dir) = make_request(
            manifest,
            r#"
            return {
              after_recording = function(ctx)
                while true do end
              end,
            }
            "#,
            Vec::new(),
        );
        request.manifest.limits.timeout_ms = 1_000;

        let outcome = execute(request);
        let error = outcome.error.expect("timeout error");
        assert!(error.contains("timed out"), "got: {}", error);
    }

    #[test]
    fn files_read_outside_roots_is_rejected() {
        let manifest = test_manifest(vec![Permission::SessionRead]);
        let (request, _dir) = make_request(
            manifest,
            r#"
            return {
              after_recording = function(ctx)
                sr.files.read_text("C:/Windows/win.ini")
              end,
            }
            "#,
            vec![Permission::SessionRead],
        );
        let outcome = execute(request);
        assert!(outcome.error.is_some());
    }

    #[test]
    fn settings_are_available_in_context() {
        let mut manifest = test_manifest(Vec::new());
        manifest.settings.insert(
            "count".to_string(),
            SettingDef {
                kind: SettingKind::Integer,
                default: Some(json!(3)),
                description: None,
            },
        );
        let mut settings = Map::new();
        settings.insert("count".to_string(), json!(3));
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            dir.path().join("main.lua"),
            r#"
            return {
              after_recording = function(ctx)
                sr.template.set("count", ctx.settings.count)
              end,
            }
            "#,
        )
        .expect("write script");
        let context = HookContext::new(&manifest, settings, Hook::AfterRecording);
        let request = HookRequest {
            manifest,
            extension_dir: dir.path().to_path_buf(),
            hook: Hook::AfterRecording,
            context,
            session_id: format!("test-{}", uuid::Uuid::new_v4()),
            session_dir: None,
            workspace_dir: dir.path().join(".workspace"),
            quiet: true,
        };
        let session = request.session_id.clone();
        let outcome = execute(request);
        assert!(outcome.error.is_none(), "{:?}", outcome.error);
        assert_eq!(
            store::template_values(&session)["test_ext"]["count"],
            json!(3)
        );
        store::remove_session(&session);
    }
}
