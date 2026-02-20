use crate::graphic::color::Rgba;
use crate::graphic::render::Renderer;
use crate::graphic::ui::Theme;

pub struct Label {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub color: Rgba,
    pub scale_mult: f32,
}

impl Label {
    pub fn new(x: f32, y: f32, text: &str) -> Self {
        Self {
            x,
            y,
            text: text.to_string(),
            color: Rgba::new_rgb(1.0, 1.0, 1.0, 1.0),
            scale_mult: 1.0,
        }
    }

    pub fn with_color(mut self, rgba: Rgba) -> Self {
        self.color = rgba;
        self
    }

    pub fn with_scale(mut self, mult: f32) -> Self {
        self.scale_mult = mult;
        self
    }

    pub fn update(&mut self, _mx: f32, _my: f32, _left_down: bool, _scale_f: f32) {}

    pub fn handle_click(
        &mut self,
        _mx: f32,
        _my: f32,
        _lc: bool,
        _rc: bool,
        _scale_f: f32,
    ) -> bool {
        false
    }

    pub fn draw(&mut self, renderer: &mut Renderer, _theme: &Theme, scale_f: f32) {
        unsafe {
            let final_scale = (scale_f * self.scale_mult) as i32;
            crate::graphic::font::draw_text(
                renderer,
                &self.text,
                (self.x * scale_f) as i32,
                (self.y * scale_f) as i32,
                self.color,
                final_scale,
            );
        }
    }
}
