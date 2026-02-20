use crate::client::DarkClient;
use crate::graphic::font::get_text_width;
use crate::graphic::render::Renderer;
use crate::graphic::ui::Theme;
use crate::graphic::widget::{Slider, Toggle, Widget};
use crate::module::ModuleSetting;

pub struct ModuleButton {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub name: String,

    // Internal States
    pub is_expanded: bool,
    pub expand_anim: f32,
    pub settings: Vec<Widget>,

    is_hovered: bool,
    is_box_hovered: bool,
}

impl ModuleButton {
    pub fn new(name: &str) -> Self {
        let mut settings_widgets = Vec::new();
        if let Ok(modules) = DarkClient::instance().modules.read() {
            if let Some(m) = modules.get(name) {
                let lock = m.lock().unwrap();
                for setting in &lock.get_module_data().settings {
                    match setting {
                        ModuleSetting::Toggle { name: s_name, .. } => {
                            settings_widgets.push(Widget::Toggle(Toggle::new(name, s_name)));
                        }
                        ModuleSetting::Slider {
                            name: s_name,
                            min,
                            max,
                            ..
                        } => {
                            settings_widgets
                                .push(Widget::Slider(Slider::new(name, s_name, *min, *max)));
                        }
                        _ => {}
                    }
                }
            }
        }

        Self {
            x: 0.0,
            y: 0.0,
            w: 160.0,
            h: 16.0,
            name: name.to_string(),
            is_expanded: false,
            expand_anim: 0.0,
            settings: settings_widgets,
            is_hovered: false,
            is_box_hovered: false,
        }
    }

    pub fn update(&mut self, mx: f32, my: f32, _left_down: bool, scale_f: f32) {
        let scaled_x = self.x * scale_f;
        let scaled_y = self.y * scale_f;
        let scaled_w = self.w * scale_f;
        let scaled_h = self.h * scale_f;

        let box_side = 14.0 * scale_f;
        let box_x = scaled_x + scaled_w - box_side - (2.0 * scale_f);
        let box_y = scaled_y + (scaled_h - box_side) / 2.0;

        self.is_hovered = mx >= scaled_x
            && mx <= scaled_x + scaled_w
            && my >= scaled_y
            && my <= scaled_y + scaled_h;

        self.is_box_hovered =
            mx >= box_x && mx <= box_x + box_side && my >= box_y && my <= box_y + box_side;

        if self.is_expanded {
            if self.expand_anim < 1.0 {
                self.expand_anim = (self.expand_anim + 0.1).min(1.0);
            }
        } else {
            if self.expand_anim > 0.0 {
                self.expand_anim = (self.expand_anim - 0.1).max(0.0);
            }
        }

        // Calculate dynamic height footprint including settings
        let mut target_h = 16.0; // Base height
        if self.expand_anim > 0.01 {
            let settings_count = self.settings.len() as f32;
            target_h += self.expand_anim * (settings_count * 14.0);
        }
        self.h = target_h;

        // Propagate updates to settings
        if self.expand_anim > 0.01 {
            let mut current_set_y = self.y + 16.0;
            let set_h = 14.0;

            for child in &mut self.settings {
                match child {
                    Widget::Toggle(t) => {
                        t.x = self.x;
                        t.y = current_set_y;
                        t.w = self.w;
                        t.h = set_h;
                        t.is_expanded_anim = self.expand_anim;
                        t.update(mx, my, _left_down, scale_f);
                    }
                    Widget::Slider(s) => {
                        s.x = self.x;
                        s.y = current_set_y;
                        s.w = self.w;
                        s.h = set_h;
                        s.is_expanded_anim = self.expand_anim;
                        s.update(mx, my, _left_down, scale_f);
                    }
                    _ => {}
                }
                current_set_y += set_h;
            }
        }
    }

