use crate::client::DarkClient;
use crate::graphic::input::{GUI_OPEN, MOUSE_STATE};
use crate::module::ModuleCategory;
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    pub static ref UI_MANAGER: Mutex<UiManager> = Mutex::new(UiManager::new());
}

pub struct ModuleUiState {
    pub name: String,
    pub is_expanded: bool,
    pub expand_anim: f32, // 0.0 to 1.0
}

pub struct WindowState {
    pub title: String,
    pub category: ModuleCategory,
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
    pub scroll_y: f32,
    pub scroll_vel: f32,
    pub modules: Vec<ModuleUiState>,
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
                    title: ModuleCategory::COMBAT.display_name().to_string(),
                    category: ModuleCategory::COMBAT,
                    x: 50.0,
                    y: 50.0,
                    render_x: 50.0,
                    render_y: 50.0,
                    vel_x: 0.0,
                    vel_y: 0.0,
                    width: 160.0,
                    height: 200.0,
                    is_dragging: false,
                    drag_offset_x: 0.0,
                    drag_offset_y: 0.0,
                    scroll_y: 0.0,
                    scroll_vel: 0.0,
                    modules: vec![],
                },
                WindowState {
                    title: ModuleCategory::MOVEMENT.display_name().to_string(),
                    category: ModuleCategory::MOVEMENT,
                    x: 230.0,
                    y: 50.0,
                    render_x: 230.0,
                    render_y: 50.0,
                    vel_x: 0.0,
                    vel_y: 0.0,
                    width: 160.0,
                    height: 200.0,
                    is_dragging: false,
                    drag_offset_x: 0.0,
                    drag_offset_y: 0.0,
                    scroll_y: 0.0,
                    scroll_vel: 0.0,
                    modules: vec![],
                },
                WindowState {
                    title: ModuleCategory::RENDER.display_name().to_string(),
                    category: ModuleCategory::RENDER,
                    x: 410.0,
                    y: 50.0,
                    render_x: 410.0,
                    render_y: 50.0,
                    vel_x: 0.0,
                    vel_y: 0.0,
                    width: 160.0,
                    height: 200.0,
                    is_dragging: false,
                    drag_offset_x: 0.0,
                    drag_offset_y: 0.0,
                    scroll_y: 0.0,
                    scroll_vel: 0.0,
                    modules: vec![],
                },
                WindowState {
                    title: ModuleCategory::PLAYER.display_name().to_string(),
                    category: ModuleCategory::PLAYER,
                    x: 590.0,
                    y: 50.0,
                    render_x: 590.0,
                    render_y: 50.0,
                    vel_x: 0.0,
                    vel_y: 0.0,
                    width: 160.0,
                    height: 200.0,
                    is_dragging: false,
                    drag_offset_x: 0.0,
                    drag_offset_y: 0.0,
                    scroll_y: 0.0,
                    scroll_vel: 0.0,
                    modules: vec![],
                },
                WindowState {
                    title: ModuleCategory::WORLD.display_name().to_string(),
                    category: ModuleCategory::WORLD,
                    x: 770.0,
                    y: 50.0,
                    render_x: 770.0,
                    render_y: 50.0,
                    vel_x: 0.0,
                    vel_y: 0.0,
                    width: 160.0,
                    height: 200.0,
                    is_dragging: false,
                    drag_offset_x: 0.0,
                    drag_offset_y: 0.0,
                    scroll_y: 0.0,
                    scroll_vel: 0.0,
                    modules: vec![],
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

        // Sync real modules from DarkClient if empty
        let client_modules_guard = DarkClient::instance().modules.read().unwrap();
        for window in &mut self.windows {
            if window.modules.is_empty() {
                let mut valid_modules: Vec<_> = client_modules_guard
                    .values()
                    .filter(|m| m.lock().unwrap().get_module_data().category == window.category)
                    .collect();

                valid_modules.sort_by(|a, b| {
                    a.lock()
                        .unwrap()
                        .get_module_data()
                        .name
                        .cmp(&b.lock().unwrap().get_module_data().name)
                });

                window.modules = valid_modules
                    .into_iter()
                    .map(|m| ModuleUiState {
                        name: m.lock().unwrap().get_module_data().name.clone(),
                        is_expanded: false,
                        expand_anim: 0.0,
                    })
                    .collect();
            }

            // Animate expansions
            for m_state in &mut window.modules {
                if m_state.is_expanded {
                    if m_state.expand_anim < 1.0 {
                        m_state.expand_anim += 0.1;
                    }
                } else {
                    if m_state.expand_anim > 0.0 {
                        m_state.expand_anim -= 0.1;
                    }
                }
                m_state.expand_anim = m_state.expand_anim.clamp(0.0, 1.0);
            }
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

                // Dynamic window height expansion
                let mut target_h = 20.0 + 5.0; // Title bar + padding
                for m_state in &window.modules {
                    target_h += 16.0; // Base module height

                    if m_state.expand_anim > 0.01 {
                        if let Some(m) = client_modules_guard.get(&m_state.name) {
                            let lock = m.lock().unwrap();
                            let settings_count = lock.get_module_data().settings.len() as f32;
                            let expanded_h = settings_count * 14.0;
                            target_h += m_state.expand_anim * expanded_h;
                        }
                    }
                    target_h += 2.0; // Gap between modules
                }
                target_h += 5.0; // Bottom padding

                // Max limits and scrolling
                let max_h = 300.0; // Cap to arbitrary viewport size before scrolling
                if target_h > max_h {
                    target_h = max_h;
                    // todo scrolling bounds
                }

                // Lerp window height
                window.height += (target_h - window.height) * 0.25;
            }
        }
    }
}
