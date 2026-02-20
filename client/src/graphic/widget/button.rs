use crate::graphic::color::Rgba;
use crate::graphic::font::get_text_width;
use crate::graphic::render::Renderer;
use crate::graphic::ui::Theme;

pub struct Button {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub text: String,
    pub bg_color: Rgba,
    pub text_color: Rgba,
    pub on_click: Option<Box<dyn FnMut() + Send + Sync>>,
    is_hovered: bool,
}

impl Button {
    pub fn new(x: f32, y: f32, w: f32, h: f32, text: &str) -> Self {
        Self {
            x,
            y,
            w,
            h,
            text: text.to_string(),
            bg_color: Rgba::new_rgb(0.2, 0.2, 0.2, 1.0),
            text_color: Rgba::new_rgb(1.0, 1.0, 1.0, 1.0),
            on_click: None,
            is_hovered: false,
        }
    }

    pub fn with_bg_color(mut self, rgba: Rgba) -> Self {
        self.bg_color = rgba;
        self
    }

    pub fn with_text_color(mut self, rgba: Rgba) -> Self {
        self.text_color = rgba;
        self
    }

    pub fn on_click<F>(mut self, callback: F) -> Self
    where
        F: FnMut() + Send + Sync + 'static,
    {
        self.on_click = Some(Box::new(callback));
        self
    }

    pub fn update(&mut self, mx: f32, my: f32, _left_down: bool, scale_f: f32) {
        self.is_hovered = mx >= self.x * scale_f
            && mx <= (self.x + self.w) * scale_f
            && my >= self.y * scale_f
            && my <= (self.y + self.h) * scale_f;
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
            if let Some(ref mut cb) = self.on_click {
                cb();
            }
            return true;
        }
        false
    }

    pub fn draw(&mut self, renderer: &mut Renderer, _theme: &Theme, scale_f: f32) {
        let actual_bg = if self.is_hovered {
            Rgba::new_rgb(
                (self.bg_color.r + 0.2).min(1.0),
                (self.bg_color.g + 0.2).min(1.0),
                (self.bg_color.b + 0.2).min(1.0),
                self.bg_color.a,
            )
        } else {
            self.bg_color
        };

        unsafe {
            renderer.set_color(actual_bg);
            renderer.draw_rect(
                (self.x * scale_f) as i32,
                (self.y * scale_f) as i32,
                (self.w * scale_f) as i32,
                (self.h * scale_f) as i32,
            );

            let t_width = get_text_width(&self.text, scale_f as i32);
            let text_x = (self.x * scale_f) as i32 + ((self.w * scale_f) as i32 - t_width) / 2;
            let text_y =
                (self.y * scale_f) as i32 + ((self.h * scale_f) as i32 - (7 * scale_f as i32)) / 2;

            crate::graphic::font::draw_text(
                renderer,
                &self.text,
                text_x,
                text_y,
                self.text_color,
                scale_f as i32,
            );
        }
    }
}
