# Extension Guide

Extensions are sandboxed Lua scripts that run at defined points in the recording
pipeline. They can inspect the session, run processes, make HTTP requests, and
add values to the notification template. They are separate from platforms:
platforms describe how to find and fetch a stream, extensions run code around
the recording lifecycle.

Start with [example/extension.json](example/extension.json) and
[example/main.lua](example/main.lua). The [schema.json](schema.json) describes
the manifest fields.

> [!NOTE]
> If you publish an extension on GitHub, add the topic
> `stream-recorder-rs-extension` so it is easy to find.

## Security model

Extension Lua runs in a fresh sandbox for every hook. The sandbox has no
`io`, `os`, `package`, `require`, `debug`, or `coroutine` library. The only way
to touch the outside world is through the host API.

The host never gives an extension a platform token, uploader token, webhook URL,
`.env` contents, keyring value, or the global configuration. The context is
built from an allowlist of fields.

The `process` permission is different. It runs programs as your user. Those
programs can read files your user can read, including
`~/.config/stream_recorder/.env` and the system keyring. The host still never
passes a token to the extension, but a process extension can reach one on its
own. Treat `process` like installing a native program.

| Permission     | What it allows                                              |
| -------------- | ----------------------------------------------------------- |
| `session.read` | Read the current recording and thumbnail through `sr.files`. |
| `network`      | Make HTTP requests through `sr.http`.                       |
| `process`      | Run programs through `sr.process`.                          |

`extension install` prints every requested permission and asks for confirmation.
An update that adds permissions asks again. Use
`config set disabled_extensions "id1,id2"` to disable extensions without
removing them.

Hooks have a timeout and a memory limit, which stop ordinary runaway loops and
allocations. They are not a hostile-code sandbox against deliberate CPU
exhaustion. Only install extensions you trust.

## Where extensions live

```
~/.config/stream_recorder/extensions/
  death_counter/
    extension.json
    main.lua
    config.toml        # optional, user-edited
    assets/            # optional, read-only to the script
    .workspace/        # host-managed scratch space
```

On Windows the root is `%APPDATA%\stream_recorder\extensions\`.

`main.lua` must return a table. Each key is a hook name:

```lua
local M = {}

function M.after_recording(ctx)
  sr.log.info("recording finished: " .. ctx.recording.file)
end

return M
```

Only hooks listed in `extension.json` are called. A hook can be absent from the
table, but then it should not be listed in the manifest.

## Creating an extension

```bash
stream-recorder extension new my_extension
```

This creates `my_extension/` in the current directory with:

- `extension.json`
- `main.lua`
- `sr.d.lua`
- `.luarc.json`
- `.gitignore`

Use `--path <dir>` to choose the parent directory and
`--name "Display Name"` to set the display name. `extension create` is an
alias.

Copy the folder into the extensions directory to install it, then restart the
recorder.

## Editor support

LuaLS reports `Undefined global 'sr'` unless it has the API definitions. The
repo includes [sr.d.lua](sr.d.lua), a `---@meta` definition file for the full
`sr` API and hook context.

Open `extensions/` as the LuaLS workspace, or point your existing workspace
library at `extensions/sr.d.lua`. For an installed extension, copy `sr.d.lua`
next to `main.lua` and add `.luarc.json`:

```json
{
    "runtime.version": "Lua 5.4",
    "workspace.library": ["./sr.d.lua"]
}
```

Then annotate the module and hook:

```lua
---@type HookModule
local M = {}

---@param ctx HookContext
function M.after_recording(ctx)
  local recording = ctx.recording
  local seconds = recording and recording.duration_seconds or 0
  sr.template.set("duration_minutes", math.floor(seconds / 60))
end

