# Stream Recorder

A high-performance CLI tool written in Rust for recording live streams from a variety of platforms. It automatically monitors specified accounts or channels, records streams with FFmpeg, generates thumbnails, uploads content to multiple file hosting services, and sends notifications via Discord webhooks.

## Prerequisites

- Rust
- FFmpeg (must be installed and available in PATH)

## Installation

### From cargo

```bash
cargo install stream-recorder
```

### From Source

```bash
git clone https://github.com/sn0w12/stream-recorder-rs.git
cd stream-recorder-rs
cargo build --release
```

## Quick Start

For additional help, run:

```bash
stream-recorder -h
```

1. **Install a platform**

    To begin recording streams, you need to either install or create a platform json file. If you are looking to create your own, read the [docs](./platforms/DOCS.md) or look into `/platforms/*` for examples.

    To install an already existing one, you can use the cli.

    ```bash
    stream-recorder platform install {url}
    ```

    To find existing platforms, you can use the search command.

    ```bash
    stream-recorder platform search
    ```

2. **Save a platform API token** (if required by the platform):

    Using keyring (recommended):

    ```bash
    stream-recorder token save-platform PLATFORM_ID YOUR_TOKEN_HERE
    ```

    Or define tokens in a `.env` file at `~/.config/stream-recorder/.env`:

    ```env
    PLATFORM_API_TOKEN=YOUR_TOKEN_HERE
    ```

3. **Configure monitored accounts** (use `platform_id:username` format):

    ```bash
    stream-recorder config add monitors platform1:user1
    stream-recorder config add monitors platform2:user2
    ```

4. **Set output directory (optional):**

    ```bash
    stream-recorder config set output_directory ./my_recordings
    ```

5. **Start monitoring:**
    ```bash
    stream-recorder
    ```

The tool will run continuously, monitoring for live streams and recording them automatically.

## Stream Recording Flow

The diagram below shows the main phases of a recording session and how the major parts of the system interact.

```mermaid
flowchart TD
    A[Start application] --> B[Load config and platforms]
    B --> C[Create monitor tasks]
    C --> D[Poll platform pipelines]

    D --> E{Stream live?}
    E -->|No| F[Sleep]
    F --> D

    E -->|Yes| G[Start recording]
    G --> H[Save stream segment to disk]
    H --> I{Continuation window enabled?}

    I -->|No| J[Post-process recording]
    I -->|Yes| K[Wait for stream continuation]
    K --> L{Stream resumes?}
    L -->|Yes| G
    L -->|No| M[Merge session segments]
    M --> J

    J --> N[Upload to enabled services]
    N --> D

    style A fill:#d9f2ff,stroke:#1f6f8b,color:#111
    style D fill:#fff4d6,stroke:#a36a00,color:#111
    style G fill:#e8f7e8,stroke:#2f7d32,color:#111
    style J fill:#ffe6e6,stroke:#b42318,color:#111
```

## Configuration

Configuration is stored in `~/.config/stream-recorder/config.toml` (Linux/macOS) or `%APPDATA%\stream-recorder\config.toml` (Windows).

The running recorder watches this file for changes. Most settings take effect the next time they are read, and changes to `monitors` automatically start or stop the affected monitor tasks.

### Setting Types

1. Duration
    - A time span used by settings like `step_delay`, `fetch_interval`, and retention windows.
    - Examples: `500ms`, `30s`, `5m`, `1h30m`, `1.5d 2h`

2. FileSize
    - A byte size used by settings like `min_free_space`.
    - Examples: `42`, `10KB`, `5MiB`, `1.5GB`

### Available Settings

#### Monitoring

| Setting                            | Description                                                                                            | Default |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------ | ------- |
| `monitors`                         | List of usernames to monitor                                                                           | `none`  |
| `min_stream_duration`              | Minimum recorded duration required before post-processing. Accepts values like 5m, 90s, or 1h.         | `none`  |
| `stream_reconnect_delay`           | How long to wait for a stream continuation before post-processing. Accepts values like 5m, 30s, or 1h. | `none`  |
| `stream_metadata_refresh_interval` | Refresh extracted stream metadata during active recordings. Accepts values like 30s, 5m, or 1h.        | `none`  |
| `step_delay`                       | Delay between each step in a platform. Accepts values like 500ms, 2s, or 1m.                           | `500ms` |
| `fetch_interval`                   | How often monitors are fetched. Accepts values like 30s, 2m, or 1h.                                    | `2m`    |

#### Video

| Setting         | Description                                                                                                    | Default |
| --------------- | -------------------------------------------------------------------------------------------------------------- | ------- |
| `video_quality` | Quality target for variable bitrate video encoding (lower is better)                                           | `26`    |
| `video_bitrate` | Constant video bitrate for CBR encoding (e.g. 6M, 5000k). When set, uses CBR mode and overrides video_quality. | `none`  |
| `max_bitrate`   | Maximum video bitrate (e.g. 6M, 2500k). When set, adds -maxrate and -bufsize to ffmpeg                         | `none`  |
| `max_fps`       | Maximum framerate a stream will be recorded at.                                                                | `none`  |

#### Post Processing

| Setting             | Description                                                                | Default |
| ------------------- | -------------------------------------------------------------------------- | ------- |
| `title_clean_regex` | Global regular expressions used to clean stream titles for uploader naming | `none`  |

#### Uploads

| Setting              | Description                            | Default |
| -------------------- | -------------------------------------- | ------- |
| `max_upload_retries` | Maximum number of upload retries       | `3`     |
| `disabled_uploaders` | List of uploaders to skip uploading to | `none`  |

