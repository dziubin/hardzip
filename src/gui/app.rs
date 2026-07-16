//! HardZIP GUI — styl menedżera archiwów (jak 7-Zip)
//! Menu bar + toolbar + file list + status bar

use eframe::egui;
use std::path::PathBuf;

use crate::core::archive::{create_archive, extract_archive, ArchiveOptions};
use crate::core::foreign;
use crate::gui::i18n::get_strings;
use crate::gui::panels::{
    render_about_panel, render_crypto_panel, render_help_panel,
    render_settings_panel, render_watcher_panel, render_dropzone_panel,
    AppSettings, CryptoState, DropZoneState, WatcherState,
};
use crate::gui::theme::{apply_theme, ThemeColors};
use crate::utils::fs::generate_extract_dir;
use crate::utils::progress::format_bytes;

/// View mode
#[derive(Debug, Clone, PartialEq)]
enum View {
    FileManager,
    Settings,
    Help,
    About,
    Crypto,
    Watcher,
}

/// File list display mode
#[derive(Debug, Clone, PartialEq)]
enum ListMode {
    Details,
    List,
    Icons,
}

/// A file entry displayed in the list
#[derive(Debug, Clone)]
struct FileListEntry {
    name: String,
    size: u64,
    modified: String,
    is_dir: bool,
    path: PathBuf,
}

pub struct HardZipApp {
    view: View,
    list_mode: ListMode,
    // File manager state
    current_path: String,
    file_list: Vec<FileListEntry>,
    selected_indices: Vec<usize>,
    archive_open: Option<PathBuf>,
    archive_browsing: Option<PathBuf>,  // Currently viewing inside this archive
    status_message: String,
    // Sub-panels
    settings: AppSettings,
    crypto_state: CryptoState,
    watcher_state: WatcherState,
    dropzone_state: DropZoneState,
    // Dialogs
    show_add_dialog: bool,
    show_extract_dialog: bool,
    extract_output: String,
    extract_password: String,
    add_files: Vec<PathBuf>,
    add_output: String,
    add_password: String,
    add_level: u32,
}

impl Default for HardZipApp {
    fn default() -> Self {
        let mut app = Self {
            view: View::FileManager,
            list_mode: ListMode::Details,
            current_path: "DRIVES".to_string(),
            file_list: Vec::new(),
            selected_indices: Vec::new(),
            archive_open: None,
            archive_browsing: None,
            status_message: "Ready".to_string(),
            settings: AppSettings::default(),
            crypto_state: CryptoState::default(),
            watcher_state: WatcherState::default(),
            dropzone_state: DropZoneState::default(),
            show_add_dialog: false,
            show_extract_dialog: false,
            extract_output: String::new(),
            extract_password: String::new(),
            add_files: Vec::new(),
            add_output: String::new(),
            add_password: String::new(),
            add_level: 19,
        };
        app.refresh_file_list();
        app
    }
}

impl HardZipApp {
    fn refresh_file_list(&mut self) {
        self.file_list.clear();
        self.selected_indices.clear();

        // Special "DRIVES" view — show drive letters (Windows "My Computer")
        if self.current_path == "DRIVES" {
            for letter in b'A'..=b'Z' {
                let drive = format!("{}:\\", letter as char);
                let path = std::path::Path::new(&drive);
                if path.exists() {
                    self.file_list.push(FileListEntry {
                        name: format!("Drive ({}:)", letter as char),
                        size: 0,
                        modified: String::new(),
                        is_dir: true,
                        path: path.to_path_buf(),
                    });
                }
            }
            let s = get_strings(self.settings.language);
            self.status_message = format!("{} {}", self.file_list.len(), s.status_drives);
            return;
        }

        let path = std::path::Path::new(&self.current_path);
        if !path.exists() { return; }

        if let Ok(entries) = std::fs::read_dir(path) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in entries.flatten() {
                let meta = entry.metadata().ok();
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = meta.as_ref()
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        let d = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                        let secs = d.as_secs() as i64;
                        // Simple date format
                        format_timestamp(secs)
                    })
                    .unwrap_or_default();

                let item = FileListEntry {
                    name, size, modified, is_dir, path: entry.path(),
                };

