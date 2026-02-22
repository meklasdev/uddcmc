use crate::graphic::ui_engine::WindowAnimState;
use crate::graphic::{hud, menu};
use egui::{Color32, Context, Margin, Rounding, Stroke, Style, Vec2, Visuals};
use std::collections::HashMap;

pub fn render_all(ctx: &Context, window_anim_states: &mut HashMap<String, WindowAnimState>) {
    let mut style = Style::default();
    let mut visuals = Visuals::dark();

    let bg_color = Color32::from_rgb(18, 18, 18);
    let panel_color = Color32::from_rgb(25, 25, 25);
    let accent_color = Color32::from_rgb(26, 171, 138); // Vape's Teal
    let border_color = Color32::from_rgb(35, 35, 35);

    visuals.window_fill = bg_color;
    visuals.panel_fill = panel_color;
    visuals.window_stroke = Stroke::new(1.0, border_color);

    visuals.selection.bg_fill = accent_color;
    visuals.selection.stroke = Stroke::NONE;

    visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
    visuals.widgets.inactive.rounding = Rounding::ZERO;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(210, 210, 210));

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(35, 35, 35);
    visuals.widgets.hovered.rounding = Rounding::ZERO;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    visuals.widgets.active.bg_fill = accent_color;
    visuals.widgets.active.rounding = Rounding::ZERO;

    visuals.window_rounding = Rounding::ZERO;
    style.visuals = visuals;

    style.spacing.item_spacing = Vec2::new(0.0, 0.0);
    style.spacing.window_margin = Margin::symmetric(0.0, 0.0);
    style.spacing.button_padding = Vec2::new(5.0, 5.0);
    ctx.set_style(style);

    hud::draw(ctx);

    let is_open = crate::graphic::input::GUI_OPEN.load(std::sync::atomic::Ordering::Relaxed);
    let anim_progress = ctx.animate_bool(egui::Id::new("menu_open_anim"), is_open);

    if anim_progress > 0.0 {
        menu::draw(ctx, anim_progress, window_anim_states);
    }
}
