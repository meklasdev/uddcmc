use super::Widget;
use crate::graphic::font::get_text_width;
use crate::graphic::render::Renderer;
use crate::graphic::ui::Theme;

pub struct Window {
    pub x: f32,
    pub y: f32,
    pub render_x: f32,
    pub render_y: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub w: f32,
    pub h: f32,
    pub title: String,
    pub children: Vec<Widget>,

    pub is_dragging: bool,
    drag_offset_x: f32,
    drag_offset_y: f32,
}

impl Window {
    pub fn new(x: f32, y: f32, w: f32, h: f32, title: &str) -> Self {
        Self {
            x,
            y,
            render_x: x,
            render_y: y,
            vel_x: 0.0,
            vel_y: 0.0,
            w,
            h,
            title: title.to_string(),
            children: Vec::new(),
            is_dragging: false,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
        }
    }

    pub fn add_child(mut self, child: Widget) -> Self {
        self.children.push(child);
        self
    }

    pub fn update(&mut self, mx: f32, my: f32, left_down: bool, scale_f: f32) {
        if left_down {
            if self.is_dragging {
                self.x = (mx / scale_f) - self.drag_offset_x;
                self.y = (my / scale_f) - self.drag_offset_y;
            }
        } else {
            self.is_dragging = false;
        }

        // Spring Physics
        let stiffness = 0.25;
        let damping = 0.65;

        let fx = (self.x - self.render_x) * stiffness;
        let fy = (self.y - self.render_y) * stiffness;

        self.vel_x = (self.vel_x + fx) * damping;
        self.vel_y = (self.vel_y + fy) * damping;

        self.render_x += self.vel_x;
        self.render_y += self.vel_y;

        // Propagate update to children iteratively adjusting Y offset
        let mut child_y = self.render_y + 25.0; // below title + padding
        for child in &mut self.children {
            match child {
                Widget::Button(b) => {
                    b.x = self.render_x + 5.0;
                    b.y = child_y;
                    b.update(mx, my, left_down, scale_f);
                    child_y += b.h + 2.0;
                }
                Widget::Label(l) => {
                    l.x = self.render_x + 5.0;
                    l.y = child_y;
                    l.update(mx, my, left_down, scale_f);
                    child_y += 14.0 + 2.0;
                }
                Widget::ModuleButton(m) => {
                    m.x = self.render_x + 5.0;
                    m.y = child_y;
                    m.w = self.w - 10.0;
                    m.update(mx, my, left_down, scale_f);
                    child_y += m.h + 2.0;
                }
                _ => {}
            }
        }

        let target_h = (child_y - self.render_y).max(25.0);
        self.h += (target_h - self.h) * 0.25;
    }

    pub fn handle_click(
        &mut self,
        mx: f32,
        my: f32,
        left_clicked: bool,
        right_clicked: bool,
        scale_f: f32,
    ) -> bool {
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

        let sx = self.render_x * scale_f;
        let sy = self.render_y * scale_f;
        let sw = self.w * scale_f;
        let sh = self.h * scale_f;

        if mx >= sx && mx <= sx + sw && my >= sy && my <= sy + sh {
            if left_clicked || right_clicked {
                if left_clicked {
                    let title_height = 20.0 * scale_f;
                    if my <= sy + title_height {
                        self.is_dragging = true;
                        self.drag_offset_x = (mx - sx) / scale_f;
                        self.drag_offset_y = (my - sy) / scale_f;
                    }
                }
                return true;
            }
        }
        false
    }

    pub fn draw(&mut self, renderer: &mut Renderer, theme: &Theme, scale_f: f32) {
        let wx = self.x * scale_f;
        let wy = self.y * scale_f;
        let ww = self.w * scale_f;
        let wh = self.h * scale_f;
        let title_h = 20.0 * scale_f;

        let mut dx = (self.render_x - self.x) * scale_f;
        let mut dy = (self.render_y - self.y) * scale_f;
        let max_s = 15.0 * scale_f;
        dx = dx.clamp(-max_s, max_s);
        dy = dy.clamp(-max_s, max_s);

        unsafe {
            // Draw Title Bar
            renderer.set_color(theme.border.with_alpha(1.0));
            renderer.draw_rect(
                wx as i32 - 1,
                wy as i32 - 1,
                ww as i32 + 2,
                title_h as i32 + 2,
            );

            renderer.set_color(theme.title_bg);
            renderer.draw_rect(wx as i32, wy as i32, ww as i32, title_h as i32);

            let t_width = get_text_width(&self.title, scale_f as i32);
            let text_x = wx as i32 + (ww as i32 - t_width) / 2;
            let text_y = wy as i32 + (title_h as i32 - (7 * scale_f as i32)) / 2;

            crate::graphic::font::draw_text(
                renderer,
                &self.title,
                text_x,
                text_y,
                theme.text_accent.with_alpha(1.0),
                scale_f as i32,
            );

            // Draw Body (Veil)
            let body_top_y = wy + title_h;
            let body_h = wh - title_h;

            let tl_x = wx;
            let tl_y = body_top_y;
            let tr_x = wx + ww;
            let tr_y = body_top_y;
            let bl_x = wx + dx;
            let bl_y = body_top_y + body_h + dy;
            let br_x = wx + ww + dx;
            let br_y = body_top_y + body_h + dy;

            renderer.set_color(theme.border.with_alpha(1.0));
            renderer.draw_quad(
                tl_x - 1.0,
                tl_y,
                tr_x + 1.0,
                tr_y,
                bl_x - 1.0,
                bl_y + 1.0,
                br_x + 1.0,
                br_y + 1.0,
            );

            renderer.set_color(theme.window_bg);
            renderer.draw_quad(tl_x, tl_y, tr_x, tr_y, bl_x, bl_y, br_x, br_y);

            // Enable scissor bounding explicitly
            let sx = tl_x.min(bl_x) as i32;
            let sy = tl_y.min(bl_y) as i32;
            let sw = (ww + dx.abs()) as i32;
            let sh = (body_h + dy.max(0.0)) as i32;

            renderer.enable_scissor(sx, sy, sw, sh);
        }

        // Draw children
        for child in &mut self.children {
            child.draw(renderer, theme, scale_f);
        }

        unsafe {
            renderer.disable_scissor();
        }
    }
}
