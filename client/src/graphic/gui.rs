use egui::{Color32, Context, Rounding, Stroke, Style, Visuals};
use crate::graphic::{hud, menu};

pub fn render_all(ctx: &Context) {
    // 1. Applichiamo il tuo "Theme" direttamente ai Visuals di Egui
    let mut style = Style::default();
    style.visuals = Visuals::dark();
    style.visuals.window_fill = Color32::from_rgba_unmultiplied(25, 25, 25, 240); // window_bg
    style.visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(128, 0, 255)); // border viola
    style.visuals.window_rounding = Rounding::same(8.0);
    style.visuals.panel_fill = Color32::TRANSPARENT;
    ctx.set_style(style);

    // 2. Disegniamo l'HUD (sempre visibile)
    hud::draw(ctx);

    // 3. Disegniamo il ClickGUI (solo se aperto)
    let is_open = crate::graphic::input::GUI_OPEN.load(std::sync::atomic::Ordering::Relaxed);
    let anim_progress = ctx.animate_bool(egui::Id::new("menu_open_anim"), is_open);

    if anim_progress > 0.0 {
        menu::draw(ctx, anim_progress);
    }
}