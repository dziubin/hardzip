//! LZMA2 compression algorithm wrapper
//! Best for: maximum compression ratio, archives for storage/distribution

use anyhow::{Context, Result};
use super::CompressionAlgorithm;

pub struct LzmaAlgorithm;

impl LzmaAlgorithm {
    pub fn new() -> Self {
        Self
    }
}

impl CompressionAlgorithm for LzmaAlgorithm {
    fn name(&self) -> &'static str {
        "LZMA2"
    }

    fn compress(&self, data: &[u8], level: u32) -> Result<Vec<u8>> {
        let _level = level.min(self.max_level());
        let mut output = Vec::new();

        // lzma-rs uses a simple compress API
        lzma_rs::lzma2_compress(
            &mut std::io::Cursor::new(data),
            &mut output,
        )
        .context("LZMA2 compression failed")?;

        Ok(output)
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        let mut cursor = std::io::Cursor::new(data);

        lzma_rs::lzma2_decompress(&mut cursor, &mut output)
            .context("LZMA2 decompression failed")?;

        Ok(output)
    }

    fn default_level(&self) -> u32 {
        6
    }

    fn max_level(&self) -> u32 {
        9
    }
}
