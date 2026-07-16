//! HardZIP Archive (.hza) Binary Format Specification
//!
//! Layout:
//! ┌──────────────────────────────────────────┐
//! │ FILE HEADER (fixed 64 bytes)             │
//! ├──────────────────────────────────────────┤
//! │ CHUNK 0: [ChunkHeader + CompressedData]  │
//! │ CHUNK 1: [ChunkHeader + CompressedData]  │
//! │ ...                                      │
//! │ CHUNK N: [ChunkHeader + CompressedData]  │
//! ├──────────────────────────────────────────┤
//! │ FILE INDEX (list of FileEntry)           │
//! ├──────────────────────────────────────────┤
//! │ FOOTER (fixed 32 bytes)                  │
//! └──────────────────────────────────────────┘

use serde::{Deserialize, Serialize};

/// Magic bytes identifying an .hza archive
pub const MAGIC_BYTES: &[u8; 4] = b"HZA\x01";

/// Current format version
pub const FORMAT_VERSION: u16 = 1;

/// Default chunk size: 64 MB
pub const DEFAULT_CHUNK_SIZE: u64 = 64 * 1024 * 1024;

/// Maximum chunk size: 256 MB
pub const MAX_CHUNK_SIZE: u64 = 256 * 1024 * 1024;

/// Minimum chunk size: 1 MB
pub const MIN_CHUNK_SIZE: u64 = 1024 * 1024;

/// Compression algorithm identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Algorithm {
    /// No compression (store only)
    None = 0,
    /// Zstandard — good balance of speed and ratio
    Zstd = 1,
    /// LZ4 — fastest compression/decompression
    Lz4 = 2,
    /// Brotli — best for text/web content
    Brotli = 3,
    /// LZMA2 — maximum compression ratio
    Lzma = 4,
    /// Auto-select based on file content
    Auto = 255,
}

impl Algorithm {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Algorithm::None),
            1 => Some(Algorithm::Zstd),
            2 => Some(Algorithm::Lz4),
            3 => Some(Algorithm::Brotli),
            4 => Some(Algorithm::Lzma),
            255 => Some(Algorithm::Auto),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Algorithm::None => "None (Store)",
            Algorithm::Zstd => "Zstandard",
            Algorithm::Lz4 => "LZ4",
            Algorithm::Brotli => "Brotli",
            Algorithm::Lzma => "LZMA2",
            Algorithm::Auto => "Auto-Select",
        }
    }
}

impl std::fmt::Display for Algorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Encryption mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum EncryptionMode {
    /// No encryption
    None = 0,
    /// AES-256-GCM with Argon2id key derivation
    Aes256Gcm = 1,
}

impl EncryptionMode {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(EncryptionMode::None),
            1 => Some(EncryptionMode::Aes256Gcm),
            _ => None,
        }
    }
}

/// Archive flags (bitfield)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveFlags(pub u16);

impl ArchiveFlags {
    pub const NONE: Self = Self(0);
    /// File names are encrypted
    pub const ENCRYPTED_NAMES: Self = Self(1 << 0);
    /// Archive uses solid mode (cross-file dedup)
    pub const SOLID: Self = Self(1 << 1);
    /// Archive contains recovery records
    pub const RECOVERY: Self = Self(1 << 2);
    /// Archive was split into volumes
    pub const MULTIVOLUME: Self = Self(1 << 3);

    pub fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn set(&mut self, flag: Self) {
        self.0 |= flag.0;
    }
}

/// File header — first 64 bytes of every .hza file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHeader {
    /// Magic bytes: "HZA\x01" (4 bytes)
    pub magic: [u8; 4],
    /// Format version (2 bytes)
    pub version: u16,
    /// Default algorithm used (1 byte)
    pub algorithm: Algorithm,
    /// Compression level (1 byte)
    pub level: u8,
    /// Encryption mode (1 byte)
    pub encryption: EncryptionMode,
    /// Archive flags (2 bytes)
    pub flags: ArchiveFlags,
    /// Chunk size in bytes (8 bytes)
    pub chunk_size: u64,
    /// Total number of files (4 bytes)
    pub file_count: u32,
    /// Total number of chunks (4 bytes)
    pub chunk_count: u32,
    /// Total uncompressed size of all files (8 bytes)
    pub total_uncompressed_size: u64,
    /// Total compressed size of all data (8 bytes)
    pub total_compressed_size: u64,
    /// Argon2 salt for encryption (16 bytes) — all zeros if no encryption
    pub encryption_salt: [u8; 16],
    /// Reserved for future use (5 bytes)
    pub reserved: [u8; 5],
}

impl FileHeader {
    pub const SIZE: usize = 64;

    pub fn new(algorithm: Algorithm, level: u8, chunk_size: u64) -> Self {
        Self {
            magic: *MAGIC_BYTES,
            version: FORMAT_VERSION,
            algorithm,
            level,
            encryption: EncryptionMode::None,
            flags: ArchiveFlags::NONE,
            chunk_size,
            file_count: 0,
            chunk_count: 0,
            total_uncompressed_size: 0,
            total_compressed_size: 0,
            encryption_salt: [0u8; 16],
            reserved: [0u8; 5],
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == *MAGIC_BYTES && self.version == FORMAT_VERSION
    }
}

/// Chunk header — precedes each compressed chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkHeader {
    /// Index of this chunk (4 bytes)
    pub chunk_index: u32,
    /// Algorithm used for this specific chunk (1 byte)
    pub algorithm: Algorithm,
    /// Uncompressed size of this chunk (8 bytes)
    pub uncompressed_size: u64,
    /// Compressed size of this chunk (8 bytes)
    pub compressed_size: u64,
    /// CRC32 of uncompressed data (4 bytes)
    pub crc32_uncompressed: u32,
    /// CRC32 of compressed data (4 bytes)
    pub crc32_compressed: u32,
    /// AES-GCM nonce if encrypted (12 bytes) — zeros if not encrypted
    pub nonce: [u8; 12],
}

impl ChunkHeader {
    pub const SIZE: usize = 41;
}

/// File entry in the index — describes one file in the archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Relative path within the archive (UTF-8)
    pub path: String,
    /// Original uncompressed file size
    pub original_size: u64,
    /// Unix-style permissions (e.g., 0o755)
    pub permissions: u32,
    /// Modification time (Unix timestamp)
    pub modified_time: i64,
    /// Creation time (Unix timestamp)
    pub created_time: i64,
    /// Is this entry a directory?
    pub is_directory: bool,
    /// Index of first chunk containing this file's data
    pub first_chunk_index: u32,
    /// Number of chunks this file spans
    pub chunk_count: u32,
    /// XXH3 hash of the original file (for verification)
    pub xxh3_hash: u64,
}

/// Archive footer — last 32 bytes of the file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Footer {
    /// Offset to the start of the file index (8 bytes)
    pub index_offset: u64,
    /// Size of the file index in bytes (8 bytes)
    pub index_size: u64,
    /// XXH3 hash of the entire archive (excluding footer) (8 bytes)
    pub archive_hash: u64,
    /// Magic bytes repeated for validation (4 bytes)
    pub magic: [u8; 4],
    /// Reserved (4 bytes)
    pub reserved: [u8; 4],
}

impl Footer {
    pub const SIZE: usize = 32;

    pub fn is_valid(&self) -> bool {
        self.magic == *MAGIC_BYTES
    }
}
