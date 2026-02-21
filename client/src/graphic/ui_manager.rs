use crate::cleanup_client;
use crate::client::DarkClient;
use crate::graphic::color::Rgba;
use crate::graphic::input::{GUI_OPEN, MOUSE_STATE};
use crate::graphic::widget::{Button, HudWidget, ModuleButton, Widget, Window};
use crate::module::ModuleCategory;
use std::sync::Mutex;

lazy_static::lazy_static! {
    pub static ref UI_MANAGER: Mutex<UiManager> = Mutex::new(UiManager::new());
}

pub struct UiManager {
    pub windows: Vec<Widget>,
    pub reset_btn: Button,
    pub panic_btn: Button,
    pub hud_overlay: HudWidget,
    pub background_alpha: f32,
    pub is_visible: bool,
    pub initialized_layout: bool,
}

impl UiManager {
    pub fn new() -> Self {
        let combat_win = Window::new(
            50.0,
            50.0,
            160.0,
            20.0,
            ModuleCategory::COMBAT.display_name(),
        );
        let move_win = Window::new(
            230.0,
            50.0,
            160.0,
            20.0,
            ModuleCategory::MOVEMENT.display_name(),
        );
        let render_win = Window::new(
            410.0,
            50.0,
            160.0,
            20.0,
            ModuleCategory::RENDER.display_name(),
        );
        let player_win = Window::new(
            590.0,
            50.0,
            160.0,
            20.0,
            ModuleCategory::PLAYER.display_name(),
        );
        let world_win = Window::new(
            770.0,
            50.0,
            160.0,
            20.0,
            ModuleCategory::WORLD.display_name(),
        );

        let reset_btn = Button::new(0.0, 0.0, 60.0, 20.0, "Reset UI")
            .with_bg_color(Rgba::new_rgb(0.2, 0.2, 0.2, 0.8))
            .with_text_color(Rgba::new_rgb(1.0, 1.0, 1.0, 1.0));

        let panic_btn = Button::new(0.0, 0.0, 60.0, 20.0, "PANIC")
            .with_bg_color(Rgba::new_rgb(0.8, 0.2, 0.2, 0.8))
            .with_text_color(Rgba::new_rgb(1.0, 1.0, 1.0, 1.0));

        let manager = Self {
            background_alpha: 0.0,
            is_visible: false,
            initialized_layout: false,
            hud_overlay: HudWidget::new(),
            reset_btn,
            panic_btn,
            windows: vec![
                Widget::Window(combat_win),
                Widget::Window(move_win),
                Widget::Window(render_win),
                Widget::Window(player_win),
                Widget::Window(world_win),
            ],
        };
        manager
    }

    pub fn reset_ui(&mut self, screen_w: f32, screen_h: f32) {
        self.auto_wrap_windows(screen_w, screen_h);
    }

    pub fn auto_wrap_windows(&mut self, screen_w: f32, _screen_h: f32) {
        let start_x = 50.0;
        let start_y = 50.0;
        let gap_x = 20.0;
        let gap_y = 20.0;

        let mut current_x = start_x;
        let mut current_y = start_y;
        let mut row_max_height = 0.0_f32;

        for w_widget in &mut self.windows {
            if let Widget::Window(window) = w_widget {
                let mut calc_h = 25.0;
                for child in &window.children {
                    match child {
                        Widget::Button(b) => calc_h += b.h + 2.0,
                        Widget::Label(_) => calc_h += 16.0,
                        Widget::ModuleButton(m) => calc_h += m.h + 2.0,
                        _ => {}
                    }
                }

                if current_x + window.w > screen_w && current_x > start_x {
                    // Wrap to next line
                    current_x = start_x;
                    current_y += row_max_height + gap_y;
                    row_max_height = 0.0;
                }

                window.x = current_x;
                window.y = current_y;
                window.render_x = current_x;
                window.render_y = current_y;

                current_x += window.w + gap_x;
                if calc_h > row_max_height {
                    row_max_height = calc_h;
                }
            }
        }
    }

