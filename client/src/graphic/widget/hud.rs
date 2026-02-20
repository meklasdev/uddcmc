use crate::client::DarkClient;
use crate::graphic::font::draw_text;
use crate::graphic::render::Renderer;
use crate::graphic::ui::{HudColor, Theme};

pub struct HudWidget {
    pub is_visible: bool,
}

impl HudWidget {
    pub fn new() -> Self {
        Self { is_visible: true }
    }

    pub fn draw(&mut self, renderer: &mut Renderer, _theme: &Theme, scale_f: f32) {
        if !self.is_visible {
            return;
        }

        let watermark = "DarkClient";
        let w_color = HudColor::Yellow.to_rgba();

        let mut hud_y = 5.0 * scale_f;
        let hud_x = 5.0 * scale_f;
        let text_scale = (2.0 * scale_f) as i32; // Using 2.0 multiplier for large HUD text

        unsafe {
            draw_text(
                renderer,
                watermark,
                hud_x as i32,
                hud_y as i32,
                w_color,
                text_scale,
            );
        }

        hud_y += 24.0 * scale_f; // watermark margin bottom

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

            // Sort by length (longest first)
            active_mods.sort_by(|a, b| b.len().cmp(&a.len()));

            let arraylist_colors = [
                HudColor::Purple,
                HudColor::Cyan,
                HudColor::Green,
                HudColor::Red,
                HudColor::Yellow,
                HudColor::Blue,
                HudColor::White,
            ];

            for (i, mod_name) in active_mods.iter().enumerate() {
                let color = arraylist_colors[i % arraylist_colors.len()].to_rgba();
                let m_scale = (1.5 * scale_f) as i32; // Font slightly smaller than watermark
                unsafe {
                    draw_text(
                        renderer,
                        mod_name,
                        hud_x as i32,
                        hud_y as i32,
                        color,
                        m_scale,
                    );
                }
                hud_y += 18.0 * scale_f; // module spacing
            }
        }
    }
}