                if is_dir { dirs.push(item); } else { files.push(item); }
            }

            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            // Add ".." entry to go to parent directory
            if let Some(parent) = path.parent() {
                self.file_list.push(FileListEntry {
                    name: "..".to_string(),
                    size: 0,
                    modified: String::new(),
                    is_dir: true,
                    path: parent.to_path_buf(),
                });
            }

            self.file_list.extend(dirs);
            self.file_list.extend(files);
        }

        let s = get_strings(self.settings.language);
        self.status_message = format!("{} {}", self.file_list.len(), s.status_items);
    }

    fn navigate_up(&mut self) {
        // If browsing inside an archive, exit back to file system
        if self.archive_browsing.is_some() {
            let archive_path = self.archive_browsing.take().unwrap();
            if let Some(parent) = archive_path.parent() {
                self.current_path = parent.display().to_string();
            }
            self.refresh_file_list();
            return;
        }

        if self.current_path == "DRIVES" { return; }
        let path = std::path::Path::new(&self.current_path);
        if let Some(parent) = path.parent() {
            if parent == path {
                self.current_path = "DRIVES".to_string();
            } else {
                self.current_path = parent.display().to_string();
            }
        } else {
            self.current_path = "DRIVES".to_string();
        }
        self.refresh_file_list();
    }

    fn open_archive_view(&mut self, archive_path: &PathBuf) {
        match crate::core::archive::read_archive_info(archive_path, None) {
            Ok(info) => {
                self.archive_browsing = Some(archive_path.clone());
                self.file_list.clear();
                self.selected_indices.clear();

                // Add ".." to go back
                self.file_list.push(FileListEntry {
                    name: "..".to_string(),
                    size: 0,
                    modified: String::new(),
                    is_dir: true,
                    path: archive_path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf(),
                });

                // Add archive contents
                for entry in &info.files {
                    self.file_list.push(FileListEntry {
                        name: entry.path.clone(),
                        size: entry.original_size,
                        modified: String::new(),
                        is_dir: entry.is_directory,
                        path: archive_path.clone(), // all point to archive
                    });
                }

                self.current_path = format!("[{}]", archive_path.display());
                self.status_message = format!(
                    "{} | {} | {} \u{2192} {} | {} files",
                    archive_path.file_name().unwrap_or_default().to_string_lossy(),
                    info.header.algorithm,
                    format_bytes(info.header.total_uncompressed_size),
                    format_bytes(info.header.total_compressed_size),
                    info.header.file_count
                );
            }
            Err(e) => {
                self.status_message = format!("Cannot open archive: {}", e);
            }
        }
    }

    fn open_foreign_archive_view(&mut self, archive_path: &PathBuf) {
        let format = foreign::detect_format(archive_path);
        if format.is_none() { return; }

        match foreign::list_archive_contents(archive_path) {
            Ok(entries) => {
                self.archive_browsing = Some(archive_path.clone());
                self.file_list.clear();
                self.selected_indices.clear();

                // Add ".." to go back
                self.file_list.push(FileListEntry {
                    name: "..".to_string(),
                    size: 0,
                    modified: String::new(),
                    is_dir: true,
                    path: archive_path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf(),
                });

                for (name, size, is_dir) in entries {
                    self.file_list.push(FileListEntry {
                        name,
                        size,
                        modified: String::new(),
                        is_dir,
                        path: archive_path.clone(),
                    });
                }

                let total_files = self.file_list.len() - 1;
                let total_size: u64 = self.file_list.iter().map(|e| e.size).sum();
                self.current_path = format!("[{}]", archive_path.display());
                self.status_message = format!("{} | {} files | {}",
                    archive_path.file_name().unwrap_or_default().to_string_lossy(),
                    total_files, format_bytes(total_size));
            }
            Err(e) => {
                self.status_message = format!("Cannot read archive: {}", e);
            }
        }
    }

    fn navigate_into(&mut self, idx: usize) {
        if idx >= self.file_list.len() { return; }
        let entry = &self.file_list[idx];

        // Inside an archive view — ".." exits, other items do nothing (just select)
        if self.archive_browsing.is_some() {
            if entry.name == ".." {
                self.navigate_up();
            }
            return;
        }

        if entry.is_dir {
            if entry.name == ".." {
                self.navigate_up();
            } else {
                self.current_path = entry.path.display().to_string();
                self.refresh_file_list();
            }
        } else {
            // Open archive — show contents inside
            let path = entry.path.clone();
            if crate::utils::fs::is_hza_file(&path) {
                self.open_archive_view(&path);
            } else if foreign::detect_format(&path).is_some() {
                self.open_foreign_archive_view(&path);
            }
        }
    }

    fn do_extract(&mut self) {
        if let Some(ref archive) = self.archive_open.clone() {
            let output = std::path::PathBuf::from(&self.extract_output);
            let pw = if self.extract_password.is_empty() { None } else { Some(self.extract_password.as_str()) };

            let result = if crate::utils::fs::is_hza_file(archive) {
                extract_archive(archive, &output, pw, None)
            } else {
                foreign::extract_foreign(archive, &output, None)
            };

            match result {
                Ok(()) => self.status_message = format!("{}: {}", "Extracted to", output.display()),
                Err(e) => self.status_message = format!("Error: {}", e),
            }
        }
        self.show_extract_dialog = false;
        self.archive_open = None;
    }

    fn do_add(&mut self) {
        if self.add_files.is_empty() { return; }
        let output = if self.add_output.is_empty() {
            crate::utils::fs::generate_archive_name(&self.add_files[0])
        } else { PathBuf::from(&self.add_output) };

        let options = ArchiveOptions {
            algorithm: crate::core::format::Algorithm::Auto,
            level: self.add_level,
            chunk_size: 64 * 1024 * 1024,
            password: if self.add_password.is_empty() { None } else { Some(self.add_password.clone()) },
            encrypt_filenames: false,
        };

        match create_archive(&output, &self.add_files, &options, None) {
            Ok(()) => {
                let size = std::fs::metadata(&output).map(|m| format_bytes(m.len())).unwrap_or_else(|_| "?".into());
                self.status_message = format!("Created: {} ({})", output.display(), size);
            }
            Err(e) => self.status_message = format!("Error: {}", e),
        }
        self.show_add_dialog = false;
        self.add_files.clear();
        self.refresh_file_list();
    }
}

