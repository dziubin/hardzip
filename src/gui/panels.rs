//! UI panels for the HardZIP GUI — Apple-style, bilingual

use egui::{self, Color32, RichText, Ui};
use std::path::PathBuf;

use crate::core::format::Algorithm;
use crate::gui::i18n::{get_strings, Language};
use crate::gui::theme::{Theme, ThemeColors};
use crate::utils::progress::{format_bytes, format_ratio};

/// Current tab in the GUI
#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Compress,
    Extract,
    Info,
    Crypto,
    Watcher,
    DropZone,
    Settings,
    Help,
    About,
}

/// Application settings
#[derive(Debug, Clone)]
pub struct AppSettings {
    pub theme: Theme,
    pub language: Language,
    pub context_menu_status: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Light,
            language: Language::English,
            context_menu_status: None,
        }
    }
}

// ─── State structs ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CompressState {
    pub input_paths: Vec<PathBuf>,
    pub output_path: String,
    pub algorithm: Algorithm,
    pub level: u32,
    pub chunk_size_mb: u32,
    pub password: String,
    pub encrypt_names: bool,
    pub is_running: bool,
    pub progress: f32,
    pub status_message: String,
    pub result_message: Option<String>,
}

impl Default for CompressState {
    fn default() -> Self {
        Self {
            input_paths: Vec::new(), output_path: String::new(),
            algorithm: Algorithm::Auto, level: 19, chunk_size_mb: 64,
            password: String::new(), encrypt_names: false,
            is_running: false, progress: 0.0,
            status_message: String::new(), result_message: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtractState {
    pub archive_path: String,
    pub output_dir: String,
    pub password: String,
    pub is_running: bool,
    pub progress: f32,
    pub status_message: String,
    pub result_message: Option<String>,
}

impl Default for ExtractState {
    fn default() -> Self {
        Self {
            archive_path: String::new(), output_dir: String::new(),
            password: String::new(), is_running: false, progress: 0.0,
            status_message: String::new(), result_message: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CryptoState {
    pub files: Vec<PathBuf>,
    pub password: String,
    pub password_confirm: String,
    pub is_running: bool,
    pub result_message: Option<String>,
}

impl Default for CryptoState {
    fn default() -> Self {
        Self {
            files: Vec::new(), password: String::new(),
            password_confirm: String::new(), is_running: false,
            result_message: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WatcherState {
    pub watch_folder: String,
    pub output_folder: String,
    pub is_running: bool,
    pub auto_compress: bool,
    pub auto_backup: bool,
    pub events: Vec<String>,
}

impl Default for WatcherState {
    fn default() -> Self {
        Self {
            watch_folder: String::new(), output_folder: String::new(),
            is_running: false, auto_compress: true, auto_backup: false,
            events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DropZoneState {
    pub output_folder: String,
    pub dropped_file: Option<PathBuf>,
    pub last_action: Option<String>,
}

impl Default for DropZoneState {
    fn default() -> Self {
        Self {
            output_folder: String::new(), dropped_file: None, last_action: None,
        }
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn section_card(ui: &mut Ui, colors: &ThemeColors, add_contents: impl FnOnce(&mut Ui)) {
    egui::Frame::none()
        .fill(colors.surface)
        .rounding(egui::Rounding::same(8.0))
        .stroke(egui::Stroke::new(0.5_f32, colors.border))
        .inner_margin(egui::Margin::same(10.0))
        .outer_margin(egui::Margin::symmetric(0.0, 3.0))
        .show(ui, |ui| { add_contents(ui); });
}

fn section_label(ui: &mut Ui, text: &str, colors: &ThemeColors) {
    ui.label(RichText::new(text).color(colors.text_secondary).size(12.0));
    ui.add_space(4.0);
}

fn accent_button(ui: &mut Ui, text: &str, enabled: bool, colors: &ThemeColors) -> bool {
    let button = egui::Button::new(RichText::new(text).color(Color32::WHITE).size(14.0))
        .fill(if enabled { colors.accent } else { colors.button_bg })
        .rounding(egui::Rounding::same(8.0));
    ui.add_enabled(enabled, button).clicked()
}

fn success_button(ui: &mut Ui, text: &str, enabled: bool, colors: &ThemeColors) -> bool {
    let button = egui::Button::new(RichText::new(text).color(Color32::WHITE).size(14.0))
        .fill(if enabled { colors.success } else { colors.button_bg })
        .rounding(egui::Rounding::same(8.0));
    ui.add_enabled(enabled, button).clicked()
}

// ─── Compress Panel ─────────────────────────────────────────────────────────

pub fn render_compress_panel(ui: &mut Ui, state: &mut CompressState, settings: &AppSettings) {
    let s = get_strings(settings.language);
    let colors = ThemeColors::for_theme(settings.theme);

    ui.heading(RichText::new(s.compress_title).color(colors.text_primary));
    ui.add_space(12.0);

    section_card(ui, &colors, |ui| {
        section_label(ui, s.compress_input_label, &colors);
        if state.input_paths.is_empty() {
            ui.label(RichText::new(s.compress_no_files).color(colors.text_tertiary).italics().size(13.0));
        } else {
            for (i, path) in state.input_paths.iter().enumerate() {
                ui.label(RichText::new(format!("{}. {}", i + 1, path.display())).color(colors.text_primary).size(13.0));
            }
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(s.compress_add_files).clicked() {
                if let Some(paths) = rfd::FileDialog::new().pick_files() { state.input_paths.extend(paths); }
            }
            if ui.button(s.compress_add_folder).clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() { state.input_paths.push(path); }
            }
            if !state.input_paths.is_empty() && ui.button(s.compress_clear).clicked() { state.input_paths.clear(); }
        });
    });

    section_card(ui, &colors, |ui| {
        section_label(ui, s.compress_output_label, &colors);
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut state.output_path).desired_width(ui.available_width() - 100.0));
            if ui.button(s.compress_browse).clicked() {
                if let Some(p) = rfd::FileDialog::new().add_filter("HardZIP", &["hza"]).save_file() {
                    state.output_path = p.display().to_string();
                }
            }
        });
    });

    section_card(ui, &colors, |ui| {
        section_label(ui, s.compress_settings_label, &colors);
        ui.horizontal(|ui| {
            ui.label(RichText::new(s.compress_algorithm).color(colors.text_primary));
            egui::ComboBox::from_id_source("algo_select").selected_text(state.algorithm.display_name()).show_ui(ui, |ui| {
                ui.selectable_value(&mut state.algorithm, Algorithm::Auto, s.algo_auto);
                ui.selectable_value(&mut state.algorithm, Algorithm::Zstd, s.algo_zstd);
                ui.selectable_value(&mut state.algorithm, Algorithm::Lz4, s.algo_lz4);
                ui.selectable_value(&mut state.algorithm, Algorithm::Brotli, s.algo_brotli);
                ui.selectable_value(&mut state.algorithm, Algorithm::Lzma, s.algo_lzma);
                ui.selectable_value(&mut state.algorithm, Algorithm::None, s.algo_none);
            });
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new(s.compress_level).color(colors.text_primary));
            ui.add(egui::Slider::new(&mut state.level, 1..=22));
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new(s.compress_chunk_size).color(colors.text_primary));
            ui.add(egui::Slider::new(&mut state.chunk_size_mb, 1..=64).suffix(" MB"));
        });
    });

    section_card(ui, &colors, |ui| {
        section_label(ui, s.compress_encryption_label, &colors);
        ui.horizontal(|ui| {
            ui.label(RichText::new(s.compress_password).color(colors.text_primary));
            ui.add(egui::TextEdit::singleline(&mut state.password).password(true));
        });
        ui.checkbox(&mut state.encrypt_names, s.compress_encrypt_names);
    });

    ui.add_space(12.0);
    let can = !state.input_paths.is_empty() && !state.is_running;
    let txt = if state.is_running { s.compress_running } else { s.compress_button };
    if accent_button(ui, txt, can, &colors) { state.is_running = true; state.progress = 0.0; state.result_message = None; }

    if state.is_running { ui.add_space(8.0); ui.add(egui::ProgressBar::new(state.progress).text(&state.status_message)); }
    if let Some(ref msg) = state.result_message { ui.add_space(8.0); ui.label(RichText::new(msg).color(colors.success).size(13.0)); }
}

// ─── Extract Panel ──────────────────────────────────────────────────────────

pub fn render_extract_panel(ui: &mut Ui, state: &mut ExtractState, settings: &AppSettings) {
    let s = get_strings(settings.language);
    let colors = ThemeColors::for_theme(settings.theme);

    ui.heading(RichText::new(s.extract_title).color(colors.text_primary));
    ui.add_space(12.0);

    section_card(ui, &colors, |ui| {
        section_label(ui, s.extract_archive_label, &colors);
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut state.archive_path).desired_width(ui.available_width() - 100.0));
            if ui.button(s.compress_browse).clicked() {
                if let Some(p) = rfd::FileDialog::new()
                    .add_filter("Archives", &["hza", "zip", "7z", "tar", "gz", "tgz", "rar", "bz2", "xz"])
                    .add_filter("All files", &["*"])
                    .pick_file() {
                    state.archive_path = p.display().to_string();
                }
            }
        });
    });

    section_card(ui, &colors, |ui| {
        section_label(ui, s.extract_output_label, &colors);
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut state.output_dir).desired_width(ui.available_width() - 100.0));
            if ui.button(s.compress_browse).clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() { state.output_dir = p.display().to_string(); }
            }
        });
    });

    section_card(ui, &colors, |ui| {
        section_label(ui, s.extract_password_label, &colors);
        ui.horizontal(|ui| {
            ui.label(RichText::new(s.compress_password).color(colors.text_primary));
            ui.add(egui::TextEdit::singleline(&mut state.password).password(true));
        });
    });

    ui.add_space(12.0);
    let can = !state.archive_path.is_empty() && !state.is_running;
    let txt = if state.is_running { s.extract_running } else { s.extract_button };
    if success_button(ui, txt, can, &colors) { state.is_running = true; state.progress = 0.0; state.result_message = None; }

    if state.is_running { ui.add_space(8.0); ui.add(egui::ProgressBar::new(state.progress).text(&state.status_message)); }
    if let Some(ref msg) = state.result_message { ui.add_space(8.0); ui.label(RichText::new(msg).color(colors.success).size(13.0)); }
}

