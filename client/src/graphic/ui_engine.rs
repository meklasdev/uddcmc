use crate::cleanup_client;
use crate::client::DarkClient;
use crate::graphic::input::{GUI_OPEN, MOUSE_STATE};
use egui::Context;
use egui_glow::Painter;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

#[derive(Clone, Copy)]
pub struct WindowAnimState {
    pub actual_pos: egui::Pos2,
    pub velocity: egui::Vec2,
}

pub struct EguiState {
    pub ctx: Context,
    pub painter: Painter,
    pub last_left_down: bool,
    pub last_right_down: bool,
    pub last_mouse_pos: egui::Pos2,
    pub window_anim_states: HashMap<String, WindowAnimState>,
}

lazy_static! {
    pub static ref EGUI_STATE: Mutex<Option<EguiState>> = Mutex::new(None);
}

pub fn gather_egui_inputs(
    state: &mut EguiState,
    screen_width: f32,
    screen_height: f32,
    scale_factor: f32,
) -> egui::RawInput {
    let mut raw_input = egui::RawInput::default();

    let mut viewport_info = egui::ViewportInfo::default();
    viewport_info.native_pixels_per_point = Some(scale_factor);

    raw_input
        .viewports
        .insert(egui::ViewportId::ROOT, viewport_info);

    let logical_width = screen_width / scale_factor;
    let logical_height = screen_height / scale_factor;

    raw_input.screen_rect = Some(egui::Rect::from_min_max(
        egui::pos2(0.0, 0.0),
        egui::pos2(logical_width, logical_height),
    ));

    if !GUI_OPEN.load(Ordering::Relaxed) {
        return raw_input;
    }

    let mouse = MOUSE_STATE.lock().unwrap();
    let current_pos = egui::pos2(
        (mouse.x as f32) / scale_factor,
        (mouse.y as f32) / scale_factor,
    );

    if current_pos != state.last_mouse_pos {
        raw_input
            .events
            .push(egui::Event::PointerMoved(current_pos));
        state.last_mouse_pos = current_pos;
    }

    if mouse.left_down != state.last_left_down {
        raw_input.events.push(egui::Event::PointerButton {
            pos: current_pos,
            button: egui::PointerButton::Primary,
            pressed: mouse.left_down,
            modifiers: Default::default(),
        });
        state.last_left_down = mouse.left_down;
    }

    if mouse.right_down != state.last_right_down {
        raw_input.events.push(egui::Event::PointerButton {
            pos: current_pos,
            button: egui::PointerButton::Secondary,
            pressed: mouse.right_down,
            modifiers: Default::default(),
        });
        state.last_right_down = mouse.right_down;
    }

    raw_input
}

pub unsafe fn render_egui_ui() {
    let mut viewport = [0; 4];
    crate::gl::GetIntegerv(crate::gl::VIEWPORT, viewport.as_mut_ptr());
    let screen_width = viewport[2] as f32;
    let screen_height = viewport[3] as f32;

    let scale_factor = (screen_height / 720.0).max(1.0).floor();

    let mut state_guard = EGUI_STATE.lock().unwrap();

    if state_guard.is_none() {
        let gl = glow::Context::from_loader_function(|s| {
            crate::graphic::hook::get_proc_address(s) as *const _
        });
        let gl = std::sync::Arc::new(gl);
        let ctx = egui::Context::default();

        let fonts = egui::FontDefinitions::default();
        ctx.set_fonts(fonts);

        unsafe {
            crate::gl::PixelStorei(crate::gl::UNPACK_ALIGNMENT, 1);
            crate::gl::PixelStorei(crate::gl::UNPACK_ROW_LENGTH, 0);
            crate::gl::PixelStorei(crate::gl::UNPACK_SKIP_PIXELS, 0);
            crate::gl::PixelStorei(crate::gl::UNPACK_SKIP_ROWS, 0);
        }

        let painter = egui_glow::Painter::new(gl, "", None, false).unwrap();

        *state_guard = Some(EguiState {
            ctx,
            painter,
            last_left_down: false,
            last_right_down: false,
            last_mouse_pos: egui::pos2(0.0, 0.0),
            window_anim_states: HashMap::new(),
        });
    }

    let state = state_guard.as_mut().unwrap();
    let raw_input = gather_egui_inputs(state, screen_width, screen_height, scale_factor);

    let ctx = state.ctx.clone();

    let full_output = ctx.run(raw_input, |ctx| {
        crate::graphic::gui::render_all(ctx, &mut state.window_anim_states);
    });

    let clipped_primitives = state
        .ctx
        .tessellate(full_output.shapes, full_output.pixels_per_point);

    // Reset pixel unpack state that is often corrupted by Minecraft
    unsafe {
        crate::gl::ActiveTexture(crate::gl::TEXTURE0);
        crate::gl::PixelStorei(crate::gl::UNPACK_ALIGNMENT, 1);
        crate::gl::PixelStorei(crate::gl::UNPACK_ROW_LENGTH, 0);
        crate::gl::PixelStorei(crate::gl::UNPACK_SKIP_PIXELS, 0);
        crate::gl::PixelStorei(crate::gl::UNPACK_SKIP_ROWS, 0);

        crate::gl::Disable(crate::gl::CULL_FACE);
        crate::gl::Disable(crate::gl::DEPTH_TEST);
        crate::gl::Enable(crate::gl::BLEND);
    }

    state.painter.paint_and_update_textures(
        [screen_width as u32, screen_height as u32],
        full_output.pixels_per_point,
        &clipped_primitives,
        &full_output.textures_delta,
    );
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