fn format_timestamp(secs: i64) -> String {
    // Simple timestamp without chrono dependency for display
    let days = secs / 86400;
    let years = 1970 + days / 365;
    let rem_days = days % 365;
    let month = rem_days / 30 + 1;
    let day = rem_days % 30 + 1;
    let hour = (secs % 86400) / 3600;
    let min = (secs % 3600) / 60;
    format!("{:04}-{:02}-{:02} {:02}:{:02}", years, month.min(12), day.min(31), hour, min)
}

impl eframe::App for HardZipApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_theme(ctx, self.settings.theme);
        let colors = ThemeColors::for_theme(self.settings.theme);
        let s = get_strings(self.settings.language);

        // Handle drag & drop
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(ref path) = file.path {
                    if crate::utils::fs::is_hza_file(path) || foreign::detect_format(path).is_some() {
                        self.archive_open = Some(path.clone());
                        self.show_extract_dialog = true;
                        self.extract_output = generate_extract_dir(path).display().to_string();
                    } else {
                        self.add_files.push(path.clone());
                        self.show_add_dialog = true;
                    }
                }
            }
        });

        // ─── Menu bar ───────────────────────────────────────────
        egui::TopBottomPanel::top("menubar")
            .frame(egui::Frame::none()
                .fill(colors.surface)
                .inner_margin(egui::Margin::symmetric(8.0, 2.0)))
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(s.menu_file, |ui| {
                        if ui.button(s.menu_open_archive).clicked() {
                            if let Some(p) = rfd::FileDialog::new()
                                .add_filter(s.filter_archives, &["hza","zip","7z","tar","gz","tgz","bz2","xz","rar"])
                                .add_filter(s.filter_all, &["*"])
                                .pick_file() {
                                self.archive_open = Some(p.clone());
                                self.show_extract_dialog = true;
                                self.extract_output = generate_extract_dir(&p).display().to_string();
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(s.menu_exit).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button(s.menu_tools, |ui| {
                        if ui.button(s.menu_crypto).clicked() { self.view = View::Crypto; ui.close_menu(); }
                        if ui.button(s.menu_watcher).clicked() { self.view = View::Watcher; ui.close_menu(); }
                        if ui.button(s.menu_dropzone).clicked() { launch_dropzone(); ui.close_menu(); }
                        ui.separator();
                        if ui.button(s.menu_settings).clicked() { self.view = View::Settings; ui.close_menu(); }
                    });
                    ui.menu_button(s.menu_help, |ui| {
                        if ui.button(s.tab_help).clicked() { self.view = View::Help; ui.close_menu(); }
                        if ui.button(s.tab_about).clicked() { self.view = View::About; ui.close_menu(); }
                    });
                });
            });

        // ─── Toolbar ────────────────────────────────────────────
        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::none()
                .fill(colors.surface_alt)
                .inner_margin(egui::Margin::symmetric(10.0, 6.0))
                .stroke(egui::Stroke::new(0.5_f32, colors.border)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    if ui.button(egui::RichText::new(format!("\u{1F3E0} {}", s.toolbar_home)).size(13.0)).clicked() {
                        self.current_path = "DRIVES".to_string();
                        self.refresh_file_list();
                    }
                    ui.separator();
                    // Add/Pack — uses SELECTED files from the list
                    if ui.button(egui::RichText::new(format!("\u{1F4E6} {}", s.toolbar_add)).size(13.0)).clicked() {
                        let selected_paths: Vec<PathBuf> = self.selected_indices.iter()
                            .filter_map(|&idx| self.file_list.get(idx))
                            .filter(|e| e.name != "..")
                            .map(|e| e.path.clone())
                            .collect();
                        if !selected_paths.is_empty() {
                            self.add_files = selected_paths;
                            self.add_output = String::new();
                            self.show_add_dialog = true;
                        } else {
                            // Fallback: open file picker
                            if let Some(paths) = rfd::FileDialog::new().pick_files() {
                                self.add_files = paths;
                                self.add_output = String::new();
                                self.show_add_dialog = true;
                            }
                        }
                    }
                    if ui.button(egui::RichText::new(format!("\u{1F4E4} {}", s.toolbar_extract)).size(13.0)).clicked() {
                        // If selected file is an archive, extract it directly
                        let archive_from_selection = self.selected_indices.iter()
                            .filter_map(|&idx| self.file_list.get(idx))
                            .find(|e| crate::utils::fs::is_hza_file(&e.path) || foreign::detect_format(&e.path).is_some())
                            .map(|e| e.path.clone());

                        if let Some(p) = archive_from_selection {
                            self.archive_open = Some(p.clone());
                            self.show_extract_dialog = true;
                            self.extract_output = generate_extract_dir(&p).display().to_string();
                        } else if let Some(p) = rfd::FileDialog::new()
                            .add_filter(s.filter_archives, &["hza","zip","7z","tar","gz","tgz","bz2","xz","rar"])
                            .add_filter(s.filter_all, &["*"])
                            .pick_file() {
                            self.archive_open = Some(p.clone());
                            self.show_extract_dialog = true;
                            self.extract_output = generate_extract_dir(&p).display().to_string();
                        }
                    }
                    ui.separator();
                    if ui.button(egui::RichText::new(format!("\u{1F512} {}", s.toolbar_encrypt)).size(13.0)).clicked() {
                        self.view = View::Crypto;
                    }
                    if ui.button(egui::RichText::new("\u{2705} Test").size(13.0)).clicked() {
                        if !self.selected_indices.is_empty() {
                            let idx = self.selected_indices[0];
                            if idx < self.file_list.len() {
                                let path = self.file_list[idx].path.clone();
                                if crate::utils::fs::is_hza_file(&path) {
                                    let temp = std::env::temp_dir().join("hardzip_test");
                                    match extract_archive(&path, &temp, None, None) {
                                        Ok(()) => {
                                            let _ = std::fs::remove_dir_all(&temp);
                                            self.status_message = "\u{2705} Test OK!".to_string();
                                        }
                                        Err(e) => self.status_message = format!("\u{274c} FAILED: {}", e),
                                    }
                                } else {
                                    self.status_message = "Select a .hza archive to test".to_string();
                                }
                            }
                        }
                    }

                    // View mode toggle (right side)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        let d = self.list_mode == ListMode::Details;
                        let l = self.list_mode == ListMode::List;
                        let i = self.list_mode == ListMode::Icons;
                        if ui.selectable_label(i, egui::RichText::new(s.view_icons).size(11.0)).clicked() { self.list_mode = ListMode::Icons; }
                        if ui.selectable_label(l, egui::RichText::new(s.view_list).size(11.0)).clicked() { self.list_mode = ListMode::List; }
                        if ui.selectable_label(d, egui::RichText::new(s.view_details).size(11.0)).clicked() { self.list_mode = ListMode::Details; }
                    });
                });
            });

        // ─── Path bar ───────────────────────────────────────────
        egui::TopBottomPanel::top("pathbar")
            .frame(egui::Frame::none()
                .fill(colors.bg)
                .inner_margin(egui::Margin::symmetric(8.0, 4.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("\u{1F4C1}").size(14.0));
                    let mut path_edit = self.current_path.clone();
                    let r = ui.add(egui::TextEdit::singleline(&mut path_edit)
                        .desired_width(ui.available_width())
                        .font(egui::TextStyle::Monospace));
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.current_path = path_edit;
                        self.refresh_file_list();
                    }
                });
            });

        // ─── Status bar ─────────────────────────────────────────
        egui::TopBottomPanel::bottom("statusbar")
            .frame(egui::Frame::none()
                .fill(colors.surface_alt)
                .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                .stroke(egui::Stroke::new(0.5_f32, colors.border)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&self.status_message)
                        .size(11.0).color(colors.text_secondary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("HardZIP v1.0")
                            .size(10.0).color(colors.text_tertiary));
                    });
                });
            });

        // ─── Dialogs ────────────────────────────────────────────
        // Extract dialog
        if self.show_extract_dialog {
            egui::Window::new(s.dlg_extract_title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    if let Some(ref archive) = self.archive_open {
                        ui.label(format!("{}", archive.file_name().unwrap_or_default().to_string_lossy()));
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(s.dlg_extract_to);
                        ui.add(egui::TextEdit::singleline(&mut self.extract_output).desired_width(250.0));
                        if ui.button("...").clicked() {
                            if let Some(p) = rfd::FileDialog::new().pick_folder() {
                                self.extract_output = p.display().to_string();
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(s.dlg_password);
                        ui.add(egui::TextEdit::singleline(&mut self.extract_password).password(true).desired_width(200.0));
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(format!("  {}  ", s.dlg_extract_btn)).clicked() { self.do_extract(); }
                        if ui.button(format!("  {}  ", s.dlg_cancel)).clicked() { self.show_extract_dialog = false; self.archive_open = None; }
                    });
                });
        }

        // Add (compress) dialog
        if self.show_add_dialog {
            egui::Window::new(s.dlg_add_title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("{}: {} {}", s.toolbar_add, self.add_files.len(), s.dlg_files_selected));
                    for f in self.add_files.iter().take(5) {
                        ui.label(egui::RichText::new(format!("  {}", f.file_name().unwrap_or_default().to_string_lossy())).size(11.0));
                    }
                    if self.add_files.len() > 5 { ui.label(egui::RichText::new(format!("  ... {} {}", self.add_files.len() - 5, s.dlg_and_more)).size(11.0)); }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(s.dlg_save_as);
                        ui.add(egui::TextEdit::singleline(&mut self.add_output).desired_width(250.0).hint_text("(auto)"));
                        if ui.button("...").clicked() {
                            if let Some(p) = rfd::FileDialog::new().add_filter("HardZIP", &["hza"]).save_file() {
                                self.add_output = p.display().to_string();
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(s.dlg_password);
                        ui.add(egui::TextEdit::singleline(&mut self.add_password).password(true).desired_width(200.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label(s.dlg_level);
                        ui.add(egui::Slider::new(&mut self.add_level, 1..=22));
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(format!("  {}  ", s.dlg_compress_btn)).clicked() { self.do_add(); }
                        if ui.button(format!("  {}  ", s.dlg_cancel)).clicked() { self.show_add_dialog = false; self.add_files.clear(); }
                    });
                });
        }

        // ─── Central panel: file list or sub-views ──────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(colors.bg)
                .inner_margin(egui::Margin::same(0.0)))
            .show(ctx, |ui| {
                match self.view {
                    View::FileManager => self.render_file_list(ui, &colors),
                    View::Settings => {
                        self.render_subview_header(ui, "Settings", &colors);
                        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                            egui::Frame::none().inner_margin(egui::Margin::same(16.0)).show(ui, |ui| {
                                render_settings_panel(ui, &mut self.settings);
                            });
                        });
                    }
                    View::Help => {
                        self.render_subview_header(ui, "Help", &colors);
                        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                            egui::Frame::none().inner_margin(egui::Margin::same(16.0)).show(ui, |ui| {
                                render_help_panel(ui, &self.settings);
                            });
                        });
                    }
                    View::About => {
                        self.render_subview_header(ui, "About", &colors);
                        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                            egui::Frame::none().inner_margin(egui::Margin::same(16.0)).show(ui, |ui| {
                                render_about_panel(ui, &self.settings);
                            });
                        });
                    }
                    View::Crypto => {
                        self.render_subview_header(ui, "Crypto", &colors);
                        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                            egui::Frame::none().inner_margin(egui::Margin::same(16.0)).show(ui, |ui| {
                                render_crypto_panel(ui, &mut self.crypto_state, &self.settings);
                            });
                        });
                    }
                    View::Watcher => {
                        self.render_subview_header(ui, "Watcher", &colors);
                        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                            egui::Frame::none().inner_margin(egui::Margin::same(16.0)).show(ui, |ui| {
                                render_watcher_panel(ui, &mut self.watcher_state, &self.settings);
                            });
                        });
                    }
                }
            });
    }
}