    pub fn update(&mut self, scale_f: f32, screen_w: f32, screen_h: f32) {
        // Sync real modules from DarkClient if empty (only triggers once)
        for w_widget in &mut self.windows {
            if let Widget::Window(window) = w_widget {
                if window.children.is_empty() {
                    let client_modules_guard = DarkClient::instance().modules.read().unwrap();
                    let target_category = match window.title.as_str() {
                        "Combat" => ModuleCategory::COMBAT,
                        "Movement" => ModuleCategory::MOVEMENT,
                        "Render" => ModuleCategory::RENDER,
                        "Player" => ModuleCategory::PLAYER,
                        "World" => ModuleCategory::WORLD,
                        _ => ModuleCategory::COMBAT,
                    };

                    let mut valid_modules: Vec<_> = client_modules_guard
                        .values()
                        .filter(|m| m.lock().unwrap().get_module_data().category == target_category)
                        .collect();

                    valid_modules.sort_by(|a, b| {
                        a.lock()
                            .unwrap()
                            .get_module_data()
                            .name
                            .cmp(&b.lock().unwrap().get_module_data().name)
                    });

                    window.children = valid_modules
                        .into_iter()
                        .map(|m| {
                            let name = m.lock().unwrap().get_module_data().name.clone();
                            Widget::ModuleButton(ModuleButton::new(&name))
                        })
                        .collect();
                }
            }
        }

        if !self.initialized_layout {
            self.auto_wrap_windows(screen_w, screen_h);
            self.initialized_layout = true;
        }

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

        // Handle interactions & logic updates
        let (mx, my, left_clicked, right_clicked, left_down) = {
            if let Ok(mut mouse) = MOUSE_STATE.lock() {
                let l = mouse.left_clicked;
                let r = mouse.right_clicked;
                let ld = mouse.left_down;
                mouse.left_clicked = false;
                mouse.right_clicked = false;
                (mouse.x as f32, mouse.y as f32, l, r, ld)
            } else {
                (0.0, 0.0, false, false, false)
            }
        };

        // Delegate clicks
        if left_clicked || right_clicked {
            let mut clicked_idx = None;
            for (i, widget) in self.windows.iter_mut().enumerate().rev() {
                if let Widget::Window(win) = widget {
                    let scaled_x = win.render_x * scale_f;
                    let scaled_y = win.render_y * scale_f;
                    let scaled_w = win.w * scale_f;
                    let scaled_h = win.h * scale_f;

                    if mx >= scaled_x
                        && mx <= scaled_x + scaled_w
                        && my >= scaled_y
                        && my <= scaled_y + scaled_h
                    {
                        clicked_idx = Some(i);
                        break;
                    }
                }
            }

            if let Some(idx) = clicked_idx {
                let mut top_win = self.windows.remove(idx);
                top_win.handle_click(mx, my, left_clicked, right_clicked, scale_f);
                self.windows.push(top_win);
            }
        }

        // Delegate updates
        for widget in &mut self.windows {
            widget.update(mx, my, left_down, scale_f);
        }

        // Layout Context Buttons
        let btn_w = 60.0;
        let btn_h = 20.0;
        let btn_pad = 10.0;

        self.reset_btn.x = (screen_w / scale_f) - btn_w - btn_pad;
        self.reset_btn.y = btn_pad;
        self.reset_btn.w = btn_w;
        self.reset_btn.h = btn_h;

        self.panic_btn.x = self.reset_btn.x - btn_w - btn_pad;
        self.panic_btn.y = btn_pad;
        self.panic_btn.w = btn_w;
        self.panic_btn.h = btn_h;

        // Apply alpha to context buttons dynamically
        let shared_alpha = self.background_alpha.min(0.8);
        self.reset_btn.bg_color.a = shared_alpha;
        self.panic_btn.bg_color.a = shared_alpha;

        self.reset_btn.update(mx, my, left_down, scale_f);
        self.panic_btn.update(mx, my, left_down, scale_f);

        if left_clicked {
            if self
                .reset_btn
                .handle_click(mx, my, left_clicked, right_clicked, scale_f)
            {
                self.reset_ui(screen_w, screen_h);
            }
            if self
                .panic_btn
                .handle_click(mx, my, left_clicked, right_clicked, scale_f)
            {
                std::thread::spawn(|| call_panic());
            }
        }
    }
}

pub fn call_panic() {
    let client = DarkClient::instance();
    client.modules.read().unwrap().values().for_each(|module| {
        let mut module = module.lock().unwrap();
        if module.get_module_data().enabled {
            module.get_module_data_mut().set_enabled(false);
            match module.on_stop() {
                Ok(_) => {}
                Err(e) => {
                    log::error!(
                        "Failed to stop module {} on panic: {}",
                        module.get_module_data().name,
                        e
                    );
                }
            }
        }
    });
    cleanup_client();
}
