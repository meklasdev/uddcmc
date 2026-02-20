use crate::client::DarkClient;
use crate::graphic::color::Color;
use crate::graphic::render::Renderer;
use crate::graphic::ui::Theme;

pub struct Slider {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub module_name: String,
    pub setting_name: String,
    pub min: f32,
    pub max: f32,
    pub is_expanded_anim: f32,
    is_hovered: bool,
    is_dragging: bool,
}

impl Slider {
    pub fn new(module_name: &str, setting_name: &str, min: f32, max: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 160.0, // default width
            h: 14.0,  // default height
            module_name: module_name.to_string(),
            setting_name: setting_name.to_string(),
            min,
            max,
            is_expanded_anim: 0.0,
            is_hovered: false,
            is_dragging: false,
        }
    }

    pub fn update(&mut self, mx: f32, my: f32, left_down: bool, scale_f: f32) {
        let sx = self.x * scale_f;
        let sy = self.y * scale_f;
        let sw = self.w * scale_f;
        let sh = self.h * scale_f;

        self.is_hovered = mx >= sx && mx <= sx + sw && my >= sy && my <= sy + sh;

        if !left_down {
            self.is_dragging = false;
        } else if self.is_dragging {
            // Compute slider value based on mouse X constraint
            let mut value_pct = (mx - sx) / sw;
            value_pct = value_pct.clamp(0.0, 1.0);
            let mut new_val = self.min + (self.max - self.min) * value_pct;

            // Rounding logic for aesthetics if it's acting like an int or single decimal bounds.
            // Assuming 1 decimal point for general float sliders for clean UI.
            new_val = (new_val * 10.0).round() / 10.0;

            if let Ok(modules) = DarkClient::instance().modules.read() {
                if let Some(m) = modules.get(&self.module_name) {
                    if let Some(setting) = m
                        .lock()
                        .unwrap()
                        .get_module_data_mut()
                        .get_setting_mut(&self.setting_name)
                    {
                        setting.set_slider_value(new_val);
                    }
                }
            }
        }
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
            self.is_dragging = true;
            return true;
        }
        false
    }

    pub fn draw(&mut self, renderer: &mut Renderer, theme: &Theme, scale_f: f32) {
        if self.is_expanded_anim < 0.01 {
            return;
        }

        let mut val = self.min;
        if let Ok(modules) = DarkClient::instance().modules.read() {
            if let Some(m) = modules.get(&self.module_name) {
                if let Some(setting) = m
                    .lock()
                    .unwrap()
                    .get_module_data()
                    .get_setting(&self.setting_name)
                {
                    if let Some(v) = setting.get_slider_value() {
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

        let pct = ((val - self.min) / (self.max - self.min)).clamp(0.0, 1.0);
        let filled_w = sw * pct;

        unsafe {
            // Draw Background Track
            renderer.set_color(bg_color.with_alpha(bg_color.a * self.is_expanded_anim));
            renderer.draw_rect(sx as i32, sy as i32, sw as i32, sh as i32);

            // Draw Filled Track
            let track_col = theme.text_accent;
            renderer.set_color(track_col.with_alpha(self.is_expanded_anim * 0.8));
            renderer.draw_rect(sx as i32, sy as i32, filled_w as i32, sh as i32);

            // Draw Text
            let display_text = format!("{}: {:.1}", self.setting_name, val);
            crate::graphic::font::draw_text(
                renderer,
                &display_text,
                (sx + 5.0 * scale_f) as i32,
                (sy + (sh - 7.0 * scale_f) / 2.0) as i32,
                Color::Black.to_rgba(self.is_expanded_anim),
                scale_f as i32,
            );
        }
    }
}