impl HardZipApp {
    /// Renders a back-navigation header bar at the top of sub-views
    fn render_subview_header(&mut self, ui: &mut egui::Ui, title: &str, colors: &ThemeColors) {
        egui::Frame::none()
            .fill(colors.surface_alt)
            .inner_margin(egui::Margin::symmetric(12.0, 6.0))
            .stroke(egui::Stroke::new(0.5_f32, colors.border))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("\u{2190} Back").size(12.0)).clicked() {
                        self.view = View::FileManager;
                    }
                    ui.separator();
                    ui.label(egui::RichText::new(title).size(13.0).strong().color(colors.text_primary));
                });
            });
    }

    fn render_file_list(&mut self, ui: &mut egui::Ui, colors: &ThemeColors) {
        match self.list_mode {
            ListMode::Details => self.render_details_view(ui, colors),
            ListMode::List => self.render_list_view(ui, colors),
            ListMode::Icons => self.render_icons_view(ui, colors),
        }
    }

    fn render_details_view(&mut self, ui: &mut egui::Ui, colors: &ThemeColors) {
        // Table header
        egui::Frame::none()
            .fill(colors.surface_alt)
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .stroke(egui::Stroke::new(0.5_f32, colors.border))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Name").size(11.0).strong().color(colors.text_secondary));
                    ui.add_space(220.0);
                    ui.label(egui::RichText::new("Size").size(11.0).strong().color(colors.text_secondary));
                    ui.add_space(70.0);
                    ui.label(egui::RichText::new("Modified").size(11.0).strong().color(colors.text_secondary));
                });
            });

        let mut clicked_idx: Option<usize> = None;
        let mut dbl_clicked_idx: Option<usize> = None;

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (idx, entry) in self.file_list.iter().enumerate() {
                let selected = self.selected_indices.contains(&idx);
                let row_height = 22.0;

                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), row_height),
                    egui::Sense::click(),
                );

                // Background
                let bg = if selected { colors.accent }
                    else if response.hovered() { colors.surface_alt }
                    else if idx % 2 == 0 { colors.bg }
                    else { colors.surface_alt };
                ui.painter().rect_filled(rect, 0.0, bg);

                let text_color = if selected { egui::Color32::WHITE } else { colors.text_primary };
                let dim_color = if selected { egui::Color32::WHITE } else { colors.text_secondary };

                // Icon + Name
                let icon = if entry.is_dir { "\u{1F4C1}" } else { "\u{1F4C4}" };
                let icon_pos = egui::pos2(rect.left() + 16.0, rect.center().y);
                ui.painter().text(icon_pos, egui::Align2::LEFT_CENTER,
                    icon, egui::FontId::new(12.0, egui::FontFamily::Proportional), text_color);

                let name_pos = egui::pos2(rect.left() + 34.0, rect.center().y);
                let name_display = if entry.name.len() > 30 { format!("{}...", &entry.name[..27]) } else { entry.name.clone() };
                ui.painter().text(name_pos, egui::Align2::LEFT_CENTER,
                    &name_display, egui::FontId::new(12.0, egui::FontFamily::Proportional), text_color);

                // Size
                if !entry.is_dir {
                    let size_pos = egui::pos2(rect.left() + 270.0, rect.center().y);
                    ui.painter().text(size_pos, egui::Align2::LEFT_CENTER,
                        &format_bytes(entry.size), egui::FontId::new(11.0, egui::FontFamily::Proportional), dim_color);
                }

                // Modified
                let date_pos = egui::pos2(rect.left() + 360.0, rect.center().y);
                ui.painter().text(date_pos, egui::Align2::LEFT_CENTER,
                    &entry.modified, egui::FontId::new(11.0, egui::FontFamily::Proportional), dim_color);

                if response.double_clicked() { dbl_clicked_idx = Some(idx); }
                else if response.clicked() { clicked_idx = Some(idx); }
            }
        });

        if let Some(idx) = dbl_clicked_idx { self.navigate_into(idx); }
        else if let Some(idx) = clicked_idx {
            self.selected_indices = vec![idx];
            let e = &self.file_list[idx];
            if e.is_dir {
                self.status_message = format!("Folder: {}", e.name);
            } else {
                self.status_message = format!("{} | {}", e.name, format_bytes(e.size));
            }
        }
    }

    fn render_list_view(&mut self, ui: &mut egui::Ui, colors: &ThemeColors) {
        let mut clicked_idx: Option<usize> = None;
        let mut dbl_clicked_idx: Option<usize> = None;

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (idx, entry) in self.file_list.iter().enumerate() {
                let selected = self.selected_indices.contains(&idx);
                let icon = if entry.is_dir { "\u{1F4C1}" } else { "\u{1F4C4}" };

                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 20.0),
                    egui::Sense::click(),
                );

                if selected {
                    ui.painter().rect_filled(rect, egui::Rounding::same(3.0), colors.accent);
                } else if response.hovered() {
                    ui.painter().rect_filled(rect, 0.0, colors.surface_alt);
                }

                let tc = if selected { egui::Color32::WHITE } else { colors.text_primary };
                ui.painter().text(egui::pos2(rect.left() + 8.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &format!("{} {}", icon, entry.name),
                    egui::FontId::new(12.0, egui::FontFamily::Proportional), tc);

                if response.double_clicked() { dbl_clicked_idx = Some(idx); }
                else if response.clicked() { clicked_idx = Some(idx); }
            }
        });

        if let Some(idx) = dbl_clicked_idx { self.navigate_into(idx); }
        else if let Some(idx) = clicked_idx { self.selected_indices = vec![idx]; }
    }

    fn render_icons_view(&mut self, ui: &mut egui::Ui, colors: &ThemeColors) {
        let mut clicked_idx: Option<usize> = None;
        let mut dbl_clicked_idx: Option<usize> = None;

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let available_width = ui.available_width();
            let item_width = 100.0_f32;
            let item_height = 80.0_f32;
            let cols = (available_width / item_width).max(1.0) as usize;
            let rows = (self.file_list.len() + cols - 1) / cols;

            for row in 0..rows {
                ui.horizontal(|ui| {
                    for col in 0..cols {
                        let idx = row * cols + col;
                        if idx >= self.file_list.len() { break; }

                        let entry = &self.file_list[idx];
                        let selected = self.selected_indices.contains(&idx);
                        let icon = if entry.is_dir { "\u{1F4C1}" } else { "\u{1F4C4}" };

                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(item_width, item_height),
                            egui::Sense::click(),
                        );

                        // Draw background on selection
                        if selected {
                            ui.painter().rect(rect, egui::Rounding::same(6.0),
                                colors.accent, egui::Stroke::new(2.0_f32, colors.accent));
                        } else if response.hovered() {
                            ui.painter().rect(rect, egui::Rounding::same(4.0),
                                colors.surface_alt, egui::Stroke::new(1.0_f32, colors.border));
                        }

                        // Draw icon
                        let icon_pos = egui::pos2(rect.center().x, rect.top() + 22.0);
                        ui.painter().text(icon_pos, egui::Align2::CENTER_CENTER,
                            icon, egui::FontId::new(26.0, egui::FontFamily::Proportional),
                            if selected { egui::Color32::WHITE } else { colors.text_primary });

                        // Draw name
                        let name_pos = egui::pos2(rect.center().x, rect.bottom() - 14.0);
                        let name_short = if entry.name.len() > 10 {
                            format!("{}...", &entry.name[..8])
                        } else { entry.name.clone() };
                        ui.painter().text(name_pos, egui::Align2::CENTER_CENTER,
                            &name_short, egui::FontId::new(10.0, egui::FontFamily::Proportional),
                            if selected { egui::Color32::WHITE } else { colors.text_primary });

                        if response.double_clicked() { dbl_clicked_idx = Some(idx); }
                        else if response.clicked() { clicked_idx = Some(idx); }
                    }
                });
            }
        });

        if let Some(idx) = dbl_clicked_idx { self.navigate_into(idx); }
        else if let Some(idx) = clicked_idx { self.selected_indices = vec![idx]; }
    }
}