// ─── Info Panel ─────────────────────────────────────────────────────────────

pub fn render_info_panel(ui: &mut Ui, archive_path: &str, password: &str, settings: &AppSettings) {
    let s = get_strings(settings.language);
    let colors = ThemeColors::for_theme(settings.theme);

    ui.heading(RichText::new(s.info_title).color(colors.text_primary));
    ui.add_space(12.0);

    if archive_path.is_empty() {
        ui.label(RichText::new(s.info_select_archive).color(colors.text_tertiary).italics());
        return;
    }

    let path = std::path::Path::new(archive_path);
    let pw = if password.is_empty() { None } else { Some(password) };

    match crate::core::archive::read_archive_info(path, pw) {
        Ok(info) => {
            section_card(ui, &colors, |ui| {
                ui.label(RichText::new(format!("{}: HZA v{}", s.info_format, info.header.version)).color(colors.text_primary));
                ui.label(RichText::new(format!("{}: {}", s.info_algorithm, info.header.algorithm)).color(colors.text_primary));
                ui.label(RichText::new(format!("{}: {}", s.info_chunks, info.header.chunk_count)).color(colors.text_primary));
                ui.label(RichText::new(format!("{}: {}", s.info_original, format_bytes(info.header.total_uncompressed_size))).color(colors.text_primary));
                ui.label(RichText::new(format!("{}: {}", s.info_compressed, format_bytes(info.header.total_compressed_size))).color(colors.text_primary));
                ui.label(RichText::new(format!("{}: {}", s.info_saved, format_ratio(info.header.total_uncompressed_size, info.header.total_compressed_size))).color(colors.success));
            });
            ui.add_space(8.0);
            ui.label(RichText::new(format!("{} ({}):", s.info_files, info.files.len())).color(colors.text_secondary));
            section_card(ui, &colors, |ui| {
                egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                    for entry in &info.files {
                        if entry.is_directory {
                            ui.label(RichText::new(format!("  {}/", entry.path)).color(colors.accent).size(13.0));
                        } else {
                            ui.label(RichText::new(format!("  {} ({})", entry.path, format_bytes(entry.original_size))).color(colors.text_primary).size(13.0));
                        }
                    }
                });
            });
        }
        Err(e) => { ui.label(RichText::new(format!("{}: {}", s.common_error, e)).color(colors.error)); }
    }
}

