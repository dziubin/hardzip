//! Foreign archive format support — extract ZIP, tar, tar.gz, 7z
//!
//! HardZIP can open and extract archives from competing formats.

use anyhow::{Context, Result};
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::Path;

/// Supported foreign archive formats
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ForeignFormat {
    Zip,
    TarGz,
    TarBz2,
    TarXz,
    Tar,
    SevenZ,
    Gzip,
    Bzip2,
    Xz,
}

/// Detects archive format from file extension
pub fn detect_format(path: &Path) -> Option<ForeignFormat> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();

    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        return Some(ForeignFormat::TarGz);
    }
    if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") {
        return Some(ForeignFormat::TarBz2);
    }
    if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        return Some(ForeignFormat::TarXz);
    }

    match path.extension()?.to_string_lossy().to_lowercase().as_str() {
        "zip" | "jar" | "apk" | "docx" | "xlsx" => Some(ForeignFormat::Zip),
        "tar" => Some(ForeignFormat::Tar),
        "gz" | "gzip" => Some(ForeignFormat::Gzip),
        "bz2" | "bzip2" => Some(ForeignFormat::Bzip2),
        "xz" => Some(ForeignFormat::Xz),
        "7z" => Some(ForeignFormat::SevenZ),
        "tgz" => Some(ForeignFormat::TarGz),
        "tbz2" => Some(ForeignFormat::TarBz2),
        "txz" => Some(ForeignFormat::TarXz),
        _ => None,
    }
}

/// Checks if we can handle this file (either .hza or foreign format)
pub fn is_supported_archive(path: &Path) -> bool {
    crate::utils::fs::is_hza_file(path) || detect_format(path).is_some()
}

/// Lists the contents of a foreign archive without extracting.
/// Returns Vec of (filename, size, is_directory).
pub fn list_archive_contents(archive_path: &Path) -> Result<Vec<(String, u64, bool)>> {
    let format = detect_format(archive_path)
        .ok_or_else(|| anyhow::anyhow!("Unsupported format"))?;

    match format {
        ForeignFormat::Zip => list_zip_contents(archive_path),
        ForeignFormat::SevenZ => list_7z_contents(archive_path),
        ForeignFormat::TarGz => list_tar_gz_contents(archive_path),
        ForeignFormat::TarBz2 => list_tar_bz2_contents(archive_path),
        ForeignFormat::TarXz => list_tar_xz_contents(archive_path),
        ForeignFormat::Tar => list_tar_contents(archive_path),
        ForeignFormat::Gzip => {
            let stem = archive_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let size = std::fs::metadata(archive_path).map(|m| m.len()).unwrap_or(0);
            Ok(vec![(stem, size, false)])
        }
        ForeignFormat::Bzip2 => {
            let stem = archive_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let size = std::fs::metadata(archive_path).map(|m| m.len()).unwrap_or(0);
            Ok(vec![(stem, size, false)])
        }
        ForeignFormat::Xz => {
            let stem = archive_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let size = std::fs::metadata(archive_path).map(|m| m.len()).unwrap_or(0);
            Ok(vec![(stem, size, false)])
        }
    }
}

fn list_zip_contents(path: &Path) -> Result<Vec<(String, u64, bool)>> {
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file).context("Cannot read ZIP")?;
    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index_raw(i).context("Cannot read ZIP entry")?;
        let name = entry.name().to_string();
        let is_dir = entry.is_dir();
        let size = entry.size();
        if !name.is_empty() {
            entries.push((name, size, is_dir));
        }
    }
    Ok(entries)
}

fn list_7z_contents(path: &Path) -> Result<Vec<(String, u64, bool)>> {
    // sevenz-rust doesn't have a list-only API easily, so we just show the archive name
    // and indicate it needs extraction
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    Ok(vec![(format!("{} (7z archive)", name), size, false)])
}

fn list_tar_gz_contents(path: &Path) -> Result<Vec<(String, u64, bool)>> {
    let file = File::open(path)?;
    let gz = flate2::read::GzDecoder::new(std::io::BufReader::new(file));
    list_tar_entries(tar::Archive::new(gz))
}

