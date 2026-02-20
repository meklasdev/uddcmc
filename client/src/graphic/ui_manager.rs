use crate::graphic::input::{GUI_OPEN, MOUSE_STATE};
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    pub static ref UI_MANAGER: Mutex<UiManager> = Mutex::new(UiManager::new());
}

pub struct WindowState {
    pub title: String,
    pub x: f32,
    pub y: f32,
    pub render_x: f32,
    pub render_y: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub width: f32,
    pub height: f32,
    pub is_dragging: bool,
    pub drag_offset_x: f32,
    pub drag_offset_y: f32,
    pub modules: Vec<String>,
}

pub struct UiManager {
    pub windows: Vec<WindowState>,
    pub background_alpha: f32,
    pub is_visible: bool,
}

impl UiManager {
    pub fn new() -> Self {
        Self {
            background_alpha: 0.0,
            is_visible: false,
            windows: vec![
                WindowState {
                    title: "Combat".to_string(),
                    x: 50.0,
                    y: 50.0,
                    render_x: 50.0,
                    render_y: 50.0,
                    vel_x: 0.0,
                    vel_y: 0.0,
                    width: 120.0,
                    height: 200.0,
                    is_dragging: false,
                    drag_offset_x: 0.0,
                    drag_offset_y: 0.0,
                    modules: vec!["MobAura".to_string(), "Criticals".to_string()],
                },
                WindowState {
                    title: "Movement".to_string(),
                    x: 200.0,
                    y: 50.0,
                    render_x: 200.0,
                    render_y: 50.0,
                    vel_x: 0.0,
                    vel_y: 0.0,
                    width: 120.0,
                    height: 200.0,
                    is_dragging: false,
                    drag_offset_x: 0.0,
                    drag_offset_y: 0.0,
                    modules: vec!["Sprint".to_string(), "Fly".to_string()],
                },
                WindowState {
                    title: "Render".to_string(),
                    x: 350.0,
                    y: 50.0,
                    render_x: 350.0,
                    render_y: 50.0,
                    vel_x: 0.0,
                    vel_y: 0.0,
                    width: 120.0,
                    height: 200.0,
                    is_dragging: false,
                    drag_offset_x: 0.0,
                    drag_offset_y: 0.0,
                    modules: vec!["ESP".to_string(), "FullBright".to_string()],
                },
            ],
        }
    }

    pub fn update(&mut self, scale_f: f32) {
        // Handle visibility and animations
        let target_visible = GUI_OPEN.load(std::sync::atomic::Ordering::Relaxed);

        if target_visible {
            self.is_visible = true;
            if self.background_alpha < 0.6 {
                self.background_alpha += 0.05; // Fade in
            }
        } else {
            if self.background_alpha > 0.0 {
                self.background_alpha -= 0.05; // Fade out
            } else {
                self.is_visible = false;
            }
        }

        if !self.is_visible {
            return;
        }

        // Handle Mouse state
        if let Ok(mouse) = MOUSE_STATE.lock() {
            let mx = mouse.x as f32;
            let my = mouse.y as f32;

            for window in &mut self.windows {
                // Spring Physics
                let stiffness = 0.25; // How strongly it pulls towards target
                let damping = 0.65; // Defines jelly bounce vs snap (lower = more bouncy)

                let fx = (window.x - window.render_x) * stiffness;
                let fy = (window.y - window.render_y) * stiffness;

                window.vel_x = (window.vel_x + fx) * damping;
                window.vel_y = (window.vel_y + fy) * damping;

                window.render_x += window.vel_x;
                window.render_y += window.vel_y;

                let scaled_x = window.render_x * scale_f;
                let scaled_y = window.render_y * scale_f;
                let scaled_w = window.width * scale_f;
                let title_height = 20.0 * scale_f;

                let is_hovering_title = mx >= scaled_x
                    && mx <= scaled_x + scaled_w
                    && my >= scaled_y
                    && my <= scaled_y + title_height;

                if mouse.left_down {
                    if is_hovering_title && !window.is_dragging {
                        window.is_dragging = true;
                        window.drag_offset_x = (mx - scaled_x) / scale_f;
                        window.drag_offset_y = (my - scaled_y) / scale_f;
                    }

                    if window.is_dragging {
                        window.x = (mx / scale_f) - window.drag_offset_x;
                        window.y = (my / scale_f) - window.drag_offset_y;
                    }
                } else {
                    window.is_dragging = false;
                }
            }
        }
    }
}
