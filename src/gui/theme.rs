//! Apple-style themes — Light and Dark mode
//!
//! Inspired by macOS/iOS design language:
//! - Clean, minimal UI with generous spacing
//! - Subtle shadows and rounded corners
//! - SF-style typography hierarchy
//! - Muted, elegant color palette

use egui::{Color32, FontFamily, FontId, Rounding, Stroke, Style, TextStyle, Visuals};

/// Theme choice
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

/// Apple-inspired color palette
pub struct Colors;

impl Colors {
    // ─── Accent (Apple Blue) ────────────────────────────────
    pub const ACCENT: Color32 = Color32::from_rgb(0, 122, 255);        // #007AFF
    pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0, 102, 224);  // darker
    pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(64, 156, 255); // lighter

    // ─── Semantic ───────────────────────────────────────────
    pub const SUCCESS: Color32 = Color32::from_rgb(52, 199, 89);       // #34C759
    pub const WARNING: Color32 = Color32::from_rgb(255, 149, 0);       // #FF9500
    pub const ERROR: Color32 = Color32::from_rgb(255, 59, 48);         // #FF3B30

    // ─── Light theme ────────────────────────────────────────
    pub const LIGHT_BG: Color32 = Color32::from_rgb(246, 246, 246);          // main bg
    pub const LIGHT_SURFACE: Color32 = Color32::from_rgb(255, 255, 255);     // cards
    pub const LIGHT_SURFACE_ALT: Color32 = Color32::from_rgb(242, 242, 247); // grouped bg
    pub const LIGHT_BORDER: Color32 = Color32::from_rgb(209, 209, 214);      // separator
    pub const LIGHT_TEXT_PRIMARY: Color32 = Color32::from_rgb(0, 0, 0);
    pub const LIGHT_TEXT_SECONDARY: Color32 = Color32::from_rgb(99, 99, 102);
    pub const LIGHT_TEXT_TERTIARY: Color32 = Color32::from_rgb(142, 142, 147);
    pub const LIGHT_INPUT_BG: Color32 = Color32::from_rgb(239, 239, 244);
    pub const LIGHT_BUTTON_BG: Color32 = Color32::from_rgb(229, 229, 234);

    // ─── Dark theme ─────────────────────────────────────────
    pub const DARK_BG: Color32 = Color32::from_rgb(28, 28, 30);             // main bg
    pub const DARK_SURFACE: Color32 = Color32::from_rgb(44, 44, 46);        // cards
    pub const DARK_SURFACE_ALT: Color32 = Color32::from_rgb(36, 36, 38);    // grouped bg
    pub const DARK_BORDER: Color32 = Color32::from_rgb(56, 56, 58);         // separator
    pub const DARK_TEXT_PRIMARY: Color32 = Color32::from_rgb(255, 255, 255);
    pub const DARK_TEXT_SECONDARY: Color32 = Color32::from_rgb(152, 152, 157);
    pub const DARK_TEXT_TERTIARY: Color32 = Color32::from_rgb(99, 99, 102);
    pub const DARK_INPUT_BG: Color32 = Color32::from_rgb(58, 58, 60);
    pub const DARK_BUTTON_BG: Color32 = Color32::from_rgb(72, 72, 74);
}

/// Returns contextual colors based on current theme
pub struct ThemeColors {
    pub bg: Color32,
    pub surface: Color32,
    pub surface_alt: Color32,
    pub border: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub input_bg: Color32,
    pub button_bg: Color32,
    pub accent: Color32,
    pub success: Color32,
    pub error: Color32,
}

