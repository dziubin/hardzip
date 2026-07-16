//! Archive logic — packing and unpacking .hza files
//!
//! Handles reading/writing the binary format, coordinating chunk compression,
//! and managing the file index.

use anyhow::{bail, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use xxhash_rust::xxh3::xxh3_64;
use walkdir::WalkDir;

use crate::core::chunk::{
    compress_chunks_parallel, decompress_chunk, split_into_chunks, ChunkConfig,
};
use crate::core::crypto::{generate_salt, CryptoEngine};
use crate::core::format::*;

/// Options for creating an archive
#[derive(Debug, Clone)]
pub struct ArchiveOptions {
    pub algorithm: Algorithm,
    pub level: u32,
    pub chunk_size: u64,
    pub password: Option<String>,
    pub encrypt_filenames: bool,
}

impl Default for ArchiveOptions {
    fn default() -> Self {
        Self {
            algorithm: Algorithm::Auto,
            level: 19,
            chunk_size: 64 * 1024 * 1024, // 64MB chunks for better ratio
            password: None,
            encrypt_filenames: false,
        }
    }
}

/// Information about an archive (for display)
#[derive(Debug, Clone)]
pub struct ArchiveInfo {
    pub header: FileHeader,
    pub files: Vec<FileEntry>,
    pub footer: Footer,
    pub compression_ratio: f64,
}

/// Creates a .hza archive from a list of input paths (files and/or directories)
pub fn create_archive(
    output_path: &Path,
    input_paths: &[PathBuf],
    options: &ArchiveOptions,
    progress_callback: Option<&dyn Fn(u64, u64, &str)>,
) -> Result<()> {
    // Collect all files to archive
    let file_list = collect_files(input_paths)?;

    if file_list.is_empty() {
        bail!("No files to archive");
    }

    // Setup encryption if password provided
    let salt = generate_salt();
    let crypto = if let Some(ref password) = options.password {
        Some(CryptoEngine::new(password, &salt)?)
    } else {
        None
    };

    // Calculate total size for progress
    let total_size: u64 = file_list.iter().map(|(_, size)| *size).sum();

    // Prepare header
    let mut header = FileHeader::new(options.algorithm, options.level as u8, options.chunk_size);
    header.file_count = file_list.len() as u32;
    header.total_uncompressed_size = total_size;

    if crypto.is_some() {
        header.encryption = EncryptionMode::Aes256Gcm;
        header.encryption_salt = salt;
        if options.encrypt_filenames {
            header.flags.set(ArchiveFlags::ENCRYPTED_NAMES);
        }
    }

    // Open output file
    let file = File::create(output_path)
        .with_context(|| format!("Cannot create archive: {}", output_path.display()))?;
    let mut writer = BufWriter::new(file);

    // Write placeholder header (will update later)
    write_header(&mut writer, &header)?;

    // Process each file
    let mut file_entries: Vec<FileEntry> = Vec::new();
    let mut chunk_index: u32 = 0;
    let mut total_compressed: u64 = 0;
    let mut processed_bytes: u64 = 0;

    let chunk_config = ChunkConfig {
        chunk_size: options.chunk_size,
        algorithm: options.algorithm,
        level: options.level,
        encrypt: crypto.is_some(),
    };

    for (file_path, file_size) in &file_list {
        // Report progress
        if let Some(cb) = progress_callback {
            let name = file_path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            cb(processed_bytes, total_size, &name);
        }

        // Get file metadata
        let metadata = fs::metadata(file_path)
            .with_context(|| format!("Cannot read metadata: {}", file_path.display()))?;

        let modified_time = metadata.modified()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
            .unwrap_or(0);

        let created_time = metadata.created()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
            .unwrap_or(0);

        // Relative path for storage
        let relative_path = compute_relative_path(file_path, input_paths);

        if metadata.is_dir() {
            file_entries.push(FileEntry {
                path: relative_path,
                original_size: 0,
                permissions: 0o755,
                modified_time,
                created_time,
                is_directory: true,
                first_chunk_index: 0,
                chunk_count: 0,
                xxh3_hash: 0,
            });
            continue;
        }

        // Read file data
        let data = fs::read(file_path)
            .with_context(|| format!("Cannot read file: {}", file_path.display()))?;

        let file_hash = xxh3_64(&data);

        // Split into chunks and compress
        let chunks = split_into_chunks(&data, options.chunk_size);
        let first_chunk = chunk_index;

        let compressed_chunks = compress_chunks_parallel(
            &chunks,
            &chunk_config,
            crypto.as_ref(),
            None,
        )?;

        // Write chunks
        for mut compressed in compressed_chunks {
            // Fix chunk index to be global
            compressed.header.chunk_index = chunk_index;
            write_chunk_header(&mut writer, &compressed.header)?;
            writer.write_all(&compressed.data)?;
            total_compressed += compressed.header.compressed_size;
            chunk_index += 1;
        }

        // Create file entry
        file_entries.push(FileEntry {
            path: relative_path,
            original_size: *file_size,
            permissions: 0o644,
            modified_time,
            created_time,
            is_directory: false,
            first_chunk_index: first_chunk,
            chunk_count: chunk_index - first_chunk,
            xxh3_hash: file_hash,
        });

        processed_bytes += file_size;
    }

    // Write file index
    let index_offset = writer.stream_position()?;
    let index_data = serialize_file_index(&file_entries, crypto.as_ref(), options.encrypt_filenames)?;
    writer.write_all(&index_data)?;
    let index_size = index_data.len() as u64;

    // Compute archive hash (we'll use a placeholder for now — real impl would hash everything)
    let archive_hash = xxh3_64(&index_data);

    // Write footer
    let footer = Footer {
        index_offset,
        index_size,
        archive_hash,
        magic: *MAGIC_BYTES,
        reserved: [0u8; 4],
    };
    write_footer(&mut writer, &footer)?;

    // Update header with final counts
    header.chunk_count = chunk_index;
    header.total_compressed_size = total_compressed;

    writer.seek(SeekFrom::Start(0))?;
    write_header(&mut writer, &header)?;

    writer.flush()?;

    // Final progress
    if let Some(cb) = progress_callback {
        cb(total_size, total_size, "Done");
    }

    Ok(())
}

/// Extracts a .hza archive to a destination directory
pub fn extract_archive(
    archive_path: &Path,
    output_dir: &Path,
    password: Option<&str>,
    progress_callback: Option<&dyn Fn(u64, u64, &str)>,
) -> Result<()> {
    let file = File::open(archive_path)
        .with_context(|| format!("Cannot open archive: {}", archive_path.display()))?;
    let mut reader = BufReader::new(file);

    // Read header
    let header = read_header(&mut reader)?;
    if !header.is_valid() {
        bail!("Invalid archive: magic bytes mismatch. Not a .hza file.");
    }

    // Setup decryption
    let crypto = if header.encryption != EncryptionMode::None {
        let pw = password.ok_or_else(|| anyhow::anyhow!("Archive is encrypted — password required"))?;
        Some(CryptoEngine::new(pw, &header.encryption_salt)?)
    } else {
        None
    };

    // Read footer to find index
    let _file_size = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::End(-(Footer::SIZE as i64)))?;
    let footer = read_footer(&mut reader)?;

    if !footer.is_valid() {
        bail!("Invalid archive: footer magic bytes mismatch. Archive may be corrupted.");
    }

    // Read file index
    reader.seek(SeekFrom::Start(footer.index_offset))?;
    let mut index_data = vec![0u8; footer.index_size as usize];
    reader.read_exact(&mut index_data)?;
    let file_entries = deserialize_file_index(&index_data, crypto.as_ref(), header.flags.has(ArchiveFlags::ENCRYPTED_NAMES))?;

    let total_size = header.total_uncompressed_size;
    let mut processed: u64 = 0;

    // Create output directory
    fs::create_dir_all(output_dir)
        .with_context(|| format!("Cannot create output directory: {}", output_dir.display()))?;

    // Seek back to start of chunks (after header)
    reader.seek(SeekFrom::Start(FileHeader::SIZE as u64))?;

    // Read all chunks into memory map (index -> data)
    let mut chunk_map: Vec<(ChunkHeader, Vec<u8>)> = Vec::new();
    for _ in 0..header.chunk_count {
        let chunk_header = read_chunk_header(&mut reader)?;
        let mut chunk_data = vec![0u8; chunk_header.compressed_size as usize];
        reader.read_exact(&mut chunk_data)?;
        chunk_map.push((chunk_header, chunk_data));
    }

    // Extract each file
    for entry in &file_entries {
        if let Some(cb) = progress_callback {
            cb(processed, total_size, &entry.path);
        }

        let out_path = output_dir.join(&entry.path);

        if entry.is_directory {
            fs::create_dir_all(&out_path)?;
            continue;
        }

        // Ensure parent directory exists
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Decompress chunks for this file
        let mut file_data = Vec::with_capacity(entry.original_size as usize);
        for i in 0..entry.chunk_count {
            let global_idx = (entry.first_chunk_index + i) as usize;
            if global_idx >= chunk_map.len() {
                bail!("Archive corrupted: chunk index {} out of range", global_idx);
            }
            let (ref ch, ref data) = chunk_map[global_idx];
            let decompressed = decompress_chunk(data, ch, crypto.as_ref())?;
            file_data.extend_from_slice(&decompressed);
        }

        // Verify hash
        let hash = xxh3_64(&file_data);
        if hash != entry.xxh3_hash {
            bail!(
                "Integrity check failed for '{}': XXH3 hash mismatch",
                entry.path
            );
        }

        // Truncate to original size (last chunk may have padding)
        file_data.truncate(entry.original_size as usize);

        // Write file
        fs::write(&out_path, &file_data)
            .with_context(|| format!("Cannot write file: {}", out_path.display()))?;

        processed += entry.original_size;
    }

    if let Some(cb) = progress_callback {
        cb(total_size, total_size, "Done");
    }

    Ok(())
}

/// Reads archive info without extracting
pub fn read_archive_info(archive_path: &Path, password: Option<&str>) -> Result<ArchiveInfo> {
    let file = File::open(archive_path)
        .with_context(|| format!("Cannot open archive: {}", archive_path.display()))?;
    let mut reader = BufReader::new(file);

    let header = read_header(&mut reader)?;
    if !header.is_valid() {
        bail!("Not a valid .hza archive");
    }

    let crypto = if header.encryption != EncryptionMode::None {
        let pw = password.ok_or_else(|| anyhow::anyhow!("Archive is encrypted — password required"))?;
        Some(CryptoEngine::new(pw, &header.encryption_salt)?)
    } else {
        None
    };

    // Read footer
    reader.seek(SeekFrom::End(-(Footer::SIZE as i64)))?;
    let footer = read_footer(&mut reader)?;

    // Read index
    reader.seek(SeekFrom::Start(footer.index_offset))?;
    let mut index_data = vec![0u8; footer.index_size as usize];
    reader.read_exact(&mut index_data)?;
    let files = deserialize_file_index(&index_data, crypto.as_ref(), header.flags.has(ArchiveFlags::ENCRYPTED_NAMES))?;

    let ratio = if header.total_uncompressed_size > 0 {
        header.total_compressed_size as f64 / header.total_uncompressed_size as f64
    } else {
        0.0
    };

    Ok(ArchiveInfo {
        header,
        files,
        footer,
        compression_ratio: ratio,
    })
}

