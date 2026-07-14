//! Visual theme — colour palette, widget styling and panel frames.

use egui::{Color32, Context, Frame, Margin, Rounding, Stroke, Vec2};

/// The injector colour palette: a premium ultra-dark base with an indigo accent.
pub struct Palette;

impl Palette {
    /// Window background.
    pub const BG: Color32 = Color32::from_rgb(10, 11, 14);
    /// Header / footer panel background.
    pub const PANEL: Color32 = Color32::from_rgb(17, 18, 22);
    /// Border line color.
    pub const BORDER: Color32 = Color32::from_rgb(32, 34, 44);
    /// Resting process-card background.
    pub const CARD: Color32 = Color32::from_rgb(20, 22, 28);
    /// Hovered process-card background.
    pub const CARD_HOVER: Color32 = Color32::from_rgb(28, 30, 40);
    /// Accent — selection, primary action, branding.
    pub const ACCENT: Color32 = Color32::from_rgb(124, 108, 246);
    /// Tinted background of the selected card.
    pub const ACCENT_SOFT: Color32 = Color32::from_rgb(24, 22, 45);
    /// Primary text.
    pub const TEXT: Color32 = Color32::from_rgb(238, 240, 245);
    /// Secondary / muted text.
    pub const TEXT_DIM: Color32 = Color32::from_rgb(142, 145, 160);
    /// Success.
    pub const OK: Color32 = Color32::from_rgb(74, 210, 149);
    /// In-progress / caution.
    pub const WARN: Color32 = Color32::from_rgb(230, 180, 80);
    /// Failure.
    pub const ERR: Color32 = Color32::from_rgb(229, 107, 107);
}

/// Installs the dark theme on the egui context. Called once at startup.
pub fn apply(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;

    v.dark_mode = true;
    v.panel_fill = Palette::BG;
    v.window_fill = Palette::BG;
    v.extreme_bg_color = Palette::PANEL;
    v.override_text_color = Some(Palette::TEXT);
    v.selection.bg_fill = Palette::ACCENT.linear_multiply(0.4);
    v.selection.stroke = Stroke::new(1.0, Palette::ACCENT);
    v.hyperlink_color = Palette::ACCENT;

    let w = &mut v.widgets;
    w.noninteractive.rounding = Rounding::same(12.0);
    w.inactive.rounding = Rounding::same(12.0);
    w.hovered.rounding = Rounding::same(12.0);
    w.active.rounding = Rounding::same(12.0);
    w.inactive.bg_fill = Palette::CARD;
    w.inactive.weak_bg_fill = Palette::CARD;
    w.inactive.fg_stroke = Stroke::new(1.0, Palette::TEXT_DIM);
    w.hovered.bg_fill = Palette::CARD_HOVER;
    w.hovered.weak_bg_fill = Palette::CARD_HOVER;
    w.hovered.fg_stroke = Stroke::new(1.0, Palette::TEXT);
    w.active.bg_fill = Palette::ACCENT;
    w.active.weak_bg_fill = Palette::ACCENT;

    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(16.0, 10.0);

    ctx.set_style(style);
}

/// Frame for the top header panel.
pub fn header_frame() -> Frame {
    Frame::none()
        .fill(Palette::PANEL)
        .inner_margin(Margin::symmetric(24.0, 20.0))
        .stroke(Stroke::new(1.0, Palette::BORDER))
}

/// Frame for the bottom footer panel.
pub fn footer_frame() -> Frame {
    Frame::none()
        .fill(Palette::PANEL)
        .inner_margin(Margin::symmetric(20.0, 16.0))
        .stroke(Stroke::new(1.0, Palette::BORDER))
}
