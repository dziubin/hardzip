//! Filesystem utility helpers

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Gets the total size of a file or directory (recursive)
pub fn get_total_size(path: &Path) -> Result<u64> {
    if path.is_file() {
        Ok(fs::metadata(path)?.len())
    } else if path.is_dir() {
        let mut total = 0u64;
        for entry in WalkDir::new(path).follow_links(true) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total += entry.metadata()?.len();
            }
        }
        Ok(total)
    } else {
        Ok(0)
    }
}

/// Counts files in a path (recursive for directories)
pub fn count_files(path: &Path) -> Result<usize> {
    if path.is_file() {
        Ok(1)
    } else if path.is_dir() {
        let count = WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count();
        Ok(count)
    } else {
        Ok(0)
    }
}

/// Generates output archive path from input path
/// e.g., "my_folder" -> "my_folder.hza"
/// e.g., "document.txt" -> "document.txt.hza"
pub fn generate_archive_name(input_path: &Path) -> PathBuf {
    let name = input_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();

    let parent = input_path.parent().unwrap_or(Path::new("."));
    parent.join(format!("{}.hza", name))
}

/// Generates output directory path for extraction
/// e.g., "archive.hza" -> "archive" (in current directory)
pub fn generate_extract_dir(archive_path: &Path) -> PathBuf {
    let stem = archive_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let parent = archive_path.parent().unwrap_or(Path::new("."));
    parent.join(stem)
}

/// Checks if a path has the .hza extension
pub fn is_hza_file(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext.to_string_lossy().to_lowercase() == "hza")
        .unwrap_or(false)
}

/// Validates that all input paths exist
pub fn validate_inputs(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        if !path.exists() {
            anyhow::bail!("Path does not exist: {}", path.display());
        }
    }
    Ok(())
}

/// Gets file extension (lowercase)
pub fn get_extension(path: &Path) -> String {
    path.extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
}

/// Ensures a directory exists, creating it if necessary
pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
    }
    Ok(())
}
