//! Brotli compression algorithm wrapper
//! Best for: text, JSON, XML, HTML, web content

use anyhow::{Context, Result};
use super::CompressionAlgorithm;

pub struct BrotliAlgorithm;

impl BrotliAlgorithm {
    pub fn new() -> Self {
        Self
    }
}

impl CompressionAlgorithm for BrotliAlgorithm {
    fn name(&self) -> &'static str {
        "Brotli"
    }

    fn compress(&self, data: &[u8], level: u32) -> Result<Vec<u8>> {
        let level = level.min(self.max_level());
        let mut output = Vec::new();
        let mut cursor = std::io::Cursor::new(data);

        // Buffer size = 4KB, lg_window_size = 22 (4MB window)
        let mut params = brotli::enc::BrotliEncoderParams::default();
        params.quality = level as i32;
        params.lgwin = 22;

        brotli::BrotliCompress(&mut cursor, &mut output, &params)
            .context("Brotli compression failed")?;

        Ok(output)
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        let mut cursor = std::io::Cursor::new(data);

        brotli::BrotliDecompress(&mut cursor, &mut output)
            .context("Brotli decompression failed")?;

        Ok(output)
    }

    fn default_level(&self) -> u32 {
        6
    }

    fn max_level(&self) -> u32 {
        11
    }
}
