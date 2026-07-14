//! Centralized visual theme for the in-game overlay.
//! Optimized for "KRASNOSTAV Minecraft" with deep charcoal background,
//! subtle borders, and glowing neon blue accenting.

use egui::{Color32, Context, Margin, Rounding, Stroke, Style, Vec2, Visuals};
use std::sync::atomic::{AtomicU32, Ordering};

// --- Palette ---------------------------------------------------------------

// #3b82f6 (Neon Blue / Electric Blue)
static ACCENT_COLOR: AtomicU32 = AtomicU32::new(0x3B82F6FF);

/// Brand color: highlights, enabled modules, headers, focus rings.
pub fn accent() -> Color32 {
    let val = ACCENT_COLOR.load(Ordering::Relaxed);
    Color32::from_rgba_unmultiplied(
        ((val >> 24) & 0xFF) as u8,
        ((val >> 16) & 0xFF) as u8,
        ((val >> 8) & 0xFF) as u8,
        (val & 0xFF) as u8,
    )
}

pub fn set_accent(color: Color32) {
    let val = ((color.r() as u32) << 24)
        | ((color.g() as u32) << 16)
        | ((color.b() as u32) << 8)
        | (color.a() as u32);
    ACCENT_COLOR.store(val, Ordering::Relaxed);
}

/// Muted accent for secondary emphasis.
pub fn accent_dim() -> Color32 {
    let c = accent();
    Color32::from_rgb(
        (c.r() as f32 * 0.6) as u8,
        (c.g() as f32 * 0.6) as u8,
        (c.b() as f32 * 0.6) as u8,
    )
}

/// Backgrounds matching Breeze Dev-Local theme.
pub const BASE: Color32 = Color32::from_rgb(15, 15, 17);         // #0F0F11
pub const SURFACE: Color32 = Color32::from_rgb(22, 22, 27);      // #16161B
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(30, 30, 38);
pub const ELEVATED: Color32 = Color32::from_rgb(26, 26, 32);

/// Borders and separators.
pub const BORDER: Color32 = Color32::from_rgb(44, 44, 54);

/// Text shades, brightest to dimmest.
pub const TEXT: Color32 = Color32::from_rgb(240, 242, 248);
pub const TEXT_DIM: Color32 = Color32::from_rgb(155, 158, 172);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(95, 98, 110);

/// Status colors for notifications.
pub const WARN: Color32 = Color32::from_rgb(245, 158, 11);
pub const DANGER: Color32 = Color32::from_rgb(239, 68, 68);

// --- Presets ---------------------------------------------------------------

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum AccentPreset {
    Emerald,
    #[default]
    Aqua,
    Amethyst,
    Ruby,
    Gold,
    Sakura,
}

impl AccentPreset {
    pub fn color(self) -> Color32 {
        match self {
            AccentPreset::Emerald => Color32::from_rgb(16, 185, 129),
            AccentPreset::Aqua => Color32::from_rgb(59, 130, 246), // Neon Blue #3b82f6
            AccentPreset::Amethyst => Color32::from_rgb(139, 92, 246),
            AccentPreset::Ruby => Color32::from_rgb(239, 68, 68),
            AccentPreset::Gold => Color32::from_rgb(245, 158, 11),
            AccentPreset::Sakura => Color32::from_rgb(236, 72, 153),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            AccentPreset::Emerald => "Emerald",
            AccentPreset::Aqua => "Neon Blue",
            AccentPreset::Amethyst => "Amethyst",
            AccentPreset::Ruby => "Ruby",
            AccentPreset::Gold => "Gold",
            AccentPreset::Sakura => "Sakura",
        }
    }
}

// --- Metrics ---------------------------------------------------------------

/// Corner radius for panels.
pub const RADIUS: f32 = 12.0;
/// Corner radius for inner widgets (rows, buttons, cards).
pub const RADIUS_INNER: f32 = 8.0;

// --- Style installation ----------------------------------------------------

/// Builds and installs the overlay style on `ctx`.
pub fn apply(ctx: &Context) {
    let mut style = Style::default();
    let mut v = Visuals::dark();

    let acc = accent();

    v.window_fill = BASE;
    v.panel_fill = BASE;
    v.window_stroke = Stroke::new(1.0_f32, BORDER);
    v.window_rounding = Rounding::same(RADIUS);
    v.window_shadow = egui::epaint::Shadow {
        offset: Vec2::new(0.0, 10.0),
        blur: 40.0,
        spread: 0.0,
        color: Color32::from_black_alpha(180),
    };
    v.popup_shadow = v.window_shadow;

    v.selection.bg_fill = acc;
    v.selection.stroke = Stroke::NONE;
    v.hyperlink_color = acc;

    let w = &mut v.widgets;

    w.noninteractive.bg_fill = SURFACE;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_DIM);

    w.inactive.bg_fill = SURFACE;
    w.inactive.weak_bg_fill = SURFACE;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_DIM);
    w.inactive.rounding = Rounding::same(RADIUS_INNER);
    w.inactive.expansion = 0.0;

    w.hovered.bg_fill = SURFACE_HOVER;
    w.hovered.weak_bg_fill = SURFACE_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT);
    w.hovered.rounding = Rounding::same(RADIUS_INNER);
    w.hovered.expansion = 1.0;

    w.active.bg_fill = acc;
    w.active.weak_bg_fill = acc;
    w.active.bg_stroke = Stroke::NONE;
    w.active.fg_stroke = Stroke::new(1.0_f32, Color32::BLACK);
    w.active.rounding = Rounding::same(RADIUS_INNER);
    w.active.expansion = -1.0;

    w.open.bg_fill = SURFACE_HOVER;
    w.open.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.open.fg_stroke = Stroke::new(1.0_f32, TEXT);

    style.visuals = v;

    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.window_margin = Margin::same(0.0);
    style.spacing.button_padding = Vec2::new(12.0, 8.0);
    style.spacing.interact_size.y = 20.0;
    style.spacing.slider_width = 140.0;

    style.animation_time = 0.22;

    ctx.set_style(style);
}

// --- Shared helpers --------------------------------------------------------

/// Linearly interpolates between two colors.
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        mix(a.r(), b.r()),
        mix(a.g(), b.g()),
        mix(a.b(), b.b()),
        mix(a.a(), b.a()),
    )
}

/// Returns `color` with its alpha scaled by `factor` (`0..=1`).
pub fn with_alpha(color: Color32, factor: f32) -> Color32 {
    let a = (color.a() as f32 * factor.clamp(0.0, 1.0)).round() as u8;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a)
}
