//! Zstandard compression algorithm wrapper
//! Best for: general-purpose files, good speed/ratio balance

use anyhow::{Context, Result};
use super::CompressionAlgorithm;

pub struct ZstdAlgorithm;

impl ZstdAlgorithm {
    pub fn new() -> Self {
        Self
    }
}

impl CompressionAlgorithm for ZstdAlgorithm {
    fn name(&self) -> &'static str {
        "Zstandard"
    }

    fn compress(&self, data: &[u8], level: u32) -> Result<Vec<u8>> {
        let level = level.min(self.max_level()) as i32;
        zstd::bulk::compress(data, level)
            .context("Zstd compression failed")
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Progressive capacity: try reasonable sizes
        let base = (data.len() * 8).max(64 * 1024);
        zstd::bulk::decompress(data, base)
            .or_else(|_| zstd::bulk::decompress(data, base * 4))
            .or_else(|_| zstd::bulk::decompress(data, 512 * 1024 * 1024))
            .context("Zstd decompression failed")
    }

    fn default_level(&self) -> u32 {
        19
    }

    fn max_level(&self) -> u32 {
        22
    }
}
