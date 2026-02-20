use super::Widget;
use crate::graphic::color::Rgba;
use crate::graphic::render::Renderer;
use crate::graphic::ui::Theme;

pub struct Panel {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub bg_color: Rgba,
    pub children: Vec<Widget>,
    pub is_draggable: bool,

    // Internal state
    is_dragging: bool,
    drag_offset_x: f32,
    drag_offset_y: f32,
}

impl Panel {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            x,
            y,
            w,
            h,
            bg_color: Rgba::new_rgb(0.1, 0.1, 0.1, 0.8),
            children: Vec::new(),
            is_draggable: true,
            is_dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
        }
    }

    pub fn with_bg_color(mut self, rgba: Rgba) -> Self {
        self.bg_color = rgba;
        self
    }

    pub fn with_draggable(mut self, drag: bool) -> Self {
        self.is_draggable = drag;
        self
    }

    pub fn add_child(mut self, widget: Widget) -> Self {
        self.children.push(widget);
        self
    }

    pub fn update(&mut self, mx: f32, my: f32, left_down: bool, scale_f: f32) {
        if self.is_draggable {
            if !left_down {
                self.is_dragging = false;
            } else if self.is_dragging {
                self.x = (mx / scale_f) - self.drag_offset_x;
                self.y = (my / scale_f) - self.drag_offset_y;
            } else {
                // If it's a new click within Draggable Title Area
                if mx >= self.x * scale_f
                    && mx <= (self.x + self.w) * scale_f
                    && my >= self.y * scale_f
                    && my <= (self.y + 20.0) * scale_f
                {
                    self.is_dragging = true;
                    self.drag_offset_x = (mx / scale_f) - self.x;
                    self.drag_offset_y = (my / scale_f) - self.y;
                }
            }
        }

        for child in &mut self.children {
            child.update(mx, my, left_down, scale_f);
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
        // First interact with children uniformly reversed (Top down)
        let mut consumed = false;
        for child in self.children.iter_mut().rev() {
            if child.handle_click(mx, my, left_clicked, right_clicked, scale_f) {
                consumed = true;
                break;
            }
        }

        if consumed {
            return true;
        }

        // Then interact with Panel rect itself
        if mx >= self.x * scale_f
            && mx <= (self.x + self.w) * scale_f
            && my >= self.y * scale_f
            && my <= (self.y + self.h) * scale_f
        {
            if left_clicked || right_clicked {
                return true;
            }
        }

        false
    }

    pub fn draw(&mut self, renderer: &mut Renderer, theme: &Theme, scale_f: f32) {
        unsafe {
            renderer.set_color(self.bg_color);
            renderer.draw_rect(
                (self.x * scale_f) as i32,
                (self.y * scale_f) as i32,
                (self.w * scale_f) as i32,
                (self.h * scale_f) as i32,
            );
        }

        for child in &mut self.children {
            child.draw(renderer, theme, scale_f);
        }
    }
}
