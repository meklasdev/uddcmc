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

    // Minimalist Widget Styling
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(20, 20, 20); // Darker thin rails for sliders
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(30, 30, 30));
    visuals.widgets.inactive.rounding = Rounding::same(2.0); // Slight soft curves for interactive elements
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(180, 180, 180));
    visuals.widgets.inactive.expansion = 0.0;

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(35, 35, 35);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(45, 45, 45));
    visuals.widgets.hovered.rounding = Rounding::same(2.0);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.hovered.expansion = 1.0; // slight pop on hover

    visuals.widgets.active.bg_fill = accent_color;
    visuals.widgets.active.bg_stroke = Stroke::NONE;
    visuals.widgets.active.rounding = Rounding::same(2.0);
    visuals.widgets.active.expansion = -1.0; // click compression

    visuals.window_rounding = Rounding::ZERO;
    style.visuals = visuals;

    style.spacing.item_spacing = Vec2::new(0.0, 4.0); // Slightly more breathing room vertically
    style.spacing.window_margin = Margin::symmetric(0.0, 0.0);
    style.spacing.button_padding = Vec2::new(6.0, 4.0);

    // Thinner slider rail, standard widget height
    style.spacing.interact_size.y = 12.0;
    ctx.set_style(style);

    hud::draw(ctx);

    let is_open = crate::graphic::input::GUI_OPEN.load(std::sync::atomic::Ordering::Relaxed);
    let anim_progress = ctx.animate_bool(egui::Id::new("menu_open_anim"), is_open);

    if anim_progress > 0.0 {
        menu::draw(ctx, anim_progress, window_anim_states);
    }
}
