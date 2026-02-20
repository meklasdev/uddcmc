use crate::graphic::font::{draw_text, get_text_width};
use crate::graphic::render::Renderer;
use crate::graphic::ui_manager::UI_MANAGER;

/// Defines the colors and sizes for the custom DarkClient GUI.
pub struct Theme {
    pub screen_bg: (f32, f32, f32, f32),
    pub window_bg: (f32, f32, f32, f32),
    pub title_bg: (f32, f32, f32, f32),
    pub border: (f32, f32, f32, f32),
    pub text_primary: (f32, f32, f32, f32),
    pub text_accent: (f32, f32, f32, f32),
    pub module_bg: (f32, f32, f32, f32),
    pub module_bg_hover: (f32, f32, f32, f32),
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Full screen dim overlay
            screen_bg: (0.0, 0.0, 0.0, 0.5),
            // Window background
            window_bg: (0.1, 0.1, 0.1, 0.95),
            // Title bar
            title_bg: (0.05, 0.05, 0.05, 1.0),
            // Accent border
            border: (0.5, 0.0, 1.0, 1.0),
            // Texts
            text_primary: (1.0, 1.0, 1.0, 1.0),
            text_accent: (1.0, 1.0, 0.0, 1.0),
            // Module rects
            module_bg: (0.15, 0.15, 0.15, 1.0),
            module_bg_hover: (0.25, 0.25, 0.25, 1.0),
        }
    }
}

/// Represents the main DarkClient GUI window and renders it.
pub fn render_gui(renderer: &mut Renderer) {
    let mut ui = match UI_MANAGER.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    let screen_w = renderer.screen_width;
    let screen_h = renderer.screen_height;

    let base_scale = (screen_h as f32 / 720.0).max(1.0).floor() as i32;
    let scale_f = base_scale as f32;

    ui.update(scale_f);

    if !ui.is_visible {
        return;
    }

    let alpha = ui.background_alpha;
    let theme = Theme::default();

    unsafe {
        // 1. Draw Full Screen Transparent Overlay
        renderer.set_color(
            theme.screen_bg.0,
            theme.screen_bg.1,
            theme.screen_bg.2,
            alpha,
        );
        renderer.draw_rect(0, 0, screen_w, screen_h);

        // Render each window
        for window in &ui.windows {
            // Rigid Title Position
            let wx = (window.x * scale_f) as f32;
            let wy = (window.y * scale_f) as f32;
            let ww = (window.width * scale_f) as f32;
            let wh = (window.height * scale_f) as f32;
            let title_h = 20.0 * scale_f;

            // Veil trailing offset
            let dx = (window.render_x - window.x) * scale_f;
            let dy = (window.render_y - window.y) * scale_f;

            // --- Draw Title Bar ---
            renderer.set_color(theme.border.0, theme.border.1, theme.border.2, alpha);
            renderer.draw_rect(
                wx as i32 - 1,
                wy as i32 - 1,
                ww as i32 + 2,
                title_h as i32 + 2,
            );

            renderer.set_color(
                theme.title_bg.0,
                theme.title_bg.1,
                theme.title_bg.2,
                theme.title_bg.3 * alpha,
            );
            renderer.draw_rect(wx as i32, wy as i32, ww as i32, title_h as i32);

            let text_scale = base_scale;
            let t_width = get_text_width(&window.title, text_scale);
            let text_x = wx as i32 + (ww as i32 - t_width) / 2;
            let text_y = wy as i32 + (title_h as i32 - (7 * text_scale)) / 2;

            draw_text(
                renderer,
                &window.title,
                text_x,
                text_y,
                theme.text_accent.0,
                theme.text_accent.1,
                theme.text_accent.2,
                alpha,
                text_scale,
            );

            // --- Draw Body (Veil) ---
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

            // Body Border (drawn slightly larger behind)
            renderer.set_color(theme.border.0, theme.border.1, theme.border.2, alpha);
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

            // Body Background
            renderer.set_color(
                theme.window_bg.0,
                theme.window_bg.1,
                theme.window_bg.2,
                theme.window_bg.3 * alpha,
            );
            renderer.draw_quad(tl_x, tl_y, tr_x, tr_y, bl_x, bl_y, br_x, br_y);

            // Draw Modules inside Veil
            let mut mod_y = body_top_y + (5.0 * scale_f);
            for module_name in &window.modules {
                let mod_h = 16.0 * scale_f;
                let mod_w = ww - (10.0 * scale_f);
                let mod_x = wx + (5.0 * scale_f);

                // Interpolate quad stretch
                let t_top = ((mod_y - body_top_y) / body_h).clamp(0.0, 1.0);
                let t_bot = ((mod_y + mod_h - body_top_y) / body_h).clamp(0.0, 1.0);

                let mtl_x = mod_x + dx * t_top;
                let mtl_y = mod_y + dy * t_top;
                let mtr_x = mod_x + mod_w + dx * t_top;
                let mtr_y = mod_y + dy * t_top;

                let mbl_x = mod_x + dx * t_bot;
                let mbl_y = mod_y + mod_h + dy * t_bot;
                let mbr_x = mod_x + mod_w + dx * t_bot;
                let mbr_y = mod_y + mod_h + dy * t_bot;

                renderer.set_color(
                    theme.module_bg.0,
                    theme.module_bg.1,
                    theme.module_bg.2,
                    theme.module_bg.3 * alpha,
                );
                renderer.draw_quad(mtl_x, mtl_y, mtr_x, mtr_y, mbl_x, mbl_y, mbr_x, mbr_y);

                // Module text
                let text_t = ((mod_y + mod_h / 2.0 - body_top_y) / body_h).clamp(0.0, 1.0);
                let t_dx = dx * text_t;
                let t_dy = dy * text_t;

                let mod_t_width = get_text_width(module_name, text_scale);
                let mod_t_x = mod_x + t_dx + (mod_w - mod_t_width as f32) / 2.0;
                let mod_t_y = mod_y + t_dy + (mod_h - (7.0 * scale_f)) / 2.0;

                draw_text(
                    renderer,
                    module_name,
                    mod_t_x as i32,
                    mod_t_y as i32,
                    theme.text_primary.0,
                    theme.text_primary.1,
                    theme.text_primary.2,
                    alpha,
                    text_scale,
                );

                mod_y += mod_h + (2.0 * scale_f);
            }
        }
    }
}
