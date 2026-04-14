use std::path::PathBuf;

use crate::{config::Config, stream::postprocess::thumb::create_video_thumbnail_grid};
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ThumbnailAction {
    /// Generate a thumbnail from a recorded video
    Generate {
        /// Path to the recorded video file
        #[arg(short, long)]
        input: String,
        /// Path to save the generated thumbnail image
        #[arg(short, long)]
        output: Option<String>,
        /// Size of the generated thumbnail (WIDTHxHEIGHT)
        #[arg(short, long)]
        size: Option<String>,
        /// Grid layout for the thumbnail (ROWSxCOLUMNS)
        #[arg(short, long)]
        grid: Option<String>,
    },
}

pub async fn handle_thumbnail_action(action: ThumbnailAction) -> anyhow::Result<()> {
    let config = Config::get();
    match action {
        ThumbnailAction::Generate {
            input,
            output,
            size,
            grid,
        } => {
            let video_path = PathBuf::from(input);
            let output_path = if let Some(out) = output {
                PathBuf::from(out)
            } else {
                video_path.with_extension("jpg")
            };
            let size_str = size.unwrap_or_else(|| config.get_thumbnail_size());
            let grid_str = grid.unwrap_or_else(|| config.get_thumbnail_grid());

            create_video_thumbnail_grid(&video_path, &output_path, &size_str, &grid_str).await?;
        }
    }
    Ok(())
}
