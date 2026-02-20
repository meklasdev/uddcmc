use crate::client::DarkClient;
use crate::graphic::font::{draw_text, get_text_width};
use crate::graphic::input::MOUSE_STATE;
use crate::graphic::render::Renderer;
use crate::graphic::ui_manager::UI_MANAGER;

// --- GUI CONFIGURATION CONSTANTS ---
pub const GUI_TITLE_HEIGHT: f32 = 20.0;
pub const GUI_MODULE_HEIGHT: f32 = 16.0;
pub const GUI_SETTING_HEIGHT: f32 = 14.0;
pub const GUI_PADDING: f32 = 5.0;
pub const GUI_MAX_STRETCH: f32 = 30.0;

// Text Scales
pub const TEXT_SCALE_HUD_WATERMARK: f32 = 2.0;
pub const TEXT_SCALE_HUD_MODULES: f32 = 1.4;

// Layout spacing
pub const HUD_PADDING_Y: f32 = 5.0;
pub const HUD_PADDING_X: f32 = 5.0;
pub const HUD_WATERMARK_MARGIN_BOTTOM: f32 = 24.0;
pub const HUD_MODULES_SPACING: f32 = 16.0;

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

pub enum HudColor {
    Yellow,
    Green,
    Red,
    Blue,
    White,
    Purple,
    Cyan,
}

impl HudColor {
    pub fn to_rgba(&self) -> (f32, f32, f32, f32) {
        match self {
            HudColor::Yellow => (1.0, 1.0, 0.0, 1.0),
            HudColor::Green => (0.2, 0.8, 0.2, 1.0),
            HudColor::Red => (0.9, 0.2, 0.2, 1.0),
            HudColor::Blue => (0.2, 0.4, 1.0, 1.0),
            HudColor::White => (1.0, 1.0, 1.0, 1.0),
            HudColor::Purple => (0.6, 0.2, 0.8, 1.0),
            HudColor::Cyan => (0.2, 0.8, 0.9, 1.0),
        }
    }
}

