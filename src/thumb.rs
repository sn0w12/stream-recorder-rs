use anyhow::Result;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// Creates a 3x3 grid thumbnail from a video file using ffmpeg
///
/// # Arguments
/// * `input_path` - Path to the input video file
/// * `output_path` - Path where the thumbnail will be saved
/// * `width` - Width of each thumbnail in the grid (default: 320)
/// * `height` - Height of each thumbnail in the grid (default: 180)
///
/// # Returns
/// Result indicating success or failure
pub async fn create_video_thumbnail_grid(
    input_path: &Path,
    output_path: &Path,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<()> {
    let width = width.unwrap_or(320);
    let height = height.unwrap_or(180);

    let duration = get_video_duration(input_path).await?;

    // Calculate 9 timestamps evenly distributed throughout the video
    // Skip the first and last 5% to avoid black frames at start/end
    let start_offset = duration * 0.05;
    let effective_duration = duration * 0.9;
    let step = effective_duration / 8.0;

    let timestamps: Vec<f64> = (0..9).map(|i| start_offset + step * i as f64).collect();

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

    create_grid_from_frames(&frame_paths, output_path, width, height).await?;
    drop(_guard);

    Ok(())
}

/// Gets the duration of a video file in seconds using ffprobe
async fn get_video_duration(input_path: &Path) -> Result<f64> {
    let output = Command::new("ffprobe")
        .args(&[
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

    let status = Command::new("ffmpeg")
        .args(&[
            "-ss",
            &timestamp_str,
            "-i",
            input_path.to_str().unwrap(),
            "-vf",
            &format!("scale={}:{}", width, height),
            "-frames:v",
            "1",
            "-q:v",
            "2",
            output_path.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to extract frame at {}s", timestamp));
    }

    Ok(())
}

/// Creates a 3x3 grid from 9 frame images
async fn create_grid_from_frames(
    frame_paths: &[std::path::PathBuf],
    output_path: &Path,
    frame_width: u32,
    frame_height: u32,
) -> Result<()> {
    if frame_paths.len() != 9 {
        return Err(anyhow::anyhow!(
            "Expected 9 frames, got {}",
            frame_paths.len()
        ));
    }

    let mut filter_parts = Vec::new();

    // Add each frame as an input
    for (i, _frame_path) in frame_paths.iter().enumerate() {
        filter_parts.push(format!(
            "[{}:v]scale={}:{}[v{}];",
            i, frame_width, frame_height, i
        ));
    }

    // Arrange in 3x3 grid
    filter_parts.push(format!(
        "[v0][v1][v2]hstack=3[top];[v3][v4][v5]hstack=3[middle];[v6][v7][v8]hstack=3[bottom];[top][middle][bottom]vstack=3[v]"
    ));

    let filter = filter_parts.join("");
    let mut cmd = Command::new("ffmpeg");

    for frame_path in frame_paths {
        cmd.arg("-i").arg(frame_path);
    }

    cmd.args(&[
        "-filter_complex",
        &filter,
        "-map",
        "[v]",
        "-q:v",
        "2",
        output_path.to_str().unwrap(),
    ]);

    let status = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to create grid thumbnail"));
    }

    Ok(())
}