    pub fn handle_click(
        &mut self,
        mx: f32,
        my: f32,
        left_clicked: bool,
        right_clicked: bool,
        scale_f: f32,
    ) -> bool {
        if self.expand_anim > 0.01 {
            for child in self.settings.iter_mut().rev() {
                if child.handle_click(mx, my, left_clicked, right_clicked, scale_f) {
                    return true;
                }
            }
        }

        if self.is_hovered {
            if left_clicked {
                if self.is_box_hovered {
                    self.is_expanded = !self.is_expanded;
                } else {
                    if let Some(m) = DarkClient::instance()
                        .modules
                        .read()
                        .unwrap()
                        .get(&self.name)
                    {
                        let mut glk = m.lock().unwrap();
                        let currently_enabled = glk.get_module_data().enabled;
                        glk.get_module_data_mut().set_enabled(!currently_enabled);
                        if !currently_enabled {
                            let _ = glk.on_start();
                        } else {
                            let _ = glk.on_stop();
                        }
                    }
                }
                return true;
            }
            if right_clicked {
                self.is_expanded = !self.is_expanded;
                return true;
            }
        }
        false
    }

    pub fn draw(&mut self, renderer: &mut Renderer, theme: &Theme, scale_f: f32) {
        let alpha = 1.0;
        let bg_color = if self.is_hovered {
            theme.module_bg_hover
        } else {
            theme.module_bg
        };

        unsafe {
            // Background
            renderer.set_color(bg_color.with_alpha(bg_color.a * alpha));
            renderer.draw_rect(
                (self.x * scale_f) as i32,
                (self.y * scale_f) as i32,
                (self.w * scale_f) as i32,
                (16.0 * scale_f) as i32,
            );

            // Fetch state
            let mut is_enabled = false;
            if let Some(m) = DarkClient::instance()
                .modules
                .read()
                .unwrap()
                .get(&self.name)
            {
                is_enabled = m.lock().unwrap().get_module_data().enabled;
            }

            let text_color = if is_enabled {
                theme.text_accent
            } else {
                theme.text_primary
            };

            let t_width = get_text_width(&self.name, scale_f as i32);
            let text_x = (self.x * scale_f) as i32 + ((self.w * scale_f) as i32 - t_width) / 2;
            let text_y =
                (self.y * scale_f) as i32 + ((16.0 * scale_f) as i32 - (7 * scale_f as i32)) / 2;

            // Name
            crate::graphic::font::draw_text(
                renderer,
                &self.name,
                text_x,
                text_y,
                text_color.with_alpha(alpha),
                scale_f as i32,
            );

            // Expansion Box
            let box_side = 14.0 * scale_f;
            let box_x = (self.x * scale_f) + (self.w * scale_f) - box_side - (2.0 * scale_f);
            let box_y = (self.y * scale_f) + ((16.0 * scale_f) - box_side) / 2.0;

            let box_bg = if self.is_box_hovered {
                theme.module_bg_hover
            } else {
                theme.module_bg
            };

            renderer.set_color(theme.border.with_alpha(alpha * 0.5));
            renderer.draw_rect(
                box_x as i32 - 1,
                box_y as i32,
                box_side as i32 + 1,
                box_side as i32,
            );
            renderer.set_color(box_bg.with_alpha(box_bg.a * alpha));
            renderer.draw_rect(box_x as i32, box_y as i32, box_side as i32, box_side as i32);

            let arrow = if self.is_expanded { "^" } else { "v" };
            let aw = get_text_width(arrow, scale_f as i32);
            crate::graphic::font::draw_text(
                renderer,
                arrow,
                (box_x + (box_side - aw as f32) / 2.0) as i32,
                (box_y + (box_side - 7.0 * scale_f) / 2.0) as i32,
                theme.text_primary.with_alpha(alpha),
                scale_f as i32,
            );

            // Draw Settings
            if self.expand_anim > 0.01 {
                for child in &mut self.settings {
                    child.draw(renderer, theme, scale_f);
                }
            }
        }
    }
}