// ─── Crypto Panel ───────────────────────────────────────────────────────────

pub fn render_crypto_panel(ui: &mut Ui, state: &mut CryptoState, settings: &AppSettings) {
    let s = get_strings(settings.language);
    let colors = ThemeColors::for_theme(settings.theme);

    ui.heading(RichText::new(s.crypto_title).color(colors.text_primary));
    ui.add_space(4.0);
    ui.label(RichText::new(s.crypto_one_click).color(colors.text_secondary).size(13.0));
    ui.add_space(12.0);

    // File selection
    section_card(ui, &colors, |ui| {
        section_label(ui, s.crypto_select_files, &colors);

        if state.files.is_empty() {
            ui.label(RichText::new(s.compress_no_files).color(colors.text_tertiary).italics().size(13.0));
        } else {
            for (i, path) in state.files.iter().enumerate() {
                ui.label(RichText::new(format!("{}. {}", i + 1, path.display())).color(colors.text_primary).size(13.0));
            }
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(s.compress_add_files).clicked() {
                if let Some(paths) = rfd::FileDialog::new().pick_files() { state.files.extend(paths); }
            }
            if !state.files.is_empty() && ui.button(s.compress_clear).clicked() { state.files.clear(); }
        });
    });

    // Password
    section_card(ui, &colors, |ui| {
        section_label(ui, s.crypto_desc, &colors);
        ui.horizontal(|ui| {
            ui.label(RichText::new(s.crypto_password).color(colors.text_primary));
            ui.add(egui::TextEdit::singleline(&mut state.password).password(true).desired_width(200.0));
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new(s.crypto_password_confirm).color(colors.text_primary));
            ui.add(egui::TextEdit::singleline(&mut state.password_confirm).password(true).desired_width(200.0));
        });

        let passwords_match = !state.password.is_empty() && state.password == state.password_confirm;
        if !state.password.is_empty() && !state.password_confirm.is_empty() && !passwords_match {
            ui.label(RichText::new("Passwords do not match").color(colors.error).size(12.0));
        }
    });

    ui.add_space(12.0);

    // Encrypt / Decrypt buttons
    let can_encrypt = !state.files.is_empty() && !state.password.is_empty()
        && state.password == state.password_confirm && !state.is_running;

    ui.horizontal(|ui| {
        let enc_text = if state.is_running { s.crypto_encrypting } else { s.crypto_encrypt_button };
        if accent_button(ui, enc_text, can_encrypt, &colors) {
            // Encrypt each file using our crypto engine
            state.is_running = true;
            let mut results = Vec::new();
            for file_path in &state.files {
                match encrypt_file_standalone(file_path, &state.password) {
                    Ok(out) => results.push(format!("OK: {}", out.display())),
                    Err(e) => results.push(format!("ERR: {}", e)),
                }
            }
            state.is_running = false;
            state.result_message = Some(results.join("\n"));
        }

        let can_decrypt = !state.files.is_empty() && !state.password.is_empty() && !state.is_running;
        let dec_text = if state.is_running { s.crypto_decrypting } else { s.crypto_decrypt_button };
        if success_button(ui, dec_text, can_decrypt, &colors) {
            state.is_running = true;
            let mut results = Vec::new();
            for file_path in &state.files {
                match decrypt_file_standalone(file_path, &state.password) {
                    Ok(out) => results.push(format!("OK: {}", out.display())),
                    Err(e) => results.push(format!("ERR: {}", e)),
                }
            }
            state.is_running = false;
            state.result_message = Some(results.join("\n"));
        }
    });

    if let Some(ref msg) = state.result_message {
        ui.add_space(8.0);
        section_card(ui, &colors, |ui| {
            ui.label(RichText::new(msg).color(colors.text_primary).size(12.0).family(egui::FontFamily::Monospace));
        });
    }
}

/// Encrypts a single file to .hza.enc
fn encrypt_file_standalone(path: &PathBuf, password: &str) -> Result<PathBuf, String> {
    let data = std::fs::read(path).map_err(|e| format!("Read error: {}", e))?;
    let salt = crate::core::crypto::generate_salt();
    let engine = crate::core::crypto::CryptoEngine::new(password, &salt)
        .map_err(|e| format!("Key error: {}", e))?;
    let (encrypted, nonce) = engine.encrypt(&data).map_err(|e| format!("Encrypt error: {}", e))?;

    // Write: salt (16) + nonce (12) + ciphertext
    let out_path = path.with_extension(format!("{}.enc",
        path.extension().unwrap_or_default().to_string_lossy()));
    let mut output = Vec::with_capacity(16 + 12 + encrypted.len());
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&encrypted);
    std::fs::write(&out_path, &output).map_err(|e| format!("Write error: {}", e))?;
    Ok(out_path)
}

/// Decrypts a .enc file back to original
fn decrypt_file_standalone(path: &PathBuf, password: &str) -> Result<PathBuf, String> {
    let data = std::fs::read(path).map_err(|e| format!("Read error: {}", e))?;
    if data.len() < 28 { return Err("File too small to be encrypted".to_string()); }

    let mut salt = [0u8; 16];
    salt.copy_from_slice(&data[..16]);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&data[16..28]);
    let ciphertext = &data[28..];

    let engine = crate::core::crypto::CryptoEngine::new(password, &salt)
        .map_err(|e| format!("Key error: {}", e))?;
    let decrypted = engine.decrypt(ciphertext, &nonce)
        .map_err(|_| "Decryption failed — wrong password".to_string())?;

    // Remove .enc extension
    let out_path = if path.to_string_lossy().ends_with(".enc") {
        PathBuf::from(path.to_string_lossy().trim_end_matches(".enc"))
    } else {
        path.with_extension("dec")
    };
    std::fs::write(&out_path, &decrypted).map_err(|e| format!("Write error: {}", e))?;
    Ok(out_path)
}

// ─── Watcher Panel ──────────────────────────────────────────────────────────

pub fn render_watcher_panel(ui: &mut Ui, state: &mut WatcherState, settings: &AppSettings) {
    let s = get_strings(settings.language);
    let colors = ThemeColors::for_theme(settings.theme);

    ui.heading(RichText::new(s.watcher_title).color(colors.text_primary));
    ui.add_space(4.0);
    ui.label(RichText::new(s.watcher_desc).color(colors.text_secondary).size(13.0));
    ui.add_space(12.0);

    // Folder selection
    section_card(ui, &colors, |ui| {
        section_label(ui, s.watcher_watch_folder, &colors);
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut state.watch_folder).desired_width(ui.available_width() - 100.0));
            if ui.button(s.compress_browse).clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() { state.watch_folder = p.display().to_string(); }
            }
        });

        ui.add_space(8.0);
        section_label(ui, s.watcher_output_folder, &colors);
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut state.output_folder).desired_width(ui.available_width() - 100.0));
            if ui.button(s.compress_browse).clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_folder() { state.output_folder = p.display().to_string(); }
            }
        });
    });

    // Options
    section_card(ui, &colors, |ui| {
        ui.checkbox(&mut state.auto_compress, s.watcher_auto_compress);
        ui.checkbox(&mut state.auto_backup, s.watcher_auto_backup);
    });

    ui.add_space(12.0);

    // Start / Stop
    ui.horizontal(|ui| {
        if !state.is_running {
            let can_start = !state.watch_folder.is_empty() && !state.output_folder.is_empty();
            if accent_button(ui, s.watcher_start, can_start, &colors) {
                state.is_running = true;
                state.events.push(format!("{}: {}", s.watcher_running, state.watch_folder));
            }
        } else {
            if accent_button(ui, s.watcher_stop, true, &colors) {
                state.is_running = false;
                state.events.push(s.watcher_stopped.to_string());
            }
            ui.add_space(8.0);
            ui.label(RichText::new(s.watcher_running).color(colors.success).size(13.0));
        }
    });

    // Events log
    if !state.events.is_empty() {
        ui.add_space(12.0);
        section_card(ui, &colors, |ui| {
            section_label(ui, s.watcher_events, &colors);
            egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                for event in state.events.iter().rev().take(20) {
                    ui.label(RichText::new(event).color(colors.text_secondary).size(12.0).family(egui::FontFamily::Monospace));
                }
            });
        });
    }
}

// ─── Drop Zone Panel ────────────────────────────────────────────────────────

pub fn render_dropzone_panel(ui: &mut Ui, state: &mut DropZoneState, settings: &AppSettings) {
    let s = get_strings(settings.language);
    let colors = ThemeColors::for_theme(settings.theme);

    ui.add_space(8.0);

    // Output folder (compact)
    ui.horizontal(|ui| {
        ui.label(RichText::new(s.dropzone_output_folder).color(colors.text_secondary).size(12.0));
        ui.add(egui::TextEdit::singleline(&mut state.output_folder)
            .desired_width(ui.available_width() - 80.0).hint_text("(same as source)"));
        if ui.button("...").clicked() {
            if let Some(p) = rfd::FileDialog::new().pick_folder() { state.output_folder = p.display().to_string(); }
        }
    });

    ui.add_space(12.0);

    // Drop zone — fill most of the space
    let zone_size = ui.available_size().min(egui::vec2(360.0, 200.0));
    let zone_height = zone_size.y.max(100.0);

    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), zone_height),
        egui::Sense::hover(),
    );

    let painter = ui.painter();
    let hovered = response.hovered();

    // Draw rounded drop area
    painter.rect(
        rect,
        egui::Rounding::same(12.0),
        if hovered { colors.surface } else { colors.surface_alt },
        egui::Stroke::new(if hovered { 2.5_f32 } else { 1.5_f32 },
            if hovered { colors.accent } else { colors.border }),
    );

    // Icon + text
    painter.text(
        egui::pos2(rect.center().x, rect.center().y - 12.0),
        egui::Align2::CENTER_CENTER,
        "\u{1F4E5}",
        egui::FontId::new(28.0, egui::FontFamily::Proportional),
        if hovered { colors.accent } else { colors.text_tertiary },
    );
    painter.text(
        egui::pos2(rect.center().x, rect.center().y + 18.0),
        egui::Align2::CENTER_CENTER,
        s.dropzone_drag_here,
        egui::FontId::new(14.0, egui::FontFamily::Proportional),
        if hovered { colors.accent } else { colors.text_tertiary },
    );

    // Handle dropped file
    if let Some(ref file_path) = state.dropped_file.clone() {
        let output_dir = if state.output_folder.is_empty() {
            file_path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf()
        } else { PathBuf::from(&state.output_folder) };

        let archive_name = format!("{}.hza", file_path.file_name().unwrap_or_default().to_string_lossy());
        let archive_path = output_dir.join(&archive_name);

        let options = crate::core::archive::ArchiveOptions::default();
        match crate::core::archive::create_archive(&archive_path, &[file_path.clone()], &options, None) {
            Ok(()) => {
                let size = std::fs::metadata(&archive_path).map(|m| format_bytes(m.len())).unwrap_or_else(|_| "?".into());
                state.last_action = Some(format!("{} \u{2192} {} ({})",
                    file_path.file_name().unwrap_or_default().to_string_lossy(), archive_name, size));
            }
            Err(e) => { state.last_action = Some(format!("Error: {}", e)); }
        }
        state.dropped_file = None;
    }

    // Last action result
    if let Some(ref action) = state.last_action {
        ui.add_space(8.0);
        ui.label(RichText::new(action).color(colors.success).size(12.0));
    }
}

// ─── Settings Panel ─────────────────────────────────────────────────────────

pub fn render_settings_panel(ui: &mut Ui, settings: &mut AppSettings) {
    let s = get_strings(settings.language);
    let colors = ThemeColors::for_theme(settings.theme);

    ui.heading(RichText::new(s.settings_title).color(colors.text_primary));
    ui.add_space(12.0);

    section_card(ui, &colors, |ui| {
        section_label(ui, s.settings_appearance, &colors);

        ui.horizontal(|ui| {
            ui.label(RichText::new(s.settings_theme).color(colors.text_primary));
            ui.add_space(16.0);
            if ui.selectable_label(settings.theme == Theme::Light,
                RichText::new(s.settings_theme_light).color(if settings.theme == Theme::Light { colors.accent } else { colors.text_secondary })).clicked() {
                settings.theme = Theme::Light;
            }
            if ui.selectable_label(settings.theme == Theme::Dark,
                RichText::new(s.settings_theme_dark).color(if settings.theme == Theme::Dark { colors.accent } else { colors.text_secondary })).clicked() {
                settings.theme = Theme::Dark;
            }
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(s.settings_language).color(colors.text_primary));
            ui.add_space(16.0);
            if ui.selectable_label(settings.language == Language::English,
                RichText::new("English").color(if settings.language == Language::English { colors.accent } else { colors.text_secondary })).clicked() {
                settings.language = Language::English;
            }
            if ui.selectable_label(settings.language == Language::Polish,
                RichText::new("Polski").color(if settings.language == Language::Polish { colors.accent } else { colors.text_secondary })).clicked() {
                settings.language = Language::Polish;
            }
        });
    });

    ui.add_space(4.0);

    section_card(ui, &colors, |ui| {
        section_label(ui, s.settings_integration, &colors);
        ui.label(RichText::new(s.settings_context_menu).color(colors.text_primary).strong());
        ui.add_space(4.0);
        ui.label(RichText::new(s.settings_context_menu_desc).color(colors.text_secondary).size(13.0));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(s.settings_add_context_menu).clicked() {
                match add_all_context_menu_entries() {
                    Ok(()) => settings.context_menu_status = Some(s.settings_context_menu_added.to_string()),
                    Err(e) => settings.context_menu_status = Some(format!("{}: {}", s.common_error, e)),
                }
            }
            if ui.button(s.settings_remove_context_menu).clicked() {
                match remove_all_context_menu_entries() {
                    Ok(()) => settings.context_menu_status = Some(s.settings_context_menu_removed.to_string()),
                    Err(e) => settings.context_menu_status = Some(format!("{}: {}", s.common_error, e)),
                }
            }
        });
        ui.add_space(8.0);
        ui.label(RichText::new("Menu entries:").color(colors.text_secondary).size(12.0));
        ui.label(RichText::new("  \u{2022} Compress with HardZIP").color(colors.text_primary).size(12.0));
        ui.label(RichText::new("  \u{2022} Extract with HardZIP").color(colors.text_primary).size(12.0));
        ui.label(RichText::new("  \u{2022} Encrypt with HardZIP (AES-256)").color(colors.text_primary).size(12.0));
        ui.label(RichText::new("  \u{2022} Archive info").color(colors.text_primary).size(12.0));
        ui.add_space(4.0);
        ui.label(RichText::new(s.settings_requires_admin).color(colors.text_tertiary).size(11.0).italics());
        if let Some(ref status) = settings.context_menu_status {
            ui.add_space(4.0);
            ui.label(RichText::new(status).color(colors.accent).size(13.0));
        }
    });

    ui.add_space(4.0);

    // File associations
    section_card(ui, &colors, |ui| {
        section_label(ui, "File Associations", &colors);
        ui.label(RichText::new("Associate HardZIP with archive file types:").color(colors.text_secondary).size(13.0));
        ui.add_space(8.0);
        if ui.button("Associate archive files with HardZIP").clicked() {
            let _ = associate_hza_files();
            settings.context_menu_status = Some("File association updated!".to_string());
        }
        ui.add_space(4.0);
        ui.label(RichText::new("Associates .hza .zip .7z .tar .gz .bz2 .xz with HardZIP.").color(colors.text_tertiary).size(11.0).italics());
    });

    ui.add_space(4.0);

    // Compression defaults
    section_card(ui, &colors, |ui| {
        section_label(ui, "Default Compression", &colors);
        ui.label(RichText::new("Default level: 19 (maximum quality)").color(colors.text_primary).size(13.0));
        ui.label(RichText::new("Default chunk: 64 MB").color(colors.text_primary).size(13.0));
        ui.label(RichText::new("Algorithm: Auto (tries LZMA2 + Zstd, picks smaller)").color(colors.text_primary).size(13.0));
    });
}

fn associate_hza_files() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = exe.display().to_string();
    let cmd = format!("\"{}\" \"%1\"", exe_str);
    let icon = format!("{},0", exe_str);

    // Register HardZIP as a program
    let _ = std::process::Command::new("reg").args(["add", r"HKCU\Software\Classes\HardZIP.Archive", "/ve", "/d", "HardZIP Archive", "/f"]).output();
    let _ = std::process::Command::new("reg").args(["add", r"HKCU\Software\Classes\HardZIP.Archive\DefaultIcon", "/ve", "/d", &icon, "/f"]).output();
    let _ = std::process::Command::new("reg").args(["add", r"HKCU\Software\Classes\HardZIP.Archive\shell\open\command", "/ve", "/d", &cmd, "/f"]).output();

    // Associate all supported extensions
    let extensions = [".hza", ".zip", ".7z", ".tar", ".gz", ".tgz", ".bz2", ".xz", ".tbz2", ".txz"];
    for ext in &extensions {
        let _ = std::process::Command::new("reg").args(["add", &format!(r"HKCU\Software\Classes\{}", ext), "/ve", "/d", "HardZIP.Archive", "/f"]).output();
    }

    Ok(())
}

fn add_all_context_menu_entries() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = exe.display().to_string();

    // Compress with HardZIP
    reg_add(r"HKCU\Software\Classes\*\shell\HardZIP_Compress", "Compress with HardZIP", &exe_str,
        &format!("\"{}\" compress \"%1\" -o \"%1.hza\"", exe_str))?;

    // Extract with HardZIP (for .hza files)
    reg_add(r"HKCU\Software\Classes\.hza\shell\HardZIP_Extract", "Extract with HardZIP", &exe_str,
        &format!("\"{}\" extract \"%1\"", exe_str))?;

    // Encrypt with HardZIP
    reg_add(r"HKCU\Software\Classes\*\shell\HardZIP_Encrypt", "Encrypt with HardZIP (AES-256)", &exe_str,
        &format!("\"{}\" compress \"%1\" -o \"%1.hza\" -p \"\"", exe_str))?;

    // Archive info
    reg_add(r"HKCU\Software\Classes\.hza\shell\HardZIP_Info", "HardZIP \u{2014} Archive Info", &exe_str,
        &format!("\"{}\" info \"%1\"", exe_str))?;

    Ok(())
}

fn reg_add(key: &str, label: &str, icon: &str, command: &str) -> Result<(), String> {
    let _ = std::process::Command::new("reg").args(["add", key, "/ve", "/d", label, "/f"]).output();
    let _ = std::process::Command::new("reg").args(["add", key, "/v", "Icon", "/d", icon, "/f"]).output();
    let cmd_key = format!(r"{}\command", key);
    let out = std::process::Command::new("reg").args(["add", &cmd_key, "/ve", "/d", command, "/f"]).output().map_err(|e| e.to_string())?;
    if out.status.success() { Ok(()) } else { Err(format!("Failed: {}", key)) }
}