unsafe fn draw_hud(renderer: &mut Renderer, scale_f: f32) {
    let watermark = "DarkClient";
    let w_color = HudColor::Yellow.to_rgba();

    let mut hud_y = HUD_PADDING_Y * scale_f;
    let hud_x = HUD_PADDING_X * scale_f;
    let text_scale = (TEXT_SCALE_HUD_WATERMARK * scale_f) as i32;

    draw_text(
        renderer,
        watermark,
        hud_x as i32,
        hud_y as i32,
        w_color.0,
        w_color.1,
        w_color.2,
        w_color.3,
        text_scale,
    );

    hud_y += HUD_WATERMARK_MARGIN_BOTTOM * scale_f;

    if let Ok(modules_map) = DarkClient::instance().modules.read() {
        let mut active_mods: Vec<String> = modules_map
            .values()
            .filter_map(|m| {
                let lock = m.lock().unwrap();
                if lock.get_module_data().enabled {
                    Some(lock.get_module_data().name.clone())
                } else {
                    None
                }
            })
            .collect();

        // Sort by length (longest first)
        active_mods.sort_by(|a, b| b.len().cmp(&a.len()));

        let arraylist_colors = [
            HudColor::Purple,
            HudColor::Cyan,
            HudColor::Green,
            HudColor::Red,
            HudColor::Yellow,
            HudColor::Blue,
            HudColor::White,
        ];

        for (i, mod_name) in active_mods.iter().enumerate() {
            let color = arraylist_colors[i % arraylist_colors.len()].to_rgba();
            draw_text(
                renderer,
                mod_name,
                hud_x as i32,
                hud_y as i32,
                color.0,
                color.1,
                color.2,
                color.3,
                (TEXT_SCALE_HUD_MODULES * scale_f) as i32,
            );
            hud_y += HUD_MODULES_SPACING * scale_f;
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

    ui.update(scale_f, screen_w as f32, screen_h as f32);

    unsafe {
        draw_hud(renderer, scale_f);
    }

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
        renderer.draw_rect(0, 0, screen_w as i32, screen_h as i32);

        // Retrieve mouse state for bounds interactions
        let (mx, my, left_clicked, right_clicked) = {
            let m = MOUSE_STATE.lock().unwrap();
            (m.x as f32, m.y as f32, m.left_clicked, m.right_clicked)
        };

        let windows_len = ui.windows.len();
        for window_index in 0..windows_len {
            // Rigid Title Position
            let (wx, wy, ww, wh) = {
                let window = &mut ui.windows[window_index];

                let wx = (window.x * scale_f) as f32;
                let wy = (window.y * scale_f) as f32;
                let ww = (window.width * scale_f) as f32;
                let wh = (window.height * scale_f) as f32;
                (wx, wy, ww, wh)
            };
            let title_h = GUI_TITLE_HEIGHT * scale_f;

            let mut is_topmost = true;
            for i in (window_index + 1)..windows_len {
                let higher_w = &ui.windows[i];
                let hwx = higher_w.render_x * scale_f;
                let hwy = higher_w.render_y * scale_f;
                let hww = higher_w.width * scale_f;
                let hwh = higher_w.height * scale_f;
                if mx >= hwx && mx <= hwx + hww && my >= hwy && my <= hwy + hwh {
                    is_topmost = false;
                    break;
                }
            }

            let window = &mut ui.windows[window_index];

            // Veil trailing offset
            let mut dx = (window.render_x - window.x) * scale_f;
            let mut dy = (window.render_y - window.y) * scale_f;

            // Cap the stretch visual effect
            let max_stretch = GUI_MAX_STRETCH * scale_f;
            dx = dx.clamp(-max_stretch, max_stretch);
            dy = dy.clamp(-max_stretch, max_stretch);

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
            let mut mod_y = body_top_y + (GUI_PADDING * scale_f);
            for module_state in &mut window.modules {
                let module_name = &module_state.name;
                let mod_act_h = GUI_MODULE_HEIGHT * scale_f; // base height
                let mod_w = ww - (GUI_PADDING * 2.0 * scale_f);
                let mod_x = wx + (GUI_PADDING * scale_f);

                let box_side = mod_act_h;
                let box_x = mod_x + mod_w - box_side;
                let box_y = mod_y;

                // Check interactions
                let is_hovering = is_topmost
                    && mx >= mod_x
                    && mx <= mod_x + mod_w
                    && my >= mod_y
                    && my <= mod_y + mod_act_h;

                let is_box_hovering = is_topmost
                    && mx >= box_x
                    && mx <= box_x + box_side
                    && my >= box_y
                    && my <= box_y + box_side;

                let mut is_enabled = false;
                if let Some(m) = DarkClient::instance()
                    .modules
                    .read()
                    .unwrap()
                    .get(module_name)
                {
                    let mut lock = m.lock().unwrap();
                    is_enabled = lock.get_module_data().enabled;

                    if is_hovering {
                        if left_clicked {
                            if is_box_hovering {
                                module_state.is_expanded = !module_state.is_expanded;
                            } else {
                                lock.get_module_data_mut().set_enabled(!is_enabled);
                                if !is_enabled {
                                    let _ = lock.on_start();
                                } else {
                                    let _ = lock.on_stop();
                                }
                                is_enabled = !is_enabled;
                            }
                        }
                        if right_clicked {
                            module_state.is_expanded = !module_state.is_expanded;
                        }
                    }
                }

                // Calculate visual height including expansions
                let mut expanded_height = 0.0;
                if module_state.expand_anim > 0.01 {
                    if let Some(m) = DarkClient::instance()
                        .modules
                        .read()
                        .unwrap()
                        .get(module_name)
                    {
                        let lock = m.lock().unwrap();
                        let settings_count = lock.get_module_data().settings.len() as f32;
                        expanded_height = settings_count * 14.0 * scale_f;
                    }
                }

                let mod_total_visual_h = mod_act_h + (module_state.expand_anim * expanded_height);

                // Interpolate quad stretch
                let t_top = ((mod_y - body_top_y) / body_h).clamp(0.0, 1.0);
                let t_bot = ((mod_y + mod_total_visual_h - body_top_y) / body_h).clamp(0.0, 1.0);

                let mtl_x = mod_x + dx * t_top;
                let mtl_y = mod_y + dy * t_top;
                let mtr_x = mod_x + mod_w + dx * t_top;
                let mtr_y = mod_y + dy * t_top;

                let mbl_x = mod_x + dx * t_bot;
                let mbl_y = mod_y + mod_total_visual_h + dy * t_bot;
                let mbr_x = mod_x + mod_w + dx * t_bot;
                let mbr_y = mod_y + mod_total_visual_h + dy * t_bot;

                let bg_color = if is_hovering {
                    theme.module_bg_hover
                } else {
                    theme.module_bg
                };

                renderer.set_color(bg_color.0, bg_color.1, bg_color.2, bg_color.3 * alpha);
                renderer.draw_quad(mtl_x, mtl_y, mtr_x, mtr_y, mbl_x, mbl_y, mbr_x, mbr_y);

                // Module text
                let text_t = ((mod_y + mod_act_h / 2.0 - body_top_y) / body_h).clamp(0.0, 1.0);
                let t_dx = dx * text_t;
                let t_dy = dy * text_t;

                let mod_t_width = get_text_width(module_name, text_scale);
                let mod_t_x = mod_x + t_dx + (mod_w - mod_t_width as f32) / 2.0;
                let mod_t_y = mod_y + t_dy + (mod_act_h - (7.0 * scale_f)) / 2.0;

                let text_color = if is_enabled {
                    theme.text_accent
                } else {
                    theme.text_primary
                };

                draw_text(
                    renderer,
                    module_name,
                    mod_t_x as i32,
                    mod_t_y as i32,
                    text_color.0,
                    text_color.1,
                    text_color.2,
                    alpha,
                    text_scale,
                );

                // Draw arrow box
                let t_dx_box = dx * text_t;
                let t_dy_box = dy * text_t;
                let actual_box_x = box_x + t_dx_box;
                let actual_box_y = box_y + t_dy_box;

                let box_bg = if is_box_hovering {
                    theme.module_bg_hover
                } else {
                    theme.module_bg
                };

                // Border separator
                renderer.set_color(theme.border.0, theme.border.1, theme.border.2, alpha * 0.5);
                renderer.draw_rect(
                    actual_box_x as i32 - 1,
                    actual_box_y as i32,
                    box_side as i32 + 1,
                    box_side as i32,
                );

                renderer.set_color(box_bg.0, box_bg.1, box_bg.2, box_bg.3 * alpha);
                renderer.draw_rect(
                    actual_box_x as i32,
                    actual_box_y as i32,
                    box_side as i32,
                    box_side as i32,
                );

                let arrow_char = if module_state.is_expanded { "^" } else { "v" };
                let arrow_w = get_text_width(arrow_char, text_scale);
                draw_text(
                    renderer,
                    arrow_char,
                    (actual_box_x + (box_side - arrow_w as f32) / 2.0) as i32,
                    (actual_box_y + (box_side - 7.0 * scale_f) / 2.0) as i32,
                    theme.text_primary.0,
                    theme.text_primary.1,
                    theme.text_primary.2,
                    alpha,
                    text_scale,
                );

                // Draw settings
                if module_state.expand_anim > 0.01 {
                    if let Some(m) = DarkClient::instance()
                        .modules
                        .read()
                        .unwrap()
                        .get(module_name)
                    {
                        let mut lock = m.lock().unwrap();
                        let data = lock.get_module_data_mut();

                        let mut set_y = mod_y + mod_act_h;

                        // Let's get mouse dragged state
                        let left_down = if let Ok(mouse) = MOUSE_STATE.lock() {
                            mouse.left_down
                        } else {
                            false
                        };

                        for setting in &mut data.settings {
                            let set_h = 14.0 * scale_f;
                            let text_t =
                                ((set_y + set_h / 2.0 - body_top_y) / body_h).clamp(0.0, 1.0);
                            let t_dx = dx * text_t;
                            let t_dy = dy * text_t;

                            let set_x = mod_x + (10.0 * scale_f); // Indent
                            let set_w = mod_w - (10.0 * scale_f);

                            // Calculate skewed positions for interaction
                            let actual_sx = set_x + t_dx;
                            let actual_sy = set_y + t_dy;

                            let is_set_hovering = is_topmost
                                && mx >= actual_sx
                                && mx <= actual_sx + set_w
                                && my >= actual_sy
                                && my <= actual_sy + set_h;

                            let set_alpha = alpha * module_state.expand_anim;

                            match setting {
                                crate::module::ModuleSetting::Toggle { name, value } => {
                                    if is_set_hovering && left_clicked {
                                        *value = !*value;
                                    }

                                    let t_color = if *value {
                                        theme.text_accent
                                    } else {
                                        theme.text_primary
                                    };

                                    draw_text(
                                        renderer,
                                        &format!("{}: {}", name, if *value { "On" } else { "Off" }),
                                        actual_sx as i32,
                                        (actual_sy + (set_h - 7.0 * scale_f) / 2.0) as i32,
                                        t_color.0,
                                        t_color.1,
                                        t_color.2,
                                        set_alpha,
                                        text_scale,
                                    );
                                }
                                crate::module::ModuleSetting::Slider {
                                    name,
                                    value,
                                    min,
                                    max,
                                } => {
                                    let slider_w = set_w - (60.0 * scale_f);
                                    let slider_x = actual_sx + (50.0 * scale_f);

                                    if is_set_hovering && left_down {
                                        let relative_x = (mx - slider_x).clamp(0.0, slider_w);
                                        let factor = relative_x / slider_w;
                                        *value = *min + factor * (*max - *min);
                                        // Optional: snap or round value here
                                    }

                                    draw_text(
                                        renderer,
                                        &format!("{}: {:.1}", name, value),
                                        actual_sx as i32,
                                        (actual_sy + (set_h - 7.0 * scale_f) / 2.0) as i32,
                                        theme.text_primary.0,
                                        theme.text_primary.1,
                                        theme.text_primary.2,
                                        set_alpha,
                                        text_scale,
                                    );

                                    // Draw thin slider line
                                    renderer.set_color(
                                        theme.text_primary.0,
                                        theme.text_primary.1,
                                        theme.text_primary.2,
                                        set_alpha * 0.5,
                                    );
                                    renderer.draw_rect(
                                        slider_x as i32,
                                        (actual_sy + set_h / 2.0) as i32,
                                        slider_w as i32,
                                        (2.0 * scale_f) as i32,
                                    );

                                    // Draw handle
                                    let handle_x =
                                        slider_x + ((*value - *min) / (*max - *min)) * slider_w;
                                    renderer.set_color(
                                        theme.text_accent.0,
                                        theme.text_accent.1,
                                        theme.text_accent.2,
                                        set_alpha,
                                    );
                                    renderer.draw_rect(
                                        (handle_x - 2.0 * scale_f) as i32,
                                        (actual_sy + set_h / 2.0 - 2.0 * scale_f) as i32,
                                        (4.0 * scale_f) as i32,
                                        (6.0 * scale_f) as i32,
                                    );
                                }
                                crate::module::ModuleSetting::Choice {
                                    name,
                                    value,
                                    options,
                                } => {
                                    if is_set_hovering && left_clicked {
                                        *value = (*value + 1) % options.len();
                                    }
                                    let current_opt = if *value < options.len() {
                                        &options[*value]
                                    } else {
                                        "Unknown"
                                    };

                                    draw_text(
                                        renderer,
                                        &format!("{}: {}", name, current_opt),
                                        actual_sx as i32,
                                        (actual_sy + (set_h - 7.0 * scale_f) / 2.0) as i32,
                                        theme.text_primary.0,
                                        theme.text_primary.1,
                                        theme.text_primary.2,
                                        set_alpha,
                                        text_scale,
                                    );
                                }
                                crate::module::ModuleSetting::Color { name, .. } => {
                                    draw_text(
                                        renderer,
                                        &format!("{}: [Color]", name),
                                        actual_sx as i32,
                                        (actual_sy + (set_h - 7.0 * scale_f) / 2.0) as i32,
                                        theme.text_primary.0,
                                        theme.text_primary.1,
                                        theme.text_primary.2,
                                        set_alpha,
                                        text_scale,
                                    );
                                }
                            }

                            set_y += set_h;
                        }
                    }
                }

                mod_y += mod_total_visual_h + (2.0 * scale_f);
            }
        }

        // DRAW CONTEXT BUTTONS (Top Right)
        let btn_w = 60.0 * scale_f;
        let btn_h = 20.0 * scale_f;
        let btn_pad = 10.0 * scale_f;

        let reset_x = screen_w as f32 - btn_w - btn_pad;
        let reset_y = btn_pad;
        let panic_x = reset_x - btn_w - btn_pad;
        let panic_y = btn_pad;

        // Reset UI Button
        renderer.set_color(
            theme.module_bg.0,
            theme.module_bg.1,
            theme.module_bg.2,
            ui.background_alpha.min(0.8),
        );
        renderer.draw_rect(reset_x as i32, reset_y as i32, btn_w as i32, btn_h as i32);
        draw_text(
            renderer,
            "Reset UI",
            (reset_x + 5.0 * scale_f) as i32,
            (reset_y + 3.0 * scale_f) as i32,
            theme.text_primary.0,
            theme.text_primary.1,
            theme.text_primary.2,
            theme.text_primary.3,
            base_scale,
        );

        // Panic Button
        renderer.set_color(0.8, 0.2, 0.2, ui.background_alpha.min(0.8));
        renderer.draw_rect(panic_x as i32, panic_y as i32, btn_w as i32, btn_h as i32);
        draw_text(
            renderer,
            "PANIC",
            (panic_x + 12.0 * scale_f) as i32,
            (panic_y + 3.0 * scale_f) as i32,
            1.0,
            1.0,
            1.0,
            1.0,
            base_scale,
        );

        // Check clicks on buttons
        let is_reset_hovering =
            mx >= reset_x && mx <= reset_x + btn_w && my >= reset_y && my <= reset_y + btn_h;
        let is_panic_hovering =
            mx >= panic_x && mx <= panic_x + btn_w && my >= panic_y && my <= panic_y + btn_h;

        if left_clicked {
            if is_reset_hovering {
                ui.reset_ui(screen_w as f32, screen_h as f32);
            } else if is_panic_hovering {
                std::thread::spawn(|| crate::gui::call_panic());
            }
        }
    }

    // Consume clicks globally at end of frame
    if let Ok(mut mouse) = MOUSE_STATE.lock() {
        if mouse.left_clicked {
            mouse.left_clicked = false;
        }
        if mouse.right_clicked {
            mouse.right_clicked = false;
        }
    }
}
