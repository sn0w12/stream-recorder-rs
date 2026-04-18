use super::StreamResult;
use crate::config::Config;
use crate::stream::postprocess::RecordingFile;
use crate::types::FileSize as FileSizeValue;
use fs2::available_space;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

pub async fn manage_disk_space() -> StreamResult<()> {
    let config = Config::get();
    let output_dir = config.get_output_directory();
    let output_dir_path = Path::new(&output_dir);
    let min_free_space = config.get_min_free_space();
    let min_free_bytes = min_free_space.as_bytes();

    let files = collect_recording_files(output_dir_path);
    if files.is_empty() {
        return Ok(());
    }

    let mut files_by_age = files.clone();
    files_by_age.sort_by_key(|file| file.modified);

    // Apply retention policies to determine which files to delete
    let mut planned_deletions = HashSet::new();
    if let Some(max_age_days) = config.get_retention_max_age_days() {
        let age_candidates = retention_age_candidates(&files, max_age_days, SystemTime::now());
        if !age_candidates.is_empty() {
            println!(
                "Applying age-based retention: deleting {} recording(s) older than {} day(s)...",
                age_candidates.len(),
                max_age_days
            );
        }
        planned_deletions.extend(age_candidates);
    }
    if let Some(keep_latest_per_user) = config.get_retention_keep_latest_per_user() {
        let keep_set = retention_keep_latest_per_user(&files, keep_latest_per_user);
        let per_user_count = files
            .iter()
            .filter(|file| {
                !keep_set.contains(&file.path) && !planned_deletions.contains(&file.path)
            })
            .count();
        if per_user_count > 0 {
            println!(
                "Applying per-user retention: keeping the newest {} recording(s) per user and deleting {} older file(s)...",
                keep_latest_per_user, per_user_count
            );
        }
        planned_deletions.extend(
            files
                .iter()
                .filter(|file| !keep_set.contains(&file.path))
                .map(|file| file.path.clone()),
        );
    }

    // Delete all retention-flagged files
    let mut attempted = HashSet::new();
    for file in &files_by_age {
        if planned_deletions.contains(&file.path) {
            attempted.insert(file.path.clone());
            delete_recording_assets(&file.path).await;
        }
    }

    // Free additional space if still below the minimum threshold
    if available_space(output_dir_path)? < min_free_bytes {
        println!(
            "Free space {} is below minimum {}, cleaning up old streams...",
            FileSizeValue::from_bytes(available_space(output_dir_path)?),
            min_free_space
        );
        for file in &files_by_age {
            if available_space(output_dir_path)? >= min_free_bytes {
                break;
            }
            if !attempted.contains(&file.path) {
                attempted.insert(file.path.clone());
                delete_recording_assets(&file.path).await;
            }
        }
    }

    Ok(())
}

pub async fn delete_recording_assets(recording_path: &Path) {
    if let Err(error) = tokio::fs::remove_file(recording_path).await {
        eprintln!(
            "Failed to delete video file {}: {}",
            recording_path.display(),
            error
        );
    } else {
        println!("Deleted video file: {}", recording_path.display());
    }

    if let Some(thumbnail_path) = recording_thumbnail_path(recording_path)
        && thumbnail_path.exists()
        && let Err(error) = tokio::fs::remove_file(&thumbnail_path).await
    {
        eprintln!(
            "Failed to delete thumbnail {}: {}",
            thumbnail_path.display(),
            error
        );
    }
}

fn recording_thumbnail_path(recording_path: &Path) -> Option<PathBuf> {
    recording_path
        .file_stem()
        .map(|stem| recording_path.with_file_name(format!("{}_thumb.jpg", stem.to_string_lossy())))
}

fn recording_user_key(output_dir: &Path, recording_path: &Path) -> String {
    recording_path
        .strip_prefix(output_dir)
        .ok()
        .and_then(|relative| relative.components().next())
        .and_then(|component| component.as_os_str().to_str())
        .map(|segment| segment.to_string())
        .or_else(|| {
            recording_path
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                .map(|segment| segment.to_string())
        })
        .unwrap_or_else(|| "__root__".to_string())
}

fn collect_recording_files(output_dir: &Path) -> Vec<RecordingFile> {
    WalkDir::new(output_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("mp4"))
        .filter_map(|entry| {
            let path = entry.path().to_path_buf();
            let metadata = std::fs::metadata(&path).ok()?;
            let modified = metadata.modified().ok()?;
            Some(RecordingFile {
                user_key: recording_user_key(output_dir, &path),
                path,
                modified,
            })
        })
        .collect()
}

fn retention_age_candidates(
    files: &[RecordingFile],
    max_age_days: u32,
    now: SystemTime,
) -> HashSet<PathBuf> {
    let age_limit = Duration::from_secs((max_age_days as u64).saturating_mul(24 * 60 * 60));
    let cutoff = now.checked_sub(age_limit).unwrap_or(SystemTime::UNIX_EPOCH);

    files
        .iter()
        .filter(|file| file.modified < cutoff)
        .map(|file| file.path.clone())
        .collect()
}

fn retention_keep_latest_per_user(
    files: &[RecordingFile],
    keep_latest_per_user: u32,
) -> HashSet<PathBuf> {
    let mut grouped: HashMap<String, Vec<&RecordingFile>> = HashMap::new();

    for file in files {
        grouped.entry(file.user_key.clone()).or_default().push(file);
    }

    let mut keep = HashSet::new();
    let keep_count = keep_latest_per_user as usize;

    for group in grouped.values_mut() {
        group.sort_by(|left, right| right.modified.cmp(&left.modified));
        for file in group.iter().take(keep_count) {
            keep.insert(file.path.clone());
        }
    }

    keep
}

#[cfg(test)]
mod tests {
    use super::{RecordingFile, retention_age_candidates, retention_keep_latest_per_user};
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    #[test]
    fn retention_keep_latest_per_user_keeps_newest_files_for_each_user() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        let files = vec![
            RecordingFile {
                path: PathBuf::from("recordings/alice/older.mp4"),
                modified: now - Duration::from_secs(300),
                user_key: "alice".to_string(),
            },
            RecordingFile {
                path: PathBuf::from("recordings/alice/newer.mp4"),
                modified: now - Duration::from_secs(100),
                user_key: "alice".to_string(),
            },
            RecordingFile {
                path: PathBuf::from("recordings/bob/only.mp4"),
                modified: now - Duration::from_secs(200),
                user_key: "bob".to_string(),
            },
        ];

        let keep = retention_keep_latest_per_user(&files, 1);

        assert!(keep.contains(&PathBuf::from("recordings/alice/newer.mp4")));
        assert!(keep.contains(&PathBuf::from("recordings/bob/only.mp4")));
        assert!(!keep.contains(&PathBuf::from("recordings/alice/older.mp4")));
    }

    #[test]
    fn retention_age_candidates_marks_files_below_cutoff() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        let files = vec![
            RecordingFile {
                path: PathBuf::from("recordings/old.mp4"),
                modified: now - Duration::from_secs(9 * 24 * 60 * 60),
                user_key: "old".to_string(),
            },
            RecordingFile {
                path: PathBuf::from("recordings/fresh.mp4"),
                modified: now - Duration::from_secs(2 * 24 * 60 * 60),
                user_key: "fresh".to_string(),
            },
        ];

        let candidates = retention_age_candidates(&files, 7, now);

        let expected = HashSet::from([PathBuf::from("recordings/old.mp4")]);
        assert_eq!(candidates, expected);
    }
}