fn launch_dropzone() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).arg("--dropzone").spawn();
    }
}

/// Loads the application icon from the embedded .ico file
fn load_icon() -> std::sync::Arc<egui::IconData> {
    let ico_bytes = include_bytes!("../../assets/hardzip.ico");
    // Parse ICO: find the largest image entry
    // ICO format: 6 byte header, then 16-byte entries
    // For simplicity, try to decode as image
    let img = image::load_from_memory(ico_bytes)
        .unwrap_or_else(|_| image::DynamicImage::new_rgba8(32, 32))
        .into_rgba8();
    let (w, h) = img.dimensions();
    std::sync::Arc::new(egui::IconData {
        rgba: img.into_raw(),
        width: w,
        height: h,
    })
}

pub fn run_dropzone_gui() {
    let icon = load_icon();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 350.0])
            .with_min_inner_size([300.0, 280.0])
            .with_title("HardZIP Drop Zone")
            .with_always_on_top()
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native("HardZIP Drop Zone", options,
        Box::new(|_cc| Ok(Box::new(DropZoneApp::default()))),
    ).expect("Failed to launch Drop Zone");
}

struct DropZoneApp { state: DropZoneState, settings: AppSettings }
impl Default for DropZoneApp {
    fn default() -> Self { Self { state: DropZoneState::default(), settings: AppSettings::default() } }
}
impl eframe::App for DropZoneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        apply_theme(ctx, self.settings.theme);
        ctx.input(|i| { for f in &i.raw.dropped_files { if let Some(ref p) = f.path { self.state.dropped_file = Some(p.clone()); } } });
        egui::CentralPanel::default().show(ctx, |ui| { render_dropzone_panel(ui, &mut self.state, &self.settings); });
    }
}

pub fn run_gui() {
    let icon = load_icon();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([750.0, 520.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("\u{1F4E6} HardZIP")
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native("HardZIP", options,
        Box::new(|_cc| Ok(Box::new(HardZipApp::default()))),
    ).expect("Failed to launch HardZIP GUI");
}

/// Opens GUI and immediately shows the contents of an archive
pub fn run_gui_with_file(file_path: &str) {
    let icon = load_icon();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([750.0, 520.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("\u{1F4E6} HardZIP")
            .with_icon(icon),
        ..Default::default()
    };
    let path = file_path.to_string();
    eframe::run_native("HardZIP", options,
        Box::new(move |_cc| {
            let mut app = HardZipApp::default();
            let p = PathBuf::from(&path);
            if crate::utils::fs::is_hza_file(&p) {
                app.open_archive_view(&p);
            } else if foreign::detect_format(&p).is_some() {
                app.open_foreign_archive_view(&p);
            } else {
                if let Some(parent) = p.parent() {
                    app.current_path = parent.display().to_string();
                    app.refresh_file_list();
                }
            }
            Ok(Box::new(app))
        }),
    ).expect("Failed to launch HardZIP GUI");
}