fn remove_all_context_menu_entries() -> Result<(), String> {
    let keys = [
        r"HKCU\Software\Classes\*\shell\HardZIP_Compress",
        r"HKCU\Software\Classes\.hza\shell\HardZIP_Extract",
        r"HKCU\Software\Classes\*\shell\HardZIP_Encrypt",
        r"HKCU\Software\Classes\.hza\shell\HardZIP_Info",
    ];
    for key in &keys {
        let _ = std::process::Command::new("reg").args(["delete", key, "/f"]).output();
    }
    Ok(())
}

// ─── Help Panel ─────────────────────────────────────────────────────────────

pub fn render_help_panel(ui: &mut Ui, settings: &AppSettings) {
    let s = get_strings(settings.language);
    let colors = ThemeColors::for_theme(settings.theme);

    ui.heading(RichText::new(s.help_title).color(colors.text_primary));
    ui.add_space(4.0);
    ui.label(RichText::new(s.help_intro).color(colors.text_secondary).size(14.0));
    ui.add_space(12.0);

    section_card(ui, &colors, |ui| {
        ui.label(RichText::new(s.help_cli_title).color(colors.text_primary).strong().size(15.0));
        ui.add_space(8.0);
        for cmd in [s.help_cli_compress, s.help_cli_extract, s.help_cli_info, s.help_cli_benchmark] {
            ui.label(RichText::new(cmd).color(colors.accent).family(egui::FontFamily::Monospace).size(13.0));
            ui.add_space(2.0);
        }
    });

    ui.add_space(4.0);
    section_card(ui, &colors, |ui| {
        ui.label(RichText::new(s.help_options_title).color(colors.text_primary).strong().size(15.0));
        ui.add_space(8.0);
        for (flag, desc) in [("-o, --output","Output path"),("-a, --algorithm","auto|zstd|lz4|brotli|lzma|none"),("-l, --level","1-22"),("-p, --password","Encryption password"),("--chunk-size","Chunk size MB (1-64)"),("--encrypt-names","Encrypt file names")] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(flag).color(colors.accent).family(egui::FontFamily::Monospace).size(13.0));
                ui.label(RichText::new(desc).color(colors.text_secondary).size(13.0));
            });
        }
    });

    ui.add_space(4.0);
    section_card(ui, &colors, |ui| {
        ui.label(RichText::new(s.help_algorithms_title).color(colors.text_primary).strong().size(15.0));
        ui.add_space(8.0);
        for (n, d) in [("Zstandard","Balanced speed/ratio"),("LZ4","Fastest"),("Brotli","Best for text/web"),("LZMA2","Max compression"),("Auto","Auto-selects by content")] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(n).color(colors.accent).strong().size(13.0));
                ui.label(RichText::new(format!("— {}", d)).color(colors.text_secondary).size(13.0));
            });
        }
    });

    ui.add_space(4.0);
    section_card(ui, &colors, |ui| {
        ui.label(RichText::new(s.help_encryption_title).color(colors.text_primary).strong().size(15.0));
        ui.add_space(4.0);
        ui.label(RichText::new(s.help_encryption_desc).color(colors.text_secondary).size(13.0));
    });
}

// ─── About Panel ────────────────────────────────────────────────────────────

pub fn render_about_panel(ui: &mut Ui, settings: &AppSettings) {
    let s = get_strings(settings.language);
    let colors = ThemeColors::for_theme(settings.theme);

    ui.add_space(32.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new("HardZIP").color(colors.accent).size(32.0).strong());
        ui.add_space(4.0);
        ui.label(RichText::new(s.about_subtitle).color(colors.text_secondary).size(14.0).italics());
        ui.add_space(4.0);
        ui.label(RichText::new(s.about_version).color(colors.text_tertiary).size(12.0));
    });

    ui.add_space(24.0);
    section_card(ui, &colors, |ui| {
        ui.label(RichText::new(s.about_description).color(colors.text_primary).size(14.0));
    });

    ui.add_space(8.0);
    section_card(ui, &colors, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(s.about_author).color(colors.text_secondary).size(13.0));
            ui.label(RichText::new("\u{0141}ukasz Dziubi\u{0144}ski").color(colors.text_primary).strong().size(13.0));
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(s.about_website).color(colors.text_secondary).size(13.0));
            ui.hyperlink_to(RichText::new("www.ydi.pl").color(colors.accent).size(13.0), "https://www.ydi.pl");
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(s.about_email).color(colors.text_secondary).size(13.0));
            ui.hyperlink_to(RichText::new("lukasz@ydi.pl").color(colors.accent).size(13.0), "mailto:lukasz@ydi.pl");
        });
    });

    ui.add_space(8.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(s.about_license).color(colors.text_tertiary).size(12.0));
    });
}
