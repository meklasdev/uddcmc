use crate::client::DarkClient;
use egui::{Align2, Color32, Context, Id, RichText};

pub fn draw(ctx: &Context) {
    // --- WATERMARK (In alto a sinistra) ---
    egui::Area::new(Id::new("hud_watermark"))
        .fixed_pos(egui::pos2(5.0, 5.0))
        .interactable(false) // Non blocca i click
        .show(ctx, |ui| {
            ui.add(
                egui::Label::new(
                    RichText::new("DarkClient").color(Color32::YELLOW).size(24.0).strong()
                )
                    .wrap_mode(egui::TextWrapMode::Extend)
            );
        });

    // --- ARRAYLIST / MODULI ATTIVI (Sotto il watermark) ---
    egui::Area::new(Id::new("hud_arraylist"))
        .fixed_pos(egui::pos2(5.0, 35.0))
        .interactable(false)
        .show(ctx, |ui| {
            if let Ok(modules_map) = DarkClient::instance().modules.read() {
                let mut active_mods: Vec<String> = modules_map
                    .values()
                    .filter_map(|m| {
                        let lock = m.lock().unwrap();
                        if lock.get_module_data().enabled {
                            Some(lock.get_module_data().name.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                active_mods.sort_by(|a, b| b.len().cmp(&a.len()));

                let colors = [
                    Color32::from_rgb(153, 51, 204), // Purple
                    Color32::from_rgb(51, 204, 230), // Cyan
                    Color32::GREEN,
                    Color32::RED,
                    Color32::YELLOW,
                    Color32::from_rgb(51, 102, 255), // Blue
                    Color32::WHITE,
                ];

                for (i, mod_name) in active_mods.iter().enumerate() {
                    let color = colors[i % colors.len()];
                    ui.add(
                        egui::Label::new(
                            RichText::new(mod_name).color(color).size(16.0)
                        )
                            .wrap_mode(egui::TextWrapMode::Extend) // <--- Disabilita il word-wrap!
                    );
                }
            }
        });
}