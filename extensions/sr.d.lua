---@meta

---@class StreamRecorderLog
StreamRecorderLog = {}

---@param message any
function StreamRecorderLog.info(message) end

---@param message any
function StreamRecorderLog.warn(message) end

---@param message any
function StreamRecorderLog.error(message) end

---@class StreamRecorderJson
StreamRecorderJson = {}

---@param value any
---@return string
function StreamRecorderJson.encode(value) end

---@param text string
---@return any
function StreamRecorderJson.decode(text) end

---@class StreamRecorderTemplate
StreamRecorderTemplate = {}

---@param key string
---@param value any
function StreamRecorderTemplate.set(key, value) end

---@class StreamRecorderState
StreamRecorderState = {}

---@param key string
---@return any
function StreamRecorderState.get(key) end

---@param key string
---@param value any
function StreamRecorderState.set(key, value) end

---@alias StreamRecorderFileScope "extension"|"workspace"|"session"

---@class StreamRecorderFiles
StreamRecorderFiles = {}

---@param path string
---@return string
function StreamRecorderFiles.read_text(path) end

---@param path string
---@return string
function StreamRecorderFiles.read_bytes(path) end

---@param path string
---@return any
function StreamRecorderFiles.read_json(path) end

---@param path string
---@param data string
---@return boolean
function StreamRecorderFiles.write(path, data) end

---@param path string
---@return boolean
function StreamRecorderFiles.exists(path) end

---@param scope StreamRecorderFileScope
---@return string[]
function StreamRecorderFiles.list(scope) end

---@class StreamRecorderWorkspace
StreamRecorderWorkspace = {}

---@param name string
---@return string
function StreamRecorderWorkspace.path(name) end

function StreamRecorderWorkspace.clear() end

---@class StreamRecorderTime
StreamRecorderTime = {}

---@return string
function StreamRecorderTime.now_iso() end

---@return integer
function StreamRecorderTime.unix() end

---@class StreamRecorderHttpRequest
---@field url string
---@field method? string
---@field headers? table<string, string>
---@field body? string
---@field timeout_ms? integer

---@class StreamRecorderHttpResponse
---@field status integer
---@field headers table<string, string>
---@field body string

---@class StreamRecorderHttp
StreamRecorderHttp = {}

---@param request StreamRecorderHttpRequest
---@return StreamRecorderHttpResponse
function StreamRecorderHttp.request(request) end

---@class StreamRecorderProcessRequest
---@field program string
---@field args? string[]
---@field stdin? string
---@field timeout_ms? integer
---@field max_output_bytes? integer
---@field cwd? string
---@field env? table<string, string>

---@class StreamRecorderProcessResult
---@field status integer
---@field success boolean
---@field stdout string
---@field stderr string
---@field timed_out boolean
---@field truncated boolean

---@class StreamRecorderProcess
StreamRecorderProcess = {}

---@param request StreamRecorderProcessRequest
---@return StreamRecorderProcessResult
function StreamRecorderProcess.run(request) end

---@class StreamRecorderApi
---@field log StreamRecorderLog
---@field json StreamRecorderJson
---@field template StreamRecorderTemplate
---@field state StreamRecorderState
---@field files StreamRecorderFiles
---@field workspace StreamRecorderWorkspace
---@field time StreamRecorderTime
---@field http StreamRecorderHttp
---@field process StreamRecorderProcess

---@type StreamRecorderApi
sr = {}

---@class ExtensionIdentity
---@field id string
---@field name string
---@field version string

---@class PlatformContext
---@field id string
---@field name string

---@class StreamContext
---@field title? string
---@field avatar_url? string
---@field user_id? string

---@class SessionContext
---@field id string
---@field started_at string
---@field username string
---@field platform PlatformContext
---@field stream StreamContext

---@class RecordingContext
---@field file? string
---@field output_path? string
---@field thumbnail? string
---@field duration_seconds? number
---@field size_bytes? integer
---@field segments? integer
---@field is_continuation? boolean
---@field playback_url? string

---@class ErrorContext
---@field stage string
---@field message string
---@field uploader? string
---@field file? string
---@field status_code? integer
---@field retryable? boolean

---@class UploadEntry
---@field uploader string
---@field urls string[]

---@class UploadsContext
---@field video UploadEntry[]
---@field image UploadEntry[]

---@class HookContext
---@field api integer
---@field hook string
---@field extension ExtensionIdentity
---@field settings table<string, any>
---@field session? SessionContext
---@field recording? RecordingContext
---@field uploads? UploadsContext
---@field error? ErrorContext

---@class HookModule
---@field before_recording? fun(ctx: HookContext)
---@field after_recording? fun(ctx: HookContext)
---@field before_upload? fun(ctx: HookContext)
---@field after_upload? fun(ctx: HookContext)
---@field on_error? fun(ctx: HookContext)