#### Thumbnails

| Setting          | Description                                                | Default   |
| ---------------- | ---------------------------------------------------------- | --------- |
| `thumbnail_size` | Size of each thumbnail in the grid, in WIDTHxHEIGHT format | `320x180` |
| `thumbnail_grid` | Grid layout for thumbnails, in COLSxROWS format            | `3x3`     |

#### Notifications

| Setting                            | Description                             | Default |
| ---------------------------------- | --------------------------------------- | ------- |
| `discord_webhook_url`              | Discord webhook URL for notifications   | `none`  |
| `upload_complete_message_template` | Template for upload completion messages | `none`  |

#### Storage

| Setting                          | Description                                                                 | Default        |
| -------------------------------- | --------------------------------------------------------------------------- | -------------- |
| `output_directory`               | Directory to save recordings                                                | `./recordings` |
| `min_free_space`                 | Minimum free disk space before cleanup (e.g. 20GB, 500MB)                   | `20GB`         |
| `retention_max_age`              | Delete recordings older than this age. Accepts values like 7d, 48h, or 14d. | `none`         |
| `retention_keep_latest_per_user` | Keep only this many of the newest recordings per user                       | `none`         |

### Configuration Commands

```bash
# View all configuration
stream-recorder config get

# Get specific setting
stream-recorder config get output_directory

# Set a configuration value
stream-recorder config set output_directory /path/to/recordings

# Reset a configuration value to default
stream-recorder config reset output_directory

# Get config file path
stream-recorder config get-path

# Print the configuration tables in README/markdown format
stream-recorder config md
```

## Features

### Discord Integration

Set up Discord notifications:

1. Create a webhook in your Discord server
2. Set the webhook channel to a **forum** channel. It will not work with a normal text channel.
3. Set the webhook URL:
    ```bash
    stream-recorder config set discord_webhook_url https://discord.com/api/webhooks/YOUR_WEBHOOK_ID/YOUR_WEBHOOK_TOKEN
    ```

The tool will send notifications when:

- Recording starts
- Recording completes
- Uploads complete _(using your template)_

Each monitor will create its own thread, keeping all streams organized.

### Template System

Templates are rendered using [Handlebars](https://handlebarsjs.com/), a powerful templating engine. You can use all standard Handlebars features: variables, conditionals, loops, and block helpers. See the [Handlebars documentation](https://handlebarsjs.com/guide/) for syntax and advanced usage.

#### Built-in Helpers

The following helpers are registered and available in all templates:

| Helper  | Description                                                                                  |
| ------- | -------------------------------------------------------------------------------------------- |
| `add`   | Adds two numbers. <br>Usage: `{{add a b}}`                                                   |
| `sub`   | Subtract two numbers. <br>Usage: `{{sub a b}}`                                               |
| `gt`    | Returns true if first number is greater than second. <br>Usage: `{{#if (gt a b)}}...{{/if}}` |
| `lt`    | Returns true if first number is lesser than second. <br>Usage: `{{#if (lt a b)}}...{{/if}}`  |
| `ne`    | Returns true if two values are not equal. <br>Usage: `{{#if (ne a b)}}...{{/if}}`            |
| `eq`    | Returns true if two values are equal. <br>Usage: `{{#if (eq a b)}}...{{/if}}`                |
| `lower` | Converts a string to lowercase. <br>Usage: `{{lower username}}`                              |
| `upper` | Converts a string to uppercase. <br>Usage: `{{upper stream_title}}`                          |

For real-world usage, see the example template: [templates/example.hbr](templates/example.hbr)

#### Template Variables

The following variables are available in the template context:

| Variable              | Type   | Description                                          |
| --------------------- | ------ | ---------------------------------------------------- |
| `date`                | String | Current date (YYYY-MM-DD)                            |
| `username`            | String | Streamer's username                                  |
| `output_path`         | String | Path to recorded video file                          |
| `thumbnail_path`      | String | Path to generated thumbnail                          |
| `stream_title`        | String | Title of the stream                                  |
| `duration`            | String | Duration of the stream                               |
| `file_size`           | String | File size of the stream                              |
| `<uploader>_urls`     | Array  | Array of an uploaders uploaded URLs                  |
| `<uploader>_urls_len` | Number | Length of any array variable (e.g. `bunkr_urls_len`) |

#### Testing Templates

Render a test message with mock data:

```bash
stream-recorder template render
```

### Upload Services

The tool supports uploading to multiple services. Tokens are stored securely and used automatically when available.

| Uploader  | Requires Token | Notes                                                                         |
| --------- | :------------: | ----------------------------------------------------------------------------- |
| Bunkr     |      Yes       | API token required; supports album/folder lookups.                            |
| GoFile    |      Yes       | API token supported; configurable server and folder ID.                       |
| Fileditch |       No       | Public file hosting uploader.                                                 |
| Filester  |       No       | Public file hosting uploader; supports album/folder lookups if token provided |
| JPG6      |      Yes       | Login token must be stored as `USERNAME:PASSWORD`.                            |

### File Organization

Recordings are organized as follows:

```
output_directory/
├── username1/
│   ├── username1_2025-01-01_12-00-00.mp4
│   ├── username1_2025-01-01_12-00-00_thumb.jpg
│   └── ...
└── username2/
    └── ...
```

### Disk Space Management

The tool automatically manages disk space by:

- Removing recordings older than `retention_max_age`
- Keeping only the newest `retention_keep_latest_per_user` recordings per user
- Monitoring free space in the output directory
- Deleting oldest recordings when space falls below `min_free_space_gb`
- Removing associated thumbnail files