fn list_tar_bz2_contents(path: &Path) -> Result<Vec<(String, u64, bool)>> {
    let file = File::open(path)?;
    let bz = bzip2::read::BzDecoder::new(std::io::BufReader::new(file));
    list_tar_entries(tar::Archive::new(bz))
}

fn list_tar_xz_contents(path: &Path) -> Result<Vec<(String, u64, bool)>> {
    let file = File::open(path)?;
    let xz = xz2::read::XzDecoder::new(std::io::BufReader::new(file));
    list_tar_entries(tar::Archive::new(xz))
}

fn list_tar_contents(path: &Path) -> Result<Vec<(String, u64, bool)>> {
    let file = File::open(path)?;
    list_tar_entries(tar::Archive::new(std::io::BufReader::new(file)))
}

fn list_tar_entries<R: std::io::Read>(mut archive: tar::Archive<R>) -> Result<Vec<(String, u64, bool)>> {
    let mut entries = Vec::new();
    for entry in archive.entries().context("Cannot read tar")? {
        let entry = entry?;
        let name = entry.path()?.to_string_lossy().to_string();
        let size = entry.size();
        let is_dir = entry.header().entry_type().is_dir();
        entries.push((name, size, is_dir));
    }
    Ok(entries)
}

/// Extracts a foreign archive to the given output directory
pub fn extract_foreign(
    archive_path: &Path,
    output_dir: &Path,
    progress_callback: Option<&dyn Fn(u64, u64, &str)>,
) -> Result<()> {
    let format = detect_format(archive_path)
        .ok_or_else(|| anyhow::anyhow!("Unsupported archive format: {}", archive_path.display()))?;

    fs::create_dir_all(output_dir)
        .with_context(|| format!("Cannot create output directory: {}", output_dir.display()))?;

    match format {
        ForeignFormat::Zip => extract_zip(archive_path, output_dir, progress_callback),
        ForeignFormat::TarGz => extract_tar_gz(archive_path, output_dir, progress_callback),
        ForeignFormat::TarBz2 => extract_tar_bz2(archive_path, output_dir, progress_callback),
        ForeignFormat::TarXz => extract_tar_xz(archive_path, output_dir, progress_callback),
        ForeignFormat::Tar => extract_tar(archive_path, output_dir, progress_callback),
        ForeignFormat::Gzip => extract_gzip(archive_path, output_dir),
        ForeignFormat::Bzip2 => extract_bzip2(archive_path, output_dir),
        ForeignFormat::Xz => extract_xz(archive_path, output_dir),
        ForeignFormat::SevenZ => extract_7z(archive_path, output_dir, progress_callback),
    }
}

// ─── ZIP extraction ─────────────────────────────────────────────────────────

fn extract_zip(
    archive_path: &Path,
    output_dir: &Path,
    progress_callback: Option<&dyn Fn(u64, u64, &str)>,
) -> Result<()> {
    let file = File::open(archive_path)
        .with_context(|| format!("Cannot open: {}", archive_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .context("Failed to read ZIP archive")?;

    let total = archive.len() as u64;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .context("Failed to read ZIP entry")?;

        let out_path = output_dir.join(entry.mangled_name());

        if let Some(cb) = progress_callback {
            cb(i as u64, total, &entry.name().to_string());
        }

        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&out_path)
                .with_context(|| format!("Cannot create: {}", out_path.display()))?;
            io::copy(&mut entry, &mut outfile)?;
        }
    }

    if let Some(cb) = progress_callback {
        cb(total, total, "Done");
    }
    Ok(())
}

// ─── tar.gz extraction ──────────────────────────────────────────────────────

fn extract_tar_gz(
    archive_path: &Path,
    output_dir: &Path,
    progress_callback: Option<&dyn Fn(u64, u64, &str)>,
) -> Result<()> {
    let file = File::open(archive_path)?;
    let gz = flate2::read::GzDecoder::new(BufReader::new(file));
    let mut archive = tar::Archive::new(gz);

    extract_tar_archive(&mut archive, output_dir, progress_callback)
}

