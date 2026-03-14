# Stream Recorder

A high-performance CLI tool written in Rust for recording live streams from a variety of platforms. It automatically monitors specified accounts or channels, records streams with FFmpeg, generates thumbnails, uploads content to multiple file hosting services, and sends notifications via Discord webhooks.

## Features

- **Stream Monitoring**: Continuously monitor multiple accounts across supported platforms for live streams
- **Automatic Recording**: Record streams using FFmpeg with optimized settings for quality and performance
- **Thumbnail Generation**: Automatically create video thumbnail grids for recorded streams
- **Multi-Platform Uploads**: Upload recordings and thumbnails to Bunkr, GoFile, and JPG6
- **Discord Integration**: Send real-time notifications for recording start/completion via Discord webhooks
- **Disk Space Management**: Automatically clean up old recordings when disk space is low
- **Template System**: Customizable notification messages with template variables

## Prerequisites

- Rust
- FFmpeg (must be installed and available in PATH)

## Usage

To begin recording streams, you need to either install or create a platform json file. If you are looking to create your own, look into `/platforms/*` for examples.

To install an already existing one, you can use the cli.

```bash
stream-recorder platform install {url}
```

## Installation

### From Source

```bash
git clone https://github.com/sn0w12/stream-recorder-rs.git
cd stream-recorder-rs
cargo build --release
```

The binary will be available at `target/release/stream-recorder.exe` (Windows) or `target/release/stream-recorder` (Linux/macOS).

### Pre-built Binaries

Download the latest release from the [releases page](https://github.com/sn0w12/stream-recorder-rs/releases).

## Quick Start

For additional help, run:

```bash
stream-recorder -h
```

1. **Save a platform API token** (if required by the platform):

    Using keyring (recommended):

    ```bash
    stream-recorder token save-platform PLATFORM_ID YOUR_TOKEN_HERE
    ```

    Or define tokens in a `.env` file at `~/.config/stream-recorder/.env`:

    ```env
    PLATFORM_API_TOKEN=YOUR_TOKEN_HERE
    ```

2. **Configure monitored accounts** (use `platform_id:username` format):

    ```bash
    stream-recorder config monitors add platform1:user1
    stream-recorder config monitors add platform2:user2
    ```

3. **Set output directory (optional):**

    ```bash
    stream-recorder config set output_directory ./my_recordings
    ```

4. **Start monitoring:**
    ```bash
    stream-recorder --token YOUR_TOKEN
    ```
    Or if tokens are saved:
    ```bash
    stream-recorder
    ```

The tool will run continuously, monitoring for live streams and recording them automatically.

## Configuration

Configuration is stored in `~/.config/stream-recorder/config.toml` (Linux/macOS) or `%APPDATA%\stream-recorder\config.toml` (Windows).

### Available Settings

| Setting                            | Description                              | Default        |
| ---------------------------------- | ---------------------------------------- | -------------- |
| `output_directory`                 | Directory to save recordings             | `./recordings` |
| `monitors`                         | List of usernames to monitor             | None           |
| `discord_webhook_url`              | Discord webhook URL for notifications    | None           |
| `min_free_space_gb`                | Minimum free disk space before cleanup   | 20.0           |
| `upload_complete_message_template` | Template for upload completion messages  | None           |
| `max_upload_retries`               | Maximum number of upload retries         | 3              |
| `min_stream_duration`              | Minimum stream duration before recording | None           |
| `stream_reconnect_delay_minutes`   | Delay in minutes to wait for stream continuation before post-processing. Streams resumed within this window are merged into a single recording. | None |

### Configuration Commands

```bash
# View all configuration
stream-recorder config get

# Get specific setting
stream-recorder config get output_directory

# Set a configuration value
stream-recorder config set output_directory /path/to/recordings

# Get config file path
stream-recorder config get-path

# Manage monitored users
stream-recorder config monitors add username
stream-recorder config monitors remove username
stream-recorder config monitors list
```

## Token Management

Tokens can be stored in two ways:

### Option 1: System Keyring (Recommended)

Store tokens securely using the system keyring:

```bash
# Bunkr upload token
stream-recorder token save-bunkr YOUR_BUNKR_TOKEN

# GoFile upload token
stream-recorder token save-gofile YOUR_GOFILE_TOKEN
```

### Option 2: .env File

Alternatively, you can store tokens in a `.env` file located at `~/.config/stream-recorder/.env` (Linux/macOS) or `%APPDATA%\stream-recorder\.env` (Windows).

Create the file with the following format:

```env
BUNKR_TOKEN=your_bunkr_token_here
GOFILE_TOKEN=your_gofile_token_here
```

**Note:** If both keyring and .env file contain tokens, the keyring tokens will take precedence.

## Discord Integration

Set up Discord notifications:

1. Create a webhook in your Discord server
2. Set the webhook URL:
    ```bash
    stream-recorder config set discord_webhook_url https://discord.com/api/webhooks/YOUR_WEBHOOK_ID/YOUR_WEBHOOK_TOKEN
    ```

The tool will send notifications when:

- Recording starts
- Recording completes
- Uploads complete

## Template System

Customize notification messages using templates. Templates support variables, conditionals, and loops.

### Available Variables

- `{{date}}` - Current date (YYYY-MM-DD)
- `{{username}}` - Streamer's username
- `{{user_id}}` - Streamer's user ID
- `{{output_path}}` - Path to recorded video file
- `{{thumbnail_path}}` - Path to generated thumbnail
- `{{stream_title}}` - Title of the stream
- `{{bunkr_urls}}` - Array of Bunkr upload URLs
- `{{jpg6_urls}}` - Array of JPG6 upload URLs
- `{{gofile_urls}}` - Array of GoFile upload URLs

### Template Syntax

- **Variables**: `{{variable}}`
- **Conditionals**: `{{if condition: content}}`
- **Loops**: `{{for array: content with {{item}} and {{i}}}}`
- **Array length**: `{{array_len}}`

### Example Template

```
🎥 **Stream Recorded: {{stream_title}}**
👤 User: {{username}}
📅 Date: {{date}}

{{if bunkr_urls: **Bunkr Links:**
{{for bunkr_urls: • {{item}}
}}}}

{{if jpg6_urls: **JPG6 Links:**
{{for jpg6_urls: • {{item}}
}}}}

{{if gofile_urls: **GoFile Links:**
{{for gofile_urls: • {{item}}
}}}}
```

### Testing Templates

Render a test message with mock data:

```bash
stream-recorder template render
```

## Upload Services

The tool supports uploading to multiple services. Tokens are stored securely and used automatically when available.

### Bunkr

- Requires API token
- Uploads video files

### GoFile

- Requires API token
- Uploads video files

## File Organization

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

## Disk Space Management

The tool automatically manages disk space by:

- Monitoring free space in the output directory
- Deleting oldest recordings when space falls below `min_free_space_gb`
- Removing associated thumbnail files

## License

This project is licensed under the MIT License - see the LICENSE file for details.
