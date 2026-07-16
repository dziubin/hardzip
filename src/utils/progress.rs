//! Progress reporting utilities for CLI and GUI

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Creates a progress bar for file compression/decompression
pub fn create_progress_bar(total_size: u64, operation: &str) -> ProgressBar {
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(&format!(
                "{{spinner:.green}} {} [{{bar:40.cyan/blue}}] {{bytes}}/{{total_bytes}} ({{eta}}) {{msg}}",
                operation
            ))
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("━━░"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Creates a spinner for indeterminate operations
pub fn create_spinner(message: &str) -> ProgressBar {
    let sp = ProgressBar::new_spinner();
    sp.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    sp.set_message(message.to_string());
    sp.enable_steady_tick(Duration::from_millis(80));
    sp
}

/// Formats a byte size into human-readable form
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

/// Formats a compression ratio as percentage
pub fn format_ratio(original: u64, compressed: u64) -> String {
    if original == 0 {
        return "N/A".to_string();
    }
    let ratio = (1.0 - (compressed as f64 / original as f64)) * 100.0;
    format!("{:.1}%", ratio)
}

/// Formats duration in human-readable form
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();

    if secs == 0 {
        format!("{}ms", millis)
    } else if secs < 60 {
        format!("{}.{:02}s", secs, millis / 10)
    } else {
        let mins = secs / 60;
        let remaining_secs = secs % 60;
        format!("{}m {}s", mins, remaining_secs)
    }
}

/// Formats speed in bytes/second
pub fn format_speed(bytes: u64, duration: Duration) -> String {
    let secs = duration.as_secs_f64();
    if secs == 0.0 {
        return "∞".to_string();
    }
    let speed = bytes as f64 / secs;
    format!("{}/s", format_bytes(speed as u64))
}
