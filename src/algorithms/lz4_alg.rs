//! LZ4 compression algorithm wrapper
//! Best for: speed-critical scenarios, real-time compression, logs

use anyhow::Result;
use super::CompressionAlgorithm;

pub struct Lz4Algorithm;

impl Lz4Algorithm {
    pub fn new() -> Self {
        Self
    }
}

impl CompressionAlgorithm for Lz4Algorithm {
    fn name(&self) -> &'static str {
        "LZ4"
    }

    fn compress(&self, data: &[u8], _level: u32) -> Result<Vec<u8>> {
        // lz4_flex doesn't support compression levels, always max speed
        let compressed = lz4_flex::compress_prepend_size(data);
        Ok(compressed)
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        lz4_flex::decompress_size_prepended(data)
            .map_err(|e| anyhow::anyhow!("LZ4 decompression failed: {}", e))
    }

    fn default_level(&self) -> u32 {
        1
    }

    fn max_level(&self) -> u32 {
        1 // LZ4 doesn't have compression levels in lz4_flex
    }
}
