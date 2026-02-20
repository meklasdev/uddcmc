use crate::client::DarkClient;
use crate::graphic::render::Renderer;
use crate::graphic::ui::Theme;

pub struct Toggle {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub module_name: String,
    pub setting_name: String,
    pub is_expanded_anim: f32,
    is_hovered: bool,
}

impl Toggle {
    pub fn new(module_name: &str, setting_name: &str) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 160.0,
            h: 14.0,
            module_name: module_name.to_string(),
            setting_name: setting_name.to_string(),
            is_expanded_anim: 0.0,
            is_hovered: false,
        }
    }

    pub fn update(&mut self, mx: f32, my: f32, _left_down: bool, scale_f: f32) {
        let sx = self.x * scale_f;
        let sy = self.y * scale_f;
        let sw = self.w * scale_f;
        let sh = self.h * scale_f;

        self.is_hovered = mx >= sx && mx <= sx + sw && my >= sy && my <= sy + sh;
    }

    pub fn handle_click(
        &mut self,
        _mx: f32,
        _my: f32,
        left_clicked: bool,
        _right_clicked: bool,
        _scale_f: f32,
    ) -> bool {
        if self.is_hovered && left_clicked {
            if let Ok(modules) = DarkClient::instance().modules.read() {
                if let Some(m) = modules.get(&self.module_name) {
                    if let Some(setting) = m
                        .lock()
                        .unwrap()
                        .get_module_data_mut()
                        .get_setting_mut(&self.setting_name)
                    {
                        let current = setting.get_toggle_value().unwrap_or(false);
                        setting.set_toggle_value(!current);
                    }
                }
            }
            return true;
        }
        false
    }

    pub fn draw(&mut self, renderer: &mut Renderer, theme: &Theme, scale_f: f32) {
        if self.is_expanded_anim < 0.01 {
            return;
        }

        let mut val = false;
        if let Ok(modules) = DarkClient::instance().modules.read() {
            if let Some(m) = modules.get(&self.module_name) {
                if let Some(setting) = m
                    .lock()
                    .unwrap()
                    .get_module_data()
                    .get_setting(&self.setting_name)
                {
                    if let Some(v) = setting.get_toggle_value() {
                        val = v;
                    }
                }
            }
        }

        let bg_color = if self.is_hovered {
            theme.module_bg_hover
        } else {
            theme.module_bg
        };

        let sx = self.x * scale_f;
        let sy = self.y * scale_f;
        let sw = self.w * scale_f;
        let sh = self.h * scale_f;

        unsafe {
            // Draw Background
            renderer.set_color(bg_color.with_alpha(bg_color.a * self.is_expanded_anim));
            renderer.draw_rect(sx as i32, sy as i32, sw as i32, sh as i32);

            // Draw Checkbox Box
            let box_s = 8.0 * scale_f;
            let padding_r = 10.0 * scale_f;
            let box_x = sx + sw - box_s - padding_r;
            let box_y = sy + (sh - box_s) / 2.0;

            renderer.set_color(theme.border.with_alpha(self.is_expanded_anim * 0.5));
            renderer.draw_rect(
                (box_x - 1.0) as i32,
                (box_y - 1.0) as i32,
                (box_s + 2.0) as i32,
                (box_s + 2.0) as i32,
            );

            let box_bg = if val {
                theme.text_accent
            } else {
                theme.window_bg
            };
            renderer.set_color(box_bg.with_alpha(self.is_expanded_anim));
            renderer.draw_rect(box_x as i32, box_y as i32, box_s as i32, box_s as i32);

            // Draw Text
            crate::graphic::font::draw_text(
                renderer,
                &self.setting_name,
                (sx + 5.0 * scale_f) as i32,
                (sy + (sh - 7.0 * scale_f) / 2.0) as i32,
                theme.text_primary.with_alpha(self.is_expanded_anim),
                scale_f as i32,
            );
        }
    }
}
