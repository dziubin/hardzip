//! CLI interface using clap — compress, decompress, info, benchmark commands

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::time::Instant;

use crate::core::archive::{create_archive, extract_archive, read_archive_info, ArchiveOptions};
use crate::core::format::{Algorithm, MAX_CHUNK_SIZE, MIN_CHUNK_SIZE};
use crate::utils::fs::{generate_archive_name, generate_extract_dir, is_hza_file, validate_inputs};
use crate::utils::progress::{
    create_progress_bar, format_bytes, format_duration, format_ratio, format_speed,
};

/// HardZIP — Harder, Better, Faster, Stronger compression
#[derive(Parser, Debug)]
#[command(
    name = "hardzip",
    version = "1.0.0",
    author = "HardZIP Team",
    about = "HardZIP — Harder, Better, Faster, Stronger compression\nMulti-algorithm archiver with AES-256 encryption",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Compress files/folders into a .hza archive
    #[command(alias = "c")]
    Compress {
        /// Input files or directories to compress
        #[arg(required = true)]
        input: Vec<PathBuf>,

        /// Output archive path (default: <input>.hza)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Compression algorithm
        #[arg(short, long, default_value = "auto")]
        algorithm: AlgorithmArg,

        /// Compression level (1-22 depending on algorithm)
        #[arg(short, long, default_value = "19")]
        level: u32,

        /// Chunk size in MB (1-256)
        #[arg(long, default_value = "64")]
        chunk_size: u32,

        /// Encrypt with password
        #[arg(short, long)]
        password: Option<String>,

        /// Also encrypt file names
        #[arg(long)]
        encrypt_names: bool,
    },

    /// Extract a .hza archive
    #[command(alias = "x")]
    Extract {
        /// Path to .hza archive
        #[arg(required = true)]
        archive: PathBuf,

        /// Output directory (default: archive name without extension)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Decryption password
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Show archive information and file listing
    #[command(alias = "i")]
    Info {
        /// Path to .hza archive
        #[arg(required = true)]
        archive: PathBuf,

        /// Decryption password (needed if filenames are encrypted)
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Benchmark all algorithms on a file
    #[command(alias = "b")]
    Benchmark {
        /// File to benchmark (uses first 16MB sample)
        #[arg(required = true)]
        file: PathBuf,

        /// Sample size in MB for benchmarking
        #[arg(long, default_value = "4")]
        sample_mb: u32,
    },

    /// Launch the GUI (if compiled with gui feature)
    Gui,
}

/// Algorithm argument for CLI
#[derive(Debug, Clone, ValueEnum)]
pub enum AlgorithmArg {
    Auto,
    Zstd,
    Lz4,
    Brotli,
    Lzma,
    None,
}

impl From<AlgorithmArg> for Algorithm {
    fn from(arg: AlgorithmArg) -> Self {
        match arg {
            AlgorithmArg::Auto => Algorithm::Auto,
            AlgorithmArg::Zstd => Algorithm::Zstd,
            AlgorithmArg::Lz4 => Algorithm::Lz4,
            AlgorithmArg::Brotli => Algorithm::Brotli,
            AlgorithmArg::Lzma => Algorithm::Lzma,
            AlgorithmArg::None => Algorithm::None,
        }
    }
}

/// Execute the CLI command
pub fn run_cli(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Compress {
            input,
            output,
            algorithm,
            level,
            chunk_size,
            password,
            encrypt_names,
        } => cmd_compress(input, output, algorithm, level, chunk_size, password, encrypt_names),
        Commands::Extract {
            archive,
            output,
            password,
        } => cmd_extract(archive, output, password),
        Commands::Info { archive, password } => cmd_info(archive, password),
        Commands::Benchmark { file, sample_mb } => cmd_benchmark(file, sample_mb),
        Commands::Gui => {
            #[cfg(feature = "gui")]
            {
                crate::gui::app::run_gui();
                Ok(())
            }
            #[cfg(not(feature = "gui"))]
            {
                anyhow::bail!("GUI not available — recompile with: cargo build --features gui");
            }
        }
    }
}