impl ThemeColors {
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self {
                bg: Colors::LIGHT_BG,
                surface: Colors::LIGHT_SURFACE,
                surface_alt: Colors::LIGHT_SURFACE_ALT,
                border: Colors::LIGHT_BORDER,
                text_primary: Colors::LIGHT_TEXT_PRIMARY,
                text_secondary: Colors::LIGHT_TEXT_SECONDARY,
                text_tertiary: Colors::LIGHT_TEXT_TERTIARY,
                input_bg: Colors::LIGHT_INPUT_BG,
                button_bg: Colors::LIGHT_BUTTON_BG,
                accent: Colors::ACCENT,
                success: Colors::SUCCESS,
                error: Colors::ERROR,
            },
            Theme::Dark => Self {
                bg: Colors::DARK_BG,
                surface: Colors::DARK_SURFACE,
                surface_alt: Colors::DARK_SURFACE_ALT,
                border: Colors::DARK_BORDER,
                text_primary: Colors::DARK_TEXT_PRIMARY,
                text_secondary: Colors::DARK_TEXT_SECONDARY,
                text_tertiary: Colors::DARK_TEXT_TERTIARY,
                input_bg: Colors::DARK_INPUT_BG,
                button_bg: Colors::DARK_BUTTON_BG,
                accent: Colors::ACCENT,
                success: Colors::SUCCESS,
                error: Colors::ERROR,
            },
        }
    }
}

/// Apply the Apple-style theme to egui context
pub fn apply_theme(ctx: &egui::Context, theme: Theme) {
    let colors = ThemeColors::for_theme(theme);
    let mut style = Style::default();

    let mut visuals = match theme {
        Theme::Light => Visuals::light(),
        Theme::Dark => Visuals::dark(),
    };

    // ─── Panel & window ─────────────────────────────────────
    visuals.override_text_color = Some(colors.text_primary);
    visuals.panel_fill = colors.bg;
    visuals.window_fill = colors.surface;
    visuals.extreme_bg_color = colors.input_bg;
    visuals.faint_bg_color = colors.surface_alt;

    // Rounded, subtle windows
    visuals.window_rounding = Rounding::same(12.0);
    visuals.window_stroke = Stroke::new(1.0_f32, colors.border);

    // ─── Widget styling (Apple-like rounded, clean) ─────────
    // Inactive state
    visuals.widgets.inactive.bg_fill = colors.input_bg;
    visuals.widgets.inactive.weak_bg_fill = colors.input_bg;
    visuals.widgets.inactive.rounding = Rounding::same(8.0);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, colors.border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, colors.text_secondary);

    // Hovered state
    visuals.widgets.hovered.bg_fill = colors.button_bg;
    visuals.widgets.hovered.weak_bg_fill = colors.button_bg;
    visuals.widgets.hovered.rounding = Rounding::same(8.0);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, colors.accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, colors.text_primary);

    // Active/pressed state
    visuals.widgets.active.bg_fill = colors.accent;
    visuals.widgets.active.weak_bg_fill = colors.accent;
    visuals.widgets.active.rounding = Rounding::same(8.0);
    visuals.widgets.active.bg_stroke = Stroke::NONE;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

    // Non-interactive (labels, etc.)
    visuals.widgets.noninteractive.bg_fill = colors.surface_alt;
    visuals.widgets.noninteractive.weak_bg_fill = colors.surface_alt;
    visuals.widgets.noninteractive.rounding = Rounding::same(8.0);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.5_f32, colors.border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, colors.text_primary);

    // Open (dropdown, etc.)
    visuals.widgets.open.bg_fill = colors.surface;
    visuals.widgets.open.weak_bg_fill = colors.surface;
    visuals.widgets.open.rounding = Rounding::same(8.0);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, colors.accent);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, colors.text_primary);

    // Selection
    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(0, 122, 255, 50);
    visuals.selection.stroke = Stroke::new(1.0_f32, colors.accent);

    style.visuals = visuals;

    // ─── Typography (clean, Apple-like sizes) ────────────────
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(20.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(12.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(13.0, FontFamily::Monospace),
    );

    // ─── Spacing (generous, airy) ───────────────────────────
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(16.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(16.0);
    style.spacing.indent = 20.0;

    ctx.set_style(style);
}
