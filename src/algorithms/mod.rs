pub mod zstd_alg;
pub mod lz4_alg;
pub mod brotli_alg;
pub mod lzma_alg;

use anyhow::Result;

/// Common trait for all compression algorithms
pub trait CompressionAlgorithm: Send + Sync {
    fn name(&self) -> &'static str;
    fn compress(&self, data: &[u8], level: u32) -> Result<Vec<u8>>;
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>>;
    fn default_level(&self) -> u32;
    fn max_level(&self) -> u32;
}
