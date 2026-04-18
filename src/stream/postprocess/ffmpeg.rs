use anyhow::Result;
use std::process::{ExitStatus, Output, Stdio};
use tokio::process::Command;

pub async fn run_ffmpeg_output(args: &[String]) -> Result<Output> {
    let output = Command::new("ffmpeg")
        .args(args)
        .stderr(Stdio::piped())
        .output()
        .await?;
    Ok(output)
}

pub async fn run_ffmpeg_status(args: &[String]) -> Result<ExitStatus> {
    let status = Command::new("ffmpeg")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;
    Ok(status)
}
