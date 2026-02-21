use crate::graphic::{hud, menu};
use egui::{Color32, Context, Margin, Rounding, Stroke, Style, Vec2, Visuals};

pub fn render_all(ctx: &Context) {
    // 1. Applichiamo il tuo "Theme" direttamente ai Visuals di Egui
    let mut style = Style::default();
    let mut visuals = Visuals::dark();
    // 1. Colori Base (Scuri e Piatti)
    let bg_color = Color32::from_rgb(22, 22, 22); // Grigio scurissimo (quasi nero)
    let panel_color = Color32::from_rgb(30, 30, 30); // Sfondo dei moduli
    let accent_color = Color32::from_rgb(26, 171, 138); // Il classico Verde Acqua (Teal) di Vape
    let border_color = Color32::from_rgb(45, 45, 45); // Bordino sottile

    // Sfondi Finestre
    visuals.window_fill = bg_color;
    visuals.panel_fill = panel_color;
    visuals.window_stroke = Stroke::new(1.0, border_color); // Bordo sottile di 1px

    // 2. Colori Attivi (Quando uno slider o checkbox è attivo)
    visuals.selection.bg_fill = accent_color;
    visuals.selection.stroke = Stroke::NONE;

    // 3. Stile Widget (Pulsanti, Background degli Slider)
    // Inattivo
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(35, 35, 35);
    visuals.widgets.inactive.rounding = Rounding::same(2.0); // Leggermente smussato
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(210, 210, 210)); // Testo chiaro

    // Hover (Passaggio del mouse)
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(45, 45, 45);
    visuals.widgets.hovered.rounding = Rounding::same(2.0);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);

    // Click
    visuals.widgets.active.bg_fill = accent_color;
    visuals.widgets.active.rounding = Rounding::same(2.0);

    // 4. Arrotondamento Generale (Vape è squadrata)
    visuals.window_rounding = Rounding::same(3.0);
    style.visuals = visuals;

    // 5. Spaziature (Vape è compattissima)
    style.spacing.item_spacing = Vec2::new(8.0, 4.0); // Spazio ridotto tra gli elementi
    style.spacing.window_margin = Margin::symmetric(0.0, 0.0); // Rimuove il padding ai bordi della finestra!
    style.spacing.button_padding = Vec2::new(4.0, 2.0);
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