// ─── Binary I/O helpers ─────────────────────────────────────────────────────

fn write_header(writer: &mut impl Write, header: &FileHeader) -> Result<()> {
    writer.write_all(&header.magic)?;
    writer.write_u16::<LittleEndian>(header.version)?;
    writer.write_u8(header.algorithm as u8)?;
    writer.write_u8(header.level)?;
    writer.write_u8(header.encryption as u8)?;
    writer.write_u16::<LittleEndian>(header.flags.0)?;
    writer.write_u64::<LittleEndian>(header.chunk_size)?;
    writer.write_u32::<LittleEndian>(header.file_count)?;
    writer.write_u32::<LittleEndian>(header.chunk_count)?;
    writer.write_u64::<LittleEndian>(header.total_uncompressed_size)?;
    writer.write_u64::<LittleEndian>(header.total_compressed_size)?;
    writer.write_all(&header.encryption_salt)?;
    writer.write_all(&header.reserved)?;
    Ok(())
}

fn read_header(reader: &mut impl Read) -> Result<FileHeader> {
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    let version = reader.read_u16::<LittleEndian>()?;
    let algorithm = Algorithm::from_u8(reader.read_u8()?).unwrap_or(Algorithm::Zstd);
    let level = reader.read_u8()?;
    let encryption = EncryptionMode::from_u8(reader.read_u8()?).unwrap_or(EncryptionMode::None);
    let flags = ArchiveFlags(reader.read_u16::<LittleEndian>()?);
    let chunk_size = reader.read_u64::<LittleEndian>()?;
    let file_count = reader.read_u32::<LittleEndian>()?;
    let chunk_count = reader.read_u32::<LittleEndian>()?;
    let total_uncompressed_size = reader.read_u64::<LittleEndian>()?;
    let total_compressed_size = reader.read_u64::<LittleEndian>()?;
    let mut encryption_salt = [0u8; 16];
    reader.read_exact(&mut encryption_salt)?;
    let mut reserved = [0u8; 5];
    reader.read_exact(&mut reserved)?;

    Ok(FileHeader {
        magic,
        version,
        algorithm,
        level,
        encryption,
        flags,
        chunk_size,
        file_count,
        chunk_count,
        total_uncompressed_size,
        total_compressed_size,
        encryption_salt,
        reserved,
    })
}