return M
```

`sr.d.lua` covers `sr.log`, `sr.json`, `sr.template`, `sr.state`, `sr.files`,
`sr.workspace`, `sr.time`, `sr.http`, `sr.process`, and the hook context types.
It is only for the editor. The recorder does not execute it.

## Manifest

```json
{
  "$schema": "https://raw.githubusercontent.com/sn0w12/stream-recorder-rs/refs/heads/main/extensions/schema.json",
  "id": "example",
  "name": "Example Extension",
  "version": "1.0.0",
  "stream_recorder_version": "^0.2",
  "api": 1,
  "description": "Shows how to use extension hooks.",
  "author": "you",
  "main": "main.lua",
  "files": ["main.lua", "assets/notes.txt"],
  "hooks": ["after_recording"],
  "permissions": ["session.read"],
  "limits": {
    "timeout_ms": 30000,
    "memory_mb": 64
  },
  "settings": {
    "sample_seconds": {
      "type": "number",
      "default": 10,
      "description": "Timestamp to inspect"
    },
    "lang": {
      "type": "string",
      "default": "eng",
      "description": "OCR language"
    }
  }
}
```

| Field                     | Required | Purpose                                                        |
| ------------------------- | -------- | -------------------------------------------------------------- |
| `id`                      | Yes      | Unique ID, matching `^[a-z][a-z0-9_]{0,63}$`.                  |
| `name`                    | Yes      | Display name.                                                  |
| `version`                 | Yes      | Extension version.                                             |
| `api`                     | No       | Extension API version. Must be `1`.                            |
| `main`                    | Yes      | Lua entry file, relative to the extension directory.           |
| `files`                   | No       | Extra files downloaded on install. `main` is always included.  |
| `hooks`                   | Yes      | Hooks this extension implements.                               |
| `permissions`             | No       | `session.read`, `network`, and/or `process`.                   |
| `limits.timeout_ms`       | No       | Per-hook timeout. Default 30 s, clamped to 1 s to 300 s.       |
| `limits.memory_mb`        | No       | Lua memory limit. Default 64 MB, clamped to 8 MB to 512 MB.    |
| `stream_recorder_version` | No       | Semver requirement for the recorder.                           |
| `settings`                | No       | User-editable settings, stored in `config.toml`.               |
| `source_url`              | No       | Set automatically by `extension install`.                      |

Setting types are `string`, `number`, `integer`, `boolean`, `string_array`, and
`toml`. A `toml` setting accepts any TOML value, including arrays of tables:

```toml
[[rules]]
username = "MaliceTheSuccubus"
key = "spanks"
label = "Spanks"
type = "number"

