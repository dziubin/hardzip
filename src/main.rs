//! HardZIP — Harder, Better, Faster, Stronger compression
//!
//! Multi-algorithm archiver with:
//! - Auto-algorithm selection (Zstd, LZ4, Brotli, LZMA2)
//! - Parallel chunk processing
//! - AES-256-GCM encryption with Argon2id

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//! - Custom .hza binary format
//! - CLI and GUI interfaces

mod algorithms;
mod cli;
mod core;
#[cfg(feature = "gui")]
mod gui;
mod utils;

use clap::Parser;

fn main() {
    // Initialize logger
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn")
    ).init();

    // If no arguments provided and GUI is available, launch GUI
    let args: Vec<String> = std::env::args().collect();

    if args.len() <= 1 {
        #[cfg(feature = "gui")]
        {
            gui::app::run_gui();
            return;
        }
        #[cfg(not(feature = "gui"))]
        {
            let _ = cli::Cli::parse_from(["hardzip", "--help"]);
            return;
        }
    }

    // Check for --dropzone flag (launches Drop Zone mini-window)
    #[cfg(feature = "gui")]
    if args.iter().any(|a| a == "--dropzone") {
        gui::app::run_dropzone_gui();
        return;
    }

    // If argument is a file path (not a subcommand), open GUI showing archive contents
    #[cfg(feature = "gui")]
    if args.len() == 2 {
        let path = std::path::Path::new(&args[1]);
        if path.exists() && (crate::utils::fs::is_hza_file(path) || crate::core::foreign::detect_format(path).is_some()) {
            gui::app::run_gui_with_file(&args[1]);
            return;
        }
    }

    // Parse CLI arguments
    let cli = cli::Cli::parse();

    // Run command
    if let Err(e) = cli::run_cli(cli) {
        eprintln!();
        eprintln!("  ✗ Error: {}", e);

        // Print cause chain
        for cause in e.chain().skip(1) {
            eprintln!("    Caused by: {}", cause);
        }

        eprintln!();
        std::process::exit(1);
    }
}