fn write_chunk_header(writer: &mut impl Write, ch: &ChunkHeader) -> Result<()> {
    writer.write_u32::<LittleEndian>(ch.chunk_index)?;
    writer.write_u8(ch.algorithm as u8)?;
    writer.write_u64::<LittleEndian>(ch.uncompressed_size)?;
    writer.write_u64::<LittleEndian>(ch.compressed_size)?;
    writer.write_u32::<LittleEndian>(ch.crc32_uncompressed)?;
    writer.write_u32::<LittleEndian>(ch.crc32_compressed)?;
    writer.write_all(&ch.nonce)?;
    Ok(())
}

fn read_chunk_header(reader: &mut impl Read) -> Result<ChunkHeader> {
    let chunk_index = reader.read_u32::<LittleEndian>()?;
    let algorithm = Algorithm::from_u8(reader.read_u8()?).unwrap_or(Algorithm::None);
    let uncompressed_size = reader.read_u64::<LittleEndian>()?;
    let compressed_size = reader.read_u64::<LittleEndian>()?;
    let crc32_uncompressed = reader.read_u32::<LittleEndian>()?;
    let crc32_compressed = reader.read_u32::<LittleEndian>()?;
    let mut nonce = [0u8; 12];
    reader.read_exact(&mut nonce)?;

    Ok(ChunkHeader {
        chunk_index,
        algorithm,
        uncompressed_size,
        compressed_size,
        crc32_uncompressed,
        crc32_compressed,
        nonce,
    })
}

fn write_footer(writer: &mut impl Write, footer: &Footer) -> Result<()> {
    writer.write_u64::<LittleEndian>(footer.index_offset)?;
    writer.write_u64::<LittleEndian>(footer.index_size)?;
    writer.write_u64::<LittleEndian>(footer.archive_hash)?;
    writer.write_all(&footer.magic)?;
    writer.write_all(&footer.reserved)?;
    Ok(())
}

fn read_footer(reader: &mut impl Read) -> Result<Footer> {
    let index_offset = reader.read_u64::<LittleEndian>()?;
    let index_size = reader.read_u64::<LittleEndian>()?;
    let archive_hash = reader.read_u64::<LittleEndian>()?;
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    let mut reserved = [0u8; 4];
    reader.read_exact(&mut reserved)?;

    Ok(Footer {
        index_offset,
        index_size,
        archive_hash,
        magic,
        reserved,
    })
}

// ─── File index serialization ───────────────────────────────────────────────

fn serialize_file_index(
    entries: &[FileEntry],
    crypto: Option<&CryptoEngine>,
    encrypt_names: bool,
) -> Result<Vec<u8>> {
    let json = serde_json::to_vec(entries)
        .context("Failed to serialize file index")?;

    if encrypt_names {
        if let Some(engine) = crypto {
            let (encrypted, nonce) = engine.encrypt(&json)?;
            // Prepend nonce to encrypted index
            let mut result = Vec::with_capacity(12 + encrypted.len());
            result.extend_from_slice(&nonce);
            result.extend_from_slice(&encrypted);
            return Ok(result);
        }
    }

    Ok(json)
}

fn deserialize_file_index(
    data: &[u8],
    crypto: Option<&CryptoEngine>,
    encrypted_names: bool,
) -> Result<Vec<FileEntry>> {
    let json_data = if encrypted_names {
        if let Some(engine) = crypto {
            if data.len() < 12 {
                bail!("Encrypted index data too short");
            }
            let mut nonce = [0u8; 12];
            nonce.copy_from_slice(&data[..12]);
            engine.decrypt(&data[12..], &nonce)?
        } else {
            bail!("Archive has encrypted filenames but no password provided");
        }
    } else {
        data.to_vec()
    };

    serde_json::from_slice(&json_data)
        .context("Failed to deserialize file index")
}

// ─── File collection helpers ────────────────────────────────────────────────

/// Collects all files from input paths (handles both files and directories)
fn collect_files(input_paths: &[PathBuf]) -> Result<Vec<(PathBuf, u64)>> {
    let mut files = Vec::new();

    for path in input_paths {
        if path.is_file() {
            let size = fs::metadata(path)?.len();
            files.push((path.clone(), size));
        } else if path.is_dir() {
            for entry in WalkDir::new(path).follow_links(true) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    let size = entry.metadata()?.len();
                    files.push((entry.into_path(), size));
                }
            }
        }
    }

    Ok(files)
}

/// Computes a relative path for storage in the archive
fn compute_relative_path(file_path: &Path, input_paths: &[PathBuf]) -> String {
    // Try to find the best base path
    for base in input_paths {
        if let Ok(relative) = file_path.strip_prefix(base.parent().unwrap_or(base)) {
            return relative.to_string_lossy().replace('\\', "/");
        }
    }

    // Fallback: just use the filename
    file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}
