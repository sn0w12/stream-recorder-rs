use super::ffmpeg::run_ffmpeg_status;
use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

/// Creates a thumbnail grid from a video file using ffmpeg
///
/// # Arguments
/// * `input_path` - Path to the input video file
/// * `output_path` - Path where the thumbnail will be saved
/// * `size_str` - Size of each thumbnail in the format "WIDTHxHEIGHT" (default: "320x180")
/// * `grid_str` - Grid layout in the format "COLSxROWS" (default: "3x3")
///
/// # Returns
/// Result indicating success or failure
pub async fn create_video_thumbnail_grid(
    input_path: &Path,
    output_path: &Path,
    size_str: &str,
    grid_str: &str,
) -> Result<()> {
    let (width, height) = parse_thumbnail_string(size_str).unwrap_or((320, 180));
    let (cols, rows) = parse_thumbnail_string(grid_str).unwrap_or((3, 3));
    let total_frames =
        cols.checked_mul(rows)
            .ok_or_else(|| anyhow::anyhow!("Thumbnail grid is too large"))? as usize;

    if total_frames == 0 {
        return Err(anyhow::anyhow!(
            "Thumbnail grid must contain at least one frame"
        ));
    }

    let duration = get_video_duration(input_path).await?;

    let timestamps: Vec<f64> = if total_frames == 1 {
        vec![duration * 0.5]
    } else {
        // Calculate timestamps evenly distributed throughout the video.
        // Skip the first and last 5% to avoid black frames at start/end.
        let start_offset = duration * 0.05;
        let effective_duration = duration * 0.9;
        let step = effective_duration / (total_frames as f64 - 1.0);

        (0..total_frames)
            .map(|i| start_offset + step * i as f64)
            .collect()
    };

    let temp_dir = tempfile::tempdir()?;
    let temp_dir_path = temp_dir.path().to_path_buf();

    // Use a scope guard to ensure cleanup even if there's an error
    let _guard = scopeguard::guard(temp_dir_path.clone(), |path| {
        if path.exists() {
            let _ = std::fs::remove_dir_all(&path);
        }
    });

    let mut frame_paths = Vec::new();

    for (i, &timestamp) in timestamps.iter().enumerate() {
        let frame_path = temp_dir.path().join(format!("frame_{:02}.jpg", i));
        extract_frame_at_time(input_path, &frame_path, timestamp, width, height).await?;
        frame_paths.push(frame_path);
    }

    create_grid_from_frames(&frame_paths, output_path, cols, rows, width, height).await?;
    drop(_guard);

    Ok(())
}

/// Gets the duration of a video file in seconds using ffprobe
async fn get_video_duration(input_path: &Path) -> Result<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            input_path.to_str().unwrap(),
        ])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to get video duration"));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let duration_str = json["format"]["duration"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Could not parse duration"))?;

    duration_str
        .parse::<f64>()
        .map_err(|_| anyhow::anyhow!("Invalid duration format"))
}

/// Extracts a single frame at the specified timestamp
async fn extract_frame_at_time(
    input_path: &Path,
    output_path: &Path,
    timestamp: f64,
    width: u32,
    height: u32,
) -> Result<()> {
    let timestamp_str = format!("{:.3}", timestamp);
    let input = input_path.to_string_lossy().to_string();
    let output = output_path.to_string_lossy().to_string();
    let args = vec![
        "-ss".to_string(),
        timestamp_str,
        "-i".to_string(),
        input,
        "-vf".to_string(),
        format!("scale={}:{}", width, height),
        "-frames:v".to_string(),
        "1".to_string(),
        "-q:v".to_string(),
        "2".to_string(),
        output,
    ];

    let status = run_ffmpeg_status(&args).await?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to extract frame at {}s", timestamp));
    }

    Ok(())
}

/// Creates a grid thumbnail from extracted frame images
async fn create_grid_from_frames(
    frame_paths: &[std::path::PathBuf],
    output_path: &Path,
    cols: u32,
    rows: u32,
    frame_width: u32,
    frame_height: u32,
) -> Result<()> {
    let expected_frames = cols
        .checked_mul(rows)
        .ok_or_else(|| anyhow::anyhow!("Thumbnail grid is too large"))?;

    if expected_frames == 0 {
        return Err(anyhow::anyhow!(
            "Thumbnail grid must contain at least one frame"
        ));
    }

    if frame_paths.len() != expected_frames as usize {
        return Err(anyhow::anyhow!(
            "Expected {} frames, got {}",
            expected_frames,
            frame_paths.len()
        ));
    }

    if frame_paths.len() == 1 {
        tokio::fs::copy(&frame_paths[0], output_path).await?;
        return Ok(());
    }

    let mut filter_parts = Vec::new();
    let mut input_labels = Vec::new();

    for (i, _frame_path) in frame_paths.iter().enumerate() {
        filter_parts.push(format!(
            "[{}:v]scale={}:{}[v{}];",
            i, frame_width, frame_height, i
        ));
        input_labels.push(format!("[v{}]", i));
    }

    let layout = build_xstack_layout(cols, rows, frame_width, frame_height)?;
    filter_parts.push(format!(
        "{}xstack=inputs={}:layout={}[v]",
        input_labels.join(""),
        frame_paths.len(),
        layout
    ));

    let filter = filter_parts.join("");
    let mut args = Vec::new();
    for frame_path in frame_paths {
        args.push("-i".to_string());
        args.push(frame_path.to_string_lossy().to_string());
    }

    args.extend([
        "-filter_complex".to_string(),
        filter,
        "-map".to_string(),
        "[v]".to_string(),
        "-q:v".to_string(),
        "2".to_string(),
        output_path.to_string_lossy().to_string(),
    ]);

    let status = run_ffmpeg_status(&args).await?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to create grid thumbnail"));
    }

    Ok(())
}

fn build_xstack_layout(
    cols: u32,
    rows: u32,
    frame_width: u32,
    frame_height: u32,
) -> Result<String> {
    let mut layout = Vec::with_capacity((cols * rows) as usize);

    for row in 0..rows {
        for col in 0..cols {
            layout.push(format!("{}_{}", col * frame_width, row * frame_height));
        }
    }

    Ok(layout.join("|"))
}

/// Parses a thumbnail string in the format "XxY" (e.g., "3x3").
///
/// This is shared by thumbnail generation and config validation so invalid
/// values fail early when the config is loaded or edited.
pub fn parse_thumbnail_string(s: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        return None;
    }
    let col = parts[0].trim().parse::<u32>().ok()?;
    let row = parts[1].trim().parse::<u32>().ok()?;
    Some((col, row))
}