// ─── Plain tar extraction ───────────────────────────────────────────────────

fn extract_tar(
    archive_path: &Path,
    output_dir: &Path,
    progress_callback: Option<&dyn Fn(u64, u64, &str)>,
) -> Result<()> {
    let file = File::open(archive_path)?;
    let mut archive = tar::Archive::new(BufReader::new(file));

    extract_tar_archive(&mut archive, output_dir, progress_callback)
}

fn extract_tar_archive<R: Read>(
    archive: &mut tar::Archive<R>,
    output_dir: &Path,
    progress_callback: Option<&dyn Fn(u64, u64, &str)>,
) -> Result<()> {
    let mut count: u64 = 0;
    for entry in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();

        if let Some(cb) = progress_callback {
            cb(count, count + 1, &path.display().to_string());
        }

        let out_path = output_dir.join(&path);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        entry.unpack_in(output_dir)?;
        count += 1;
    }

    if let Some(cb) = progress_callback {
        cb(count, count, "Done");
    }
    Ok(())
}

// ─── Gzip (single file) ────────────────────────────────────────────────────

fn extract_gzip(archive_path: &Path, output_dir: &Path) -> Result<()> {
    let file = File::open(archive_path)?;
    let mut gz = flate2::read::GzDecoder::new(BufReader::new(file));

    // Output filename = input without .gz
    let stem = archive_path.file_stem().unwrap_or_default();
    let out_path = output_dir.join(stem);

    let mut outfile = File::create(&out_path)?;
    io::copy(&mut gz, &mut outfile)?;

    Ok(())
}

// ─── 7z extraction ──────────────────────────────────────────────────────────

fn extract_7z(
    archive_path: &Path,
    output_dir: &Path,
    progress_callback: Option<&dyn Fn(u64, u64, &str)>,
) -> Result<()> {
    sevenz_rust::decompress_file(archive_path, output_dir)
        .map_err(|e| anyhow::anyhow!("7z extraction failed: {}", e))?;

    if let Some(cb) = progress_callback {
        cb(1, 1, "Done");
    }
    Ok(())
}

// ─── tar.bz2 extraction ─────────────────────────────────────────────────────

fn extract_tar_bz2(
    archive_path: &Path,
    output_dir: &Path,
    progress_callback: Option<&dyn Fn(u64, u64, &str)>,
) -> Result<()> {
    let file = File::open(archive_path)?;
    let bz = bzip2::read::BzDecoder::new(BufReader::new(file));
    let mut archive = tar::Archive::new(bz);
    extract_tar_archive(&mut archive, output_dir, progress_callback)
}

// ─── tar.xz extraction ──────────────────────────────────────────────────────

fn extract_tar_xz(
    archive_path: &Path,
    output_dir: &Path,
    progress_callback: Option<&dyn Fn(u64, u64, &str)>,
) -> Result<()> {
    let file = File::open(archive_path)?;
    let xz = xz2::read::XzDecoder::new(BufReader::new(file));
    let mut archive = tar::Archive::new(xz);
    extract_tar_archive(&mut archive, output_dir, progress_callback)
}

// ─── Bzip2 (single file) ────────────────────────────────────────────────────

fn extract_bzip2(archive_path: &Path, output_dir: &Path) -> Result<()> {
    let file = File::open(archive_path)?;
    let mut bz = bzip2::read::BzDecoder::new(BufReader::new(file));

    let stem = archive_path.file_stem().unwrap_or_default();
    let out_path = output_dir.join(stem);

    let mut outfile = File::create(&out_path)?;
    io::copy(&mut bz, &mut outfile)?;
    Ok(())
}

// ─── Xz (single file) ───────────────────────────────────────────────────────

fn extract_xz(archive_path: &Path, output_dir: &Path) -> Result<()> {
    let file = File::open(archive_path)?;
    let mut xz = xz2::read::XzDecoder::new(BufReader::new(file));

    let stem = archive_path.file_stem().unwrap_or_default();
    let out_path = output_dir.join(stem);

    let mut outfile = File::create(&out_path)?;
    io::copy(&mut xz, &mut outfile)?;
    Ok(())
}