// ─── Command implementations ────────────────────────────────────────────────

fn cmd_compress(
    input: Vec<PathBuf>,
    output: Option<PathBuf>,
    algorithm: AlgorithmArg,
    level: u32,
    chunk_size_mb: u32,
    password: Option<String>,
    encrypt_names: bool,
) -> Result<()> {
    validate_inputs(&input)?;

    let output_path = output.unwrap_or_else(|| generate_archive_name(&input[0]));
    let chunk_size = (chunk_size_mb as u64 * 1024 * 1024).clamp(MIN_CHUNK_SIZE, MAX_CHUNK_SIZE);

    let options = ArchiveOptions {
        algorithm: Algorithm::from(algorithm),
        level,
        chunk_size,
        password,
        encrypt_filenames: encrypt_names,
    };

    println!("╔══════════════════════════════════════════════════╗");
    println!("║           HardZIP — Compressing                 ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!("  Algorithm:  {}", options.algorithm);
    println!("  Level:      {}", options.level);
    println!("  Chunk size: {}", format_bytes(chunk_size));
    println!("  Encrypted:  {}", if options.password.is_some() { "Yes (AES-256-GCM)" } else { "No" });
    println!("  Output:     {}", output_path.display());
    println!();

    let start = Instant::now();

    // Create progress bar
    let pb = create_progress_bar(0, "Compressing");
    pb.set_message("Scanning files...");

    let progress_cb = |current: u64, total: u64, file: &str| {
        pb.set_length(total);
        pb.set_position(current);
        pb.set_message(file.to_string());
    };

    create_archive(&output_path, &input, &options, Some(&progress_cb))
        .context("Compression failed")?;

    pb.finish_with_message("Done!");

    let elapsed = start.elapsed();
    let output_size = std::fs::metadata(&output_path)?.len();
    let input_size: u64 = input.iter().map(|p| {
        crate::utils::fs::get_total_size(p).unwrap_or(0)
    }).sum();

    println!();
    println!("  ✓ Archive created successfully!");
    println!("  ─────────────────────────────────");
    println!("  Original:   {}", format_bytes(input_size));
    println!("  Compressed: {}", format_bytes(output_size));
    println!("  Ratio:      {} saved", format_ratio(input_size, output_size));
    println!("  Time:       {}", format_duration(elapsed));
    println!("  Speed:      {}", format_speed(input_size, elapsed));
    println!();

    Ok(())
}

fn cmd_extract(
    archive: PathBuf,
    output: Option<PathBuf>,
    password: Option<String>,
) -> Result<()> {
    if !archive.exists() {
        anyhow::bail!("Archive not found: {}", archive.display());
    }

    let output_dir = output.unwrap_or_else(|| generate_extract_dir(&archive));

    println!("╔══════════════════════════════════════════════════╗");
    println!("║           HardZIP — Extracting                  ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!("  Archive: {}", archive.display());
    println!("  Output:  {}", output_dir.display());
    println!();

    let start = Instant::now();
    let pb = create_progress_bar(0, "Extracting");

    let progress_cb = |current: u64, total: u64, file: &str| {
        pb.set_length(total);
        pb.set_position(current);
        pb.set_message(file.to_string());
    };

    // Detect if it's a foreign format or our .hza
    if is_hza_file(&archive) {
        extract_archive(&archive, &output_dir, password.as_deref(), Some(&progress_cb))
            .context("Extraction failed")?;
    } else if crate::core::foreign::detect_format(&archive).is_some() {
        crate::core::foreign::extract_foreign(&archive, &output_dir, Some(&progress_cb))
            .context("Extraction failed")?;
    } else {
        anyhow::bail!("Unsupported archive format: {}", archive.display());
    }

    pb.finish_with_message("Done!");

    let elapsed = start.elapsed();
    println!();
    println!("  ✓ Archive extracted successfully!");
    println!("  Time: {}", format_duration(elapsed));
    println!();

    Ok(())
}

fn cmd_info(archive: PathBuf, password: Option<String>) -> Result<()> {
    if !archive.exists() {
        anyhow::bail!("Archive not found: {}", archive.display());
    }

    let info = read_archive_info(&archive, password.as_deref())
        .context("Failed to read archive info")?;

    println!("╔══════════════════════════════════════════════════╗");
    println!("║           HardZIP — Archive Info                ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!("  File:          {}", archive.display());
    println!("  Format:        HZA v{}", info.header.version);
    println!("  Algorithm:     {}", info.header.algorithm);
    println!("  Level:         {}", info.header.level);
    println!("  Chunk size:    {}", format_bytes(info.header.chunk_size));
    println!("  Chunks:        {}", info.header.chunk_count);
    println!("  Encrypted:     {}", if info.header.encryption != crate::core::format::EncryptionMode::None { "Yes" } else { "No" });
    println!("  Original size: {}", format_bytes(info.header.total_uncompressed_size));
    println!("  Archive size:  {}", format_bytes(info.header.total_compressed_size));
    println!("  Ratio:         {} saved", format_ratio(info.header.total_uncompressed_size, info.header.total_compressed_size));
    println!();
    println!("  Files ({}):", info.files.len());
    println!("  ─────────────────────────────────────────────────");

    for entry in &info.files {
        if entry.is_directory {
            println!("  📁 {}/", entry.path);
        } else {
            println!("  📄 {} ({})", entry.path, format_bytes(entry.original_size));
        }
    }
    println!();

    Ok(())
}

fn cmd_benchmark(file: PathBuf, sample_mb: u32) -> Result<()> {
    if !file.exists() {
        anyhow::bail!("File not found: {}", file.display());
    }

    let sample_size = (sample_mb as usize) * 1024 * 1024;
    let data = std::fs::read(&file)?;
    let sample = &data[..data.len().min(sample_size)];

    println!("╔══════════════════════════════════════════════════╗");
    println!("║         HardZIP — Benchmark Mode                ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!("  File:        {}", file.display());
    println!("  Sample size: {}", format_bytes(sample.len() as u64));
    println!();
    println!("  {:<12} {:<8} {:<12} {:<12} {:<10}",
        "Algorithm", "Level", "Compressed", "Ratio", "Time");
    println!("  ─────────────────────────────────────────────────────────");

    let algorithms: Vec<(Algorithm, &str, u32)> = vec![
        (Algorithm::Lz4, "LZ4", 1),
        (Algorithm::Zstd, "Zstd", 1),
        (Algorithm::Zstd, "Zstd", 3),
        (Algorithm::Zstd, "Zstd", 9),
        (Algorithm::Zstd, "Zstd", 19),
        (Algorithm::Brotli, "Brotli", 4),
        (Algorithm::Brotli, "Brotli", 6),
        (Algorithm::Brotli, "Brotli", 9),
        (Algorithm::Lzma, "LZMA2", 6),
    ];

    for (algo, name, level) in &algorithms {
        let compressor = crate::core::compressor::get_algorithm(*algo);

        let start = Instant::now();
        let compressed = compressor.compress(sample, *level)?;
        let compress_time = start.elapsed();

        let ratio = format_ratio(sample.len() as u64, compressed.len() as u64);
        let _speed = format_speed(sample.len() as u64, compress_time);

        println!("  {:<12} {:<8} {:<12} {:<12} {:<10}",
            name,
            level,
            format_bytes(compressed.len() as u64),
            ratio,
            format_duration(compress_time),
        );
    }

    println!();
    println!("  Auto-detected algorithm for this file: {}",
        crate::core::compressor::auto_select_algorithm(sample));
    println!();

    Ok(())
}