[[rules]]
username = "MaliceTheSuccubus"
key = "cums"
label = "Cums"
type = "number"
```

Declare it in the manifest with `"type": "toml"`. The value arrives in Lua as
the matching table.

## Hooks

| Hook               | When                                                                 | Context                                                  |
| ------------------ | -------------------------------------------------------------------- | -------------------------------------------------------- |
| `before_recording` | Once per session, after the output path is chosen, before ffmpeg     | `recording.output_path`, `recording.playback_url`        |
| `after_recording`  | After segments are combined, metadata is read, thumbnail is created  | `recording.file`, `recording.thumbnail`, duration, size  |
| `before_upload`    | Before uploaders run                                                 | `uploads.video`, `uploads.image` with uploader names     |
| `after_upload`     | After uploads, before the notification template is rendered          | `uploads.video`, `uploads.image` with URLs               |
| `on_error`         | Pipeline, recording, post-processing, upload, or template failure    | `error.stage`, `error.message`, optional uploader/status |

`before_recording` is the only hook that sees `playback_url`. It is a signed
credential, so it is omitted everywhere else.

`on_error` receives a short summary, not the raw request or response. Raw
details stay in the recorder logs and Discord error thread, so platform tokens
and webhook URLs are not passed to extensions.

### Aborting a recording

`before_recording` can return a table to skip the stream:

```lua
return {
  abort = true,
  reason = "not the game I record",
  notify = true, -- also send a notice to the Discord webhook, if configured
}
```

The monitor retries on its next poll, so the hook will run again. Use
`sr.state` or a setting if you need to suppress repeated aborts.

## Context

Every hook receives an `api`, `hook`, `extension`, `settings`, and `session`
field, plus the hook-specific fields.

```lua
{
  api = 1,
  hook = "after_recording",
  extension = { id = "example", name = "Example", version = "1.0.0" },
  settings = { sample_seconds = 10 },
  session = {
    id = "…",
    started_at = "2026-11-21T12:00:00+00:00",
    username = "example_user",
    platform = { id = "twitch", name = "Twitch" },
    stream = { title = "Ranked", avatar_url = "https://…", user_id = "12345" },
  },
  recording = {
    file = "C:/recordings/example_user/2026-11-21_12-00-00.mp4",
    thumbnail = "…_thumb.jpg",
    duration_seconds = 3720.5,
    size_bytes = 123456789,
    segments = 1,
  },
}
```

Session data is never present for a pipeline error, because no session exists
yet. Check `ctx.session` before using it in `on_error`.

## Lua API

### Logging

```lua
sr.log.info("message")
sr.log.warn("message")
sr.log.error("message")
```

`print` is redirected to `sr.log.info`. Logs are prefixed with the extension ID
and capped at 200 entries per hook.

### JSON

```lua
local text = sr.json.encode({ deaths = 12 })
local value = sr.json.decode(text)
```

### Template values

```lua
sr.template.set("deaths", 12)
sr.template.set("stats", { deaths = 12, kills = 30 })
```

Keys must match `^[a-z][a-z0-9_]{0,63}$`. Values may be nil, booleans, numbers,
strings, arrays, or objects up to four levels deep. Limits are 100 keys, 64 KiB
per value, and 1 MiB per extension.

Values are available in the notification template under
`ext.<extension_id>.<key>`:

```handlebars
Deaths: {{ext.example.deaths}}
```

Values written in `before_recording`, `after_recording`, `before_upload`, or
`after_upload` survive until the final template render for the session.

### Session state

```lua
sr.state.set("cursor", { index = 4 })
local cursor = sr.state.get("cursor")
```

State is scoped to one extension and one session. It is cleared when the session
finishes. The value is JSON-compatible and capped at 64 KiB per key and 256 KiB
per extension.

### Files

```lua
local text = sr.files.read_text("assets/notes.txt")
local bytes = sr.files.read_bytes(ctx.recording.thumbnail)
local data = sr.files.read_json("assets/config.json")
sr.files.write("result.txt", "hello")
local files = sr.files.list("session")
```

`session.read` is required to read the recording or thumbnail. The extension
directory is always readable; the workspace is readable and writable. Reads are
capped at 10 MiB, text reads at 1 MiB, and writes at 10 MiB. Absolute paths are
accepted only when they resolve inside one of those roots.

`sr.files.list` accepts `"extension"`, `"workspace"`, or `"session"` and returns
absolute paths.

### Workspace

```lua
local frame = sr.workspace.path("frames/first.png")
sr.workspace.clear()
```

The workspace is a per-session scratch directory. `sr.workspace.path` creates
parent directories and returns an absolute path. It is useful for passing output
files to a process.

### HTTP

Requires `network`.

```lua
local response = sr.http.request({
  url = "https://example.com/api",
  method = "POST",
  headers = { ["Content-Type"] = "application/json" },
  body = sr.json.encode({ hello = "world" }),
  timeout_ms = 10000,
})
-- response.status, response.headers, response.body
```

Bodies are raw Lua strings, so binary uploads work. Only `http` and `https` are
accepted. Timeouts are capped at 30 s or the remaining hook budget. Response
bodies are capped at 5 MiB and request bodies at 10 MiB.

### Processes

Requires `process`. This runs programs as your user. See the security section.

```lua
local result = sr.process.run({
  program = "tesseract",
  args = { image_path, "stdout", "-l", "eng", "--psm", "6" },
  stdin = nil,
  timeout_ms = 30000,
  max_output_bytes = 1048576,
  cwd = nil,
  env = { OCR_LANG = "eng" },
})
-- result.status, result.success, result.stdout, result.stderr
-- result.timed_out, result.truncated
```

There is no shell. `args` is an argument array. The default working directory is
the workspace. The default environment is a minimal inherited set plus your
`env` overrides. The process group is killed on timeout, and output is capped at
10 MiB per stream. Calls are synchronous and run one at a time within a hook.

### Time

```lua
local iso = sr.time.now_iso()
local unix = sr.time.unix()
```

## Example: death counter OCR

This extension extracts a frame with ffmpeg, runs Tesseract, and puts the result
in the template. OCR lives entirely in the extension, not in the recorder.

`extension.json`:

```json
{
  "id": "death_counter",
  "name": "Death Counter OCR",
  "version": "1.0.0",
  "api": 1,
  "main": "main.lua",
  "hooks": ["after_recording"],
  "permissions": ["process"],
  "limits": { "timeout_ms": 120000, "memory_mb": 64 },
  "settings": {
    "sample_seconds": { "type": "number", "default": 30 },
    "lang": { "type": "string", "default": "eng" }
  }
}
```

`main.lua`:

```lua
local M = {}

