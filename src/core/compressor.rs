//! Compressor dispatcher — selects algorithm based on file content or user choice

use anyhow::Result;
use crate::algorithms::{
    CompressionAlgorithm,
    brotli_alg::BrotliAlgorithm,
    lz4_alg::Lz4Algorithm,
    lzma_alg::LzmaAlgorithm,
    zstd_alg::ZstdAlgorithm,
};
use crate::core::format::Algorithm;

/// Returns the appropriate algorithm implementation for a given Algorithm enum
pub fn get_algorithm(algo: Algorithm) -> Box<dyn CompressionAlgorithm> {
    match algo {
        Algorithm::Zstd => Box::new(ZstdAlgorithm::new()),
        Algorithm::Lz4 => Box::new(Lz4Algorithm::new()),
        Algorithm::Brotli => Box::new(BrotliAlgorithm::new()),
        Algorithm::Lzma => Box::new(LzmaAlgorithm::new()),
        Algorithm::None => Box::new(NoneAlgorithm),
        Algorithm::Auto => Box::new(ZstdAlgorithm::new()), // fallback
    }
}

/// Auto-selects the best algorithm based on file content analysis
pub fn auto_select_algorithm(data: &[u8]) -> Algorithm {
    if data.is_empty() {
        return Algorithm::None;
    }

    // Check magic bytes to identify file type
    let file_type = detect_file_type(data);

    match file_type {
        FileType::Text | FileType::Json | FileType::Xml | FileType::Html => Algorithm::Lzma,
        FileType::Log | FileType::Csv => Algorithm::Lzma,
        FileType::Image | FileType::Video | FileType::Audio => Algorithm::None, // already compressed
        FileType::CompressedArchive => Algorithm::None, // already compressed
        FileType::Executable | FileType::Binary => Algorithm::Lzma,
        FileType::Unknown => {
            // Heuristic: check if data is mostly text
            let text_ratio = count_printable_ratio(data);
            if text_ratio > 0.85 {
                Algorithm::Lzma
            } else {
                Algorithm::Lzma
            }
        }
    }
}

/// Detected file type categories
#[derive(Debug, Clone, Copy, PartialEq)]
enum FileType {
    Text,
    Json,
    Xml,
    Html,
    Log,
    Csv,
    Image,
    Video,
    Audio,
    CompressedArchive,
    Executable,
    Binary,
    Unknown,
}

/// Detects file type from magic bytes and content heuristics
fn detect_file_type(data: &[u8]) -> FileType {
    if data.len() < 4 {
        return FileType::Unknown;
    }

    // Check magic bytes
    match &data[..4] {
        // Images
        [0x89, b'P', b'N', b'G'] => return FileType::Image,
        [0xFF, 0xD8, 0xFF, _] => return FileType::Image,
        [b'G', b'I', b'F', b'8'] => return FileType::Image,
        [b'R', b'I', b'F', b'F'] => {
            // Could be WEBP or WAV
            if data.len() >= 12 {
                if &data[8..12] == b"WEBP" {
                    return FileType::Image;
                }
                if &data[8..12] == b"WAVE" {
                    return FileType::Audio;
                }
            }
        }
        // Video
        [0x00, 0x00, 0x00, _] if data.len() >= 8 && &data[4..8] == b"ftyp" => {
            return FileType::Video;
        }
        // Audio
        [b'I', b'D', b'3', _] => return FileType::Audio,
        [0xFF, 0xFB, _, _] | [0xFF, 0xFA, _, _] => return FileType::Audio,
        [b'f', b'L', b'a', b'C'] => return FileType::Audio,
        [b'O', b'g', b'g', b'S'] => return FileType::Audio,
        // Archives (already compressed)
        [b'P', b'K', 0x03, 0x04] => return FileType::CompressedArchive, // ZIP
        [0x1F, 0x8B, _, _] => return FileType::CompressedArchive,       // gzip
        [b'R', b'a', b'r', b'!'] => return FileType::CompressedArchive, // RAR
        [0xFD, b'7', b'z', b'X'] => return FileType::CompressedArchive, // xz
        [0x28, 0xB5, 0x2F, 0xFD] => return FileType::CompressedArchive, // zstd
        [b'H', b'Z', b'A', 0x01] => return FileType::CompressedArchive, // our own format!
        // Executables
        [b'M', b'Z', _, _] => return FileType::Executable, // Windows PE
        [0x7F, b'E', b'L', b'F'] => return FileType::Executable, // Linux ELF
        // PDF
        [b'%', b'P', b'D', b'F'] => return FileType::Binary,
        _ => {}
    }

    // Check for text-based formats by content
    let sample = &data[..data.len().min(1024)];

    if sample.starts_with(b"{") || sample.starts_with(b"[") {
        if is_likely_json(sample) {
            return FileType::Json;
        }
    }

    if sample.starts_with(b"<?xml") || sample.starts_with(b"<xml") {
        return FileType::Xml;
    }

    if sample.starts_with(b"<!DOCTYPE") || sample.starts_with(b"<html") || sample.starts_with(b"<HTML") {
        return FileType::Html;
    }

    // Check if it looks like a log file (timestamps at line starts)
    if looks_like_log(sample) {
        return FileType::Log;
    }

    // Check if it looks like CSV
    if looks_like_csv(sample) {
        return FileType::Csv;
    }

    // General text detection
    let text_ratio = count_printable_ratio(sample);
    if text_ratio > 0.90 {
        return FileType::Text;
    }

    FileType::Unknown
}

fn count_printable_ratio(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let printable = data.iter().filter(|&&b| {
        b == b'\n' || b == b'\r' || b == b'\t' || (b >= 0x20 && b <= 0x7E)
            || b >= 0xC0 // UTF-8 continuation bytes
    }).count();
    printable as f64 / data.len() as f64
}

fn is_likely_json(data: &[u8]) -> bool {
    // Simple heuristic: starts with { or [, contains colons and quotes
    let s = String::from_utf8_lossy(data);
    s.contains(':') && s.contains('"')
}

fn looks_like_log(data: &[u8]) -> bool {
    let s = String::from_utf8_lossy(&data[..data.len().min(512)]);
    let lines: Vec<&str> = s.lines().take(5).collect();
    if lines.len() < 3 {
        return false;
    }
    // Check if lines start with timestamps like "2024-" or "[2024-" or contain common log patterns
    let timestamp_lines = lines.iter().filter(|l| {
        l.starts_with("20") || l.starts_with("[20") || l.contains("INFO")
            || l.contains("ERROR") || l.contains("WARN") || l.contains("DEBUG")
    }).count();
    timestamp_lines >= 2
}

fn looks_like_csv(data: &[u8]) -> bool {
    let s = String::from_utf8_lossy(&data[..data.len().min(1024)]);
    let lines: Vec<&str> = s.lines().take(5).collect();
    if lines.len() < 2 {
        return false;
    }
    // Check if lines have consistent comma counts
    let comma_counts: Vec<usize> = lines.iter().map(|l| l.matches(',').count()).collect();
    if comma_counts[0] == 0 {
        return false;
    }
    comma_counts.windows(2).all(|w| w[0] == w[1])
}

/// No-op "algorithm" for storing files without compression
struct NoneAlgorithm;

impl CompressionAlgorithm for NoneAlgorithm {
    fn name(&self) -> &'static str {
        "None (Store)"
    }

    fn compress(&self, data: &[u8], _level: u32) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn default_level(&self) -> u32 {
        0
    }

    fn max_level(&self) -> u32 {
        0
    }
}
