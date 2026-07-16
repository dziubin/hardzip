//! Chunk processing — splits files into chunks and processes them in parallel

use anyhow::{Context, Result};
use rayon::prelude::*;
use crc32fast::Hasher;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::core::compressor::get_algorithm;
use crate::core::crypto::CryptoEngine;
use crate::core::format::{Algorithm, ChunkHeader};

/// Result of compressing a single chunk
#[derive(Debug)]
pub struct CompressedChunk {
    pub header: ChunkHeader,
    pub data: Vec<u8>,
}

/// Configuration for chunk processing
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Size of each chunk in bytes
    pub chunk_size: u64,
    /// Algorithm to use (Auto = per-chunk detection)
    pub algorithm: Algorithm,
    /// Compression level
    pub level: u32,
    /// Optional crypto engine for encryption
    pub encrypt: bool,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: crate::core::format::DEFAULT_CHUNK_SIZE,
            algorithm: Algorithm::Zstd,
            level: 19,
            encrypt: false,
        }
    }
}

/// Splits data into chunks of the configured size
pub fn split_into_chunks(data: &[u8], chunk_size: u64) -> Vec<&[u8]> {
    let chunk_size = chunk_size as usize;
    if chunk_size == 0 {
        return vec![data];
    }
    data.chunks(chunk_size).collect()
}

/// Compresses multiple chunks in parallel using rayon
pub fn compress_chunks_parallel(
    chunks: &[&[u8]],
    config: &ChunkConfig,
    crypto: Option<&CryptoEngine>,
    progress_callback: Option<&dyn Fn(u64)>,
) -> Result<Vec<CompressedChunk>> {
    let processed_bytes = Arc::new(AtomicU64::new(0));

    let results: Vec<Result<CompressedChunk>> = chunks
        .par_iter()
        .enumerate()
        .map(|(index, chunk_data)| {
            let result = compress_single_chunk(
                chunk_data,
                index as u32,
                config,
                crypto,
            );

            // Update progress
            let bytes = chunk_data.len() as u64;
            processed_bytes.fetch_add(bytes, Ordering::Relaxed);

            result
        })
        .collect();

    // Report final progress
    if let Some(cb) = progress_callback {
        cb(processed_bytes.load(Ordering::Relaxed));
    }

    // Collect results, propagating any errors
    results.into_iter().collect()
}

/// Compresses a single chunk
fn compress_single_chunk(
    data: &[u8],
    chunk_index: u32,
    config: &ChunkConfig,
    crypto: Option<&CryptoEngine>,
) -> Result<CompressedChunk> {
    // Determine algorithm
    let algo = if config.algorithm == Algorithm::Auto {
        Algorithm::Lzma // default to LZMA2 for best ratio
    } else {
        config.algorithm
    };

    // CRC32 of uncompressed data
    let crc_uncompressed = compute_crc32(data);

    // For Auto mode: try LZMA2 and Zstd-max, pick smaller result
    let (mut compressed, actual_algo) = if config.algorithm == Algorithm::Auto {
        let lzma = get_algorithm(Algorithm::Lzma);
        let zstd = get_algorithm(Algorithm::Zstd);

        let lzma_result = lzma.compress(data, 9);
        let zstd_result = zstd.compress(data, 22);

        match (lzma_result, zstd_result) {
            (Ok(l), Ok(z)) => {
                if l.len() <= z.len() { (l, Algorithm::Lzma) } else { (z, Algorithm::Zstd) }
            }
            (Ok(l), Err(_)) => (l, Algorithm::Lzma),
            (Err(_), Ok(z)) => (z, Algorithm::Zstd),
            (Err(e), Err(_)) => return Err(e),
        }
    } else {
        let compressor = get_algorithm(algo);
        let result = compressor.compress(data, config.level)
            .with_context(|| format!("Failed to compress chunk {}", chunk_index))?;
        (result, algo)
    };

    // If compressed is larger than original, store uncompressed
    let final_algo = if compressed.len() >= data.len() {
        compressed = data.to_vec();
        Algorithm::None
    } else {
        actual_algo
    };

    // Encrypt if needed
    let nonce = if let Some(engine) = crypto {
        let (encrypted, nonce) = engine.encrypt(&compressed)
            .with_context(|| format!("Failed to encrypt chunk {}", chunk_index))?;
        compressed = encrypted;
        nonce
    } else {
        [0u8; 12]
    };

    // CRC32 of final (compressed/encrypted) data
    let crc_compressed = compute_crc32(&compressed);

    let header = ChunkHeader {
        chunk_index,
        algorithm: final_algo,
        uncompressed_size: data.len() as u64,
        compressed_size: compressed.len() as u64,
        crc32_uncompressed: crc_uncompressed,
        crc32_compressed: crc_compressed,
        nonce,
    };

    Ok(CompressedChunk {
        header,
        data: compressed,
    })
}

/// Decompresses a single chunk
pub fn decompress_chunk(
    compressed_data: &[u8],
    header: &ChunkHeader,
    crypto: Option<&CryptoEngine>,
) -> Result<Vec<u8>> {
    // Verify CRC of compressed data
    let crc = compute_crc32(compressed_data);
    if crc != header.crc32_compressed {
        anyhow::bail!(
            "Chunk {} integrity check failed: CRC32 mismatch on compressed data",
            header.chunk_index
        );
    }

    // Decrypt if needed
    let data = if let Some(engine) = crypto {
        engine.decrypt(compressed_data, &header.nonce)
            .with_context(|| format!("Failed to decrypt chunk {}", header.chunk_index))?
    } else {
        compressed_data.to_vec()
    };

    // Decompress
    let decompressor = get_algorithm(header.algorithm);
    let decompressed = decompressor.decompress(&data)
        .with_context(|| format!("Failed to decompress chunk {}", header.chunk_index))?;

    // Verify CRC of decompressed data
    let crc = compute_crc32(&decompressed);
    if crc != header.crc32_uncompressed {
        anyhow::bail!(
            "Chunk {} integrity check failed: CRC32 mismatch on decompressed data",
            header.chunk_index
        );
    }

    Ok(decompressed)
}

/// Computes CRC32 checksum
pub fn compute_crc32(data: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize()
}