function M.after_recording(ctx)
  local frame = sr.workspace.path("frame.png")

  local extracted = sr.process.run({
    program = "ffmpeg",
    args = {
      "-hide_banner", "-loglevel", "error",
      "-ss", tostring(ctx.settings.sample_seconds),
      "-i", ctx.recording.file,
      "-frames:v", "1", "-y", frame,
    },
    timeout_ms = 60000,
  })
  if extracted.status ~= 0 then
    sr.log.error("ffmpeg failed: " .. extracted.stderr)
    return
  end

  local ocr = sr.process.run({
    program = "tesseract",
    args = { frame, "stdout", "-l", ctx.settings.lang, "--psm", "6" },
    timeout_ms = 60000,
  })
  if ocr.status ~= 0 then
    sr.log.error("tesseract failed: " .. ocr.stderr)
    return
  end

  sr.template.set("deaths", tonumber(ocr.stdout:match("(%d+)")) or 0)
end

return M
```

Template:

```handlebars
Deaths: {{ext.death_counter.deaths}}
```

## Settings

Put user values in `extensions/<id>/config.toml`:

```toml
sample_seconds = 45
lang = "eng"
```

Values are validated against the manifest types and merged with the defaults.
Unknown keys are ignored with a warning. `extension config <id>` prints the path
and effective values. Restart the recorder after editing the manifest; settings
are read at startup.

`config.toml` is ordinary text. Extension-owned API keys belong there only if
you accept that they are stored in plain text. The recorder's own tokens are
never placed in it.

## Installing and updating

```bash
stream-recorder extension install https://github.com/owner/repo
stream-recorder extension install https://example.com/extension.json
stream-recorder extension list
stream-recorder extension update death_counter
stream-recorder extension update --all
stream-recorder extension remove death_counter
```

The installer accepts a direct `extension.json` URL, a GitHub repository URL, or
a GitHub tree URL. It downloads `main` and every declared file relative to the
manifest, validates them, and swaps the directory into place. `config.toml` and
`.workspace` are preserved across updates.

## Debugging

Run a single hook with synthetic data:

```bash
stream-recorder extension debug example after_recording --file ./recording.mp4
stream-recorder extension debug example before_recording
stream-recorder extension debug example on_error
```

The debug command prints logs, the hook duration, any abort request, and the
template values the hook wrote. Permissions are still enforced, so a `process`
hook will really run programs.

## Limitations

- One Lua file. `require` is disabled.
- No native Lua modules. Use `sr.process` for native tools.
- No background processes. `sr.process.run` is synchronous.
- Manifests and settings are loaded at startup. `main.lua` is re-read for each
  hook, so script edits apply on the next run. A manifest or `config.toml`
  change needs a restart.
- Extension failures never stop a recording. They are logged and the pipeline
  continues.
