# HardZIP

**Harder, Better, Faster, Stronger** — A modern multi-algorithm archiver.

![License](https://img.shields.io/badge/license-Proprietary-blue)
![Platform](https://img.shields.io/badge/platform-Windows-lightgrey)
![Language](https://img.shields.io/badge/language-Rust-orange)

## Features

- **Multi-algorithm compression**: Zstandard, LZ4, Brotli, LZMA2
- **Auto-algorithm selection**: tries LZMA2 + Zstd per chunk, picks smaller result
- **Parallel chunk processing** via Rayon
- **Military-grade encryption**: AES-256-GCM + Argon2id key derivation
- **Custom .hza binary format** with integrity verification (CRC32 + XXH3)
- **Browse archives**: view contents of .hza, .zip, .7z, .tar.gz without extracting
- **Foreign format support**: extract ZIP, 7z, tar, tar.gz, tar.bz2, tar.xz, gzip, bzip2, xz
- **File manager GUI**: 7-Zip style interface with Details/List/Icons views
- **CLI interface**: full command-line support for scripting and automation
- **Drop Zone**: always-on-top mini-window for drag & drop compression
- **Crypto module**: one-click file encryption (AES-256)
- **Watcher**: folder monitor with auto-compress on changes
- **Bilingual**: English and Polish interface
- **Windows integration**: context menu, file associations

## Installation

### From source

```bash
# Install Rust: https://rustup.rs/
cargo build --release
```

The binary will be at `target/release/hardzip.exe`.

### Installer

Download the installer from [Releases](../../releases) or build it using [Inno Setup](https://jrsoftware.org/isdl.php) with `installer/hardzip_setup.iss`.

## Usage

### GUI

Double-click `hardzip.exe` — opens the file manager.

### CLI

```bash
# Compress
hardzip compress myfile.txt -o archive.hza
hardzip compress folder/ -o backup.hza -p "password" -l 19

# Extract (supports .hza, .zip, .7z, .tar.gz, .bz2, .xz)
hardzip extract archive.hza -o output_folder
hardzip extract data.zip

# Archive info
hardzip info archive.hza

# Benchmark algorithms on a file
hardzip benchmark largefile.bin
```

### Options

| Flag | Description |
|------|-------------|
| `-o, --output` | Output path |
| `-a, --algorithm` | `auto`, `zstd`, `lz4`, `brotli`, `lzma`, `none` |
| `-l, --level` | Compression level (1-22) |
| `-p, --password` | Encryption password |
| `--chunk-size` | Chunk size in MB (1-256) |
| `--encrypt-names` | Encrypt file names inside archive |

## Architecture

```
src/
├── main.rs          # Entry point (GUI/CLI detection)
├── cli.rs           # CLI commands (clap)
├── algorithms/      # Compression wrappers (zstd, lz4, brotli, lzma)
├── core/
│   ├── archive.rs   # .hza format pack/unpack
│   ├── chunk.rs     # Parallel chunk processing
│   ├── compressor.rs # Auto algorithm selection
│   ├── crypto.rs    # AES-256-GCM + Argon2id
│   ├── foreign.rs   # ZIP/7z/tar extraction + listing
│   └── format.rs    # Binary format specification
├── gui/
│   ├── app.rs       # Main GUI (file manager, toolbar, dialogs)
│   ├── panels.rs    # Sub-panels (settings, crypto, watcher, etc.)
│   ├── theme.rs     # Apple-style light/dark themes
│   └── i18n.rs      # Translations (EN/PL)
└── utils/
    ├── fs.rs        # File system helpers
    └── progress.rs  # Progress bars and formatting
```

## Author

**Lukasz Dziubinski**
- Website: [www.ydi.pl](https://www.ydi.pl)
- Email: lukasz@ydi.pl

## License

Proprietary. All rights reserved.
