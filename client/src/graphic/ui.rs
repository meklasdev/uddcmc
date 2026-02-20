use crate::graphic::color::{Rgb, Rgba};
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
    pub screen_bg: Rgba,
    pub window_bg: Rgba,
    pub title_bg: Rgba,
    pub border: Rgba,
    pub text_primary: Rgba,
    pub text_accent: Rgba,
    pub module_bg: Rgba,
    pub module_bg_hover: Rgba,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Full screen dim overlay
            screen_bg: Rgba::new(Rgb::new(0.0, 0.0, 0.0), 0.5),
            // Window background
            window_bg: Rgba::new(Rgb::new(0.1, 0.1, 0.1), 0.95),
            // Title bar
            title_bg: Rgba::new(Rgb::new(0.05, 0.05, 0.05), 1.0),
            // Accent border
            border: Rgba::new(Rgb::new(0.5, 0.0, 1.0), 1.0),
            // Texts
            text_primary: Rgba::new(Rgb::new(1.0, 1.0, 1.0), 1.0),
            text_accent: Rgba::new(Rgb::new(1.0, 1.0, 0.0), 1.0),
            // Module rects
            module_bg: Rgba::new(Rgb::new(0.15, 0.15, 0.15), 1.0),
            module_bg_hover: Rgba::new(Rgb::new(0.25, 0.25, 0.25), 1.0),
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
    pub fn to_rgba(&self) -> Rgba {
        match self {
            HudColor::Yellow => Rgba::new(Rgb::new(1.0, 1.0, 0.0), 1.0),
            HudColor::Green => Rgba::new(Rgb::new(0.2, 0.8, 0.2), 1.0),
            HudColor::Red => Rgba::new(Rgb::new(0.9, 0.2, 0.2), 1.0),
            HudColor::Blue => Rgba::new(Rgb::new(0.2, 0.4, 1.0), 1.0),
            HudColor::White => Rgba::new(Rgb::new(1.0, 1.0, 1.0), 1.0),
            HudColor::Purple => Rgba::new(Rgb::new(0.6, 0.2, 0.8), 1.0),
            HudColor::Cyan => Rgba::new(Rgb::new(0.2, 0.8, 0.9), 1.0),
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

    ui.hud_overlay.draw(renderer, &Theme::default(), scale_f);

    if !ui.is_visible {
        return;
    }

    let alpha = ui.background_alpha;
    let theme = Theme::default();

    unsafe {
        // 1. Draw Full Screen Transparent Overlay
        renderer.set_color(theme.screen_bg.with_alpha(alpha));
        renderer.draw_rect(0, 0, screen_w as i32, screen_h as i32);

        // Draw active windows and their nested modular dropdowns
        for widget in &mut ui.windows {
            widget.draw(renderer, &theme, scale_f);
        }

        // Draw Top Right action buttons
        ui.reset_btn.draw(renderer, &theme, scale_f);
        ui.panic_btn.draw(renderer, &theme, scale_f);
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
