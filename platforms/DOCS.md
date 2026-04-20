# Platform JSON Guide

This directory contains the platform system used by `stream-recorder`.
Platforms are JSON files that teach the recorder how to talk to a site, decide
whether a streamer is live, and extract the playback URL and metadata needed
for recording.

If you want to create your own platform, start with [example.json](example.json)
and keep [schema.json](schema.json) open while you work. The schema describes
the supported fields, and the example shows a complete working layout.

> [!NOTE]
> If you create and upload a platform to github, please add the topic `stream-recorder-rs-platform` so it is easily discoverable by the `stream-recorder platform search` command.

## What A Platform Does

A platform config is a small HTTP pipeline:

1. The recorder starts with a username and, if needed, an auth token.
2. It runs each step in `steps` in order.
3. Each step fetches an endpoint, checks an optional `live_check`, and can
   extract values from the response JSON.
4. Extracted values become available to later steps as `{variable}`
   placeholders.
5. At some point the pipeline must extract `playback_url`, because that is the
   stream URL passed to FFmpeg.

Here are the current internally used extracted values, it is recommended you include as many as you can. You can see it in the [code](../src/stream/types.rs#L14).

| Name           | Required | Purpose                        |
| -------------- | -------- | ------------------------------ |
| `playback_url` | Yes      | Used to record the stream.     |
| `stream_title` | No       | Used in discord notifications. |
| `avatar_url`   | No       | Used in discord notifications. |

## Where Platform Files Live

You can either:

1. Install a platform with `stream-recorder platform install <url>`.
2. Place a JSON file directly in the platforms directory, either at `~/.config/stream-recorder/platforms/*` (Linux/macOS) or `%APPDATA%\stream-recorder\platforms\*` (Windows).

On first install, the original URL is saved as `source_url` so the platform can
be updated later with `stream-recorder platform update`.

The installer accepts direct JSON URLs and common GitHub repo/tree URLs. If you
point it at a GitHub repository or tree, it resolves that to the raw platform
JSON automatically.

## Minimal Shape

Every platform JSON file must contain these fields:

- `id`
- `name`
- `base_url`
- `headers`
- `steps`
- `version`

Optional fields include `icon`, `token_name`, `stream_recorder_version`,
`source_url`, and `title_clean_regex`.

## Example

This is a good starting point for a custom platform:

```json
{
    "$schema": "https://raw.githubusercontent.com/sn0w12/stream-recorder-rs/refs/heads/main/platforms/schema.json",
    "id": "example",
    "name": "Example Platform",
    "version": "1.0.0",
    "stream_recorder_version": "^0.1",
    "title_clean_regex": [
        ":\\w+:",
        {
            "pattern": "\\s+",
            "replacement": " "
        }
    ],
    "base_url": "https://api.example.com/v1/",
    "token_name": "example_api_token",
    "headers": {
        "Authorization": "Bearer {token}",
        "Accept": "application/json"
    },
    "steps": [
        {
            "endpoint": "streams/{username}",
            "live_check": {
                "path": "data.is_live",
                "equals": true
            },
            "extract": {
                "stream_id": "data.id",
                "stream_title": "data.title",
                "user_id": "data.user_id",
                "avatar_url": "data.user_avatar"
            }
        },
        {
            "endpoint": "streams/{stream_id}/playback",
            "extract": {
                "playback_url": "data.hls_url"
            }
        }
    ]
}
```

## Field Reference

### Identity And Compatibility

| Field                     | Required | Purpose                                                                             |
| ------------------------- | -------- | ----------------------------------------------------------------------------------- |
| `id`                      | Yes      | Unique platform ID. This is what users reference in `platform_id:username`.         |
| `name`                    | Yes      | Human-readable platform name.                                                       |
| `version`                 | Yes      | Platform file version. It must be a non-empty string.                               |
| `stream_recorder_version` | No       | Semver requirement for the recorder version, for example `^0.1` or `>=0.1.0, <2.0`. |
| `icon`                    | No       | Optional URL for a platform icon.                                                   |

### HTTP Setup

| Field        | Required | Purpose                                                                                      |
| ------------ | -------- | -------------------------------------------------------------------------------------------- |
| `base_url`   | Yes      | Base URL for relative endpoints. It must end with `/`.                                       |
| `headers`    | Yes      | Headers sent with each request. Use `{token}` as a placeholder where needed.                 |
| `token_name` | No       | Keyring entry used to look up the auth token. If omitted, the platform does not require one. |

### Pipeline

| Field   | Required | Purpose                                                     |
| ------- | -------- | ----------------------------------------------------------- |
| `steps` | Yes      | Ordered list of fetch steps. At least one step is required. |

#### Pipeline Steps

Each step is a JSON object with these fields:

| Field        | Required | Purpose                                                                        |
| ------------ | -------- | ------------------------------------------------------------------------------ |
| `endpoint`   | Yes      | Endpoint template. Relative paths are joined to `base_url`.                    |
| `live_check` | No       | Optional condition that decides whether the stream is live.                    |
| `extract`    | No       | Map of variable names to dot-notation JSON paths. Defaults to an empty object. |

## Step Behavior

Steps run in order. A later step can use values extracted by an earlier step.
That means you can do this:

- Step 1: fetch the user page and extract `stream_id`.
- Step 2: use `streams/{stream_id}/playback` to get the real playback URL.

The initial variable map always includes at least `username` and, when
available, `token`.

### Extract Paths

Extraction uses dot notation with optional array indexing.

Examples:

- `data.id`
- `data.items[0].url`
- `response.streams[1].title`

If a path does not resolve, the value is treated as missing.

### Live Checks

`live_check` tells the recorder whether a step means the stream is currently
live.

It supports three forms:

1. A string path shorthand such as `data.is_live`.
2. An object with a required `path` and optional `exists`, `equals`, and
   `not_equals` checks.
3. `null`, if you do not want a live check on that step.

Examples:

```json
"live_check": "data.is_live"
```

```json
"live_check": {
	"path": "data.status",
	"equals": "live"
}
```

```json
"live_check": {
	"path": "data.viewer_count",
	"exists": false
}
```

Use `equals` when the API returns an explicit status value. Use the string form
when you only care that a path exists and is not null.

## Title Cleaning

`title_clean_regex` lets you rewrite the extracted `stream_title` before it is
used in filenames.

It is an ordered list of rules:

- A string rule removes every match.
- An object rule replaces every match with the given `replacement`.

Example:

```json
"title_clean_regex": [
	":\\w+:",
	{
		"pattern": "\\s+",
		"replacement": " "
	}
]
```

This example removes emoji shortcodes such as `:smile:` and collapses repeated
whitespace.

## Recommended Authoring Workflow

1. Copy [example.json](example.json) into a new file.
2. Change `id`, `name`, and `base_url` first.
3. Add the request headers your target site expects.
4. Build the first step so it can decide whether the stream is live.
5. Add later steps until you extract `playback_url`.
6. Add `stream_title` and `user_id` if the site exposes them.
7. Install or update the file with `stream-recorder platform install <url>`.
8. Test the pipeline with `stream-recorder platform debug platform_id:username`.

If the platform needs a token, install it first and save the token with the
platform's `token_name` value.

## Debugging Tips

If the platform does not work the first time:

- Run `stream-recorder platform debug <platform_id:username>` to inspect each
  step.
- Use `--show-response` to print the raw JSON responses.
- Verify that any endpoint templates still make sense after variable
  substitution.
- Check that `playback_url` is actually being extracted by the final pipeline
  step.
- If the site returns HTML instead of JSON, the config probably needs a
  different endpoint or headers.

## Related Files

- [schema.json](schema.json) defines the accepted JSON structure.
- [example.json](example.json) provides a full starter configuration.
