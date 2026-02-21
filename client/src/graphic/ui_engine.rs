use crate::cleanup_client;
use crate::client::DarkClient;
use crate::graphic::input::{GUI_OPEN, MOUSE_STATE};
use egui::Context;
use egui_glow::Painter;
use lazy_static::lazy_static;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

pub struct EguiState {
    pub ctx: Context,
    pub painter: Painter,
    pub last_left_down: bool,
    pub last_right_down: bool,
    pub last_mouse_pos: egui::Pos2,
    pub window_pos: egui::Pos2,
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
    // 1. Configura il Viewport principale con il nostro moltiplicatore di scala
    let mut viewport_info = egui::ViewportInfo::default();
    viewport_info.native_pixels_per_point = Some(scale_factor);

    // Inseriamo le info nella mappa usando l'ID di default (ROOT)
    raw_input
        .viewports
        .insert(egui::ViewportId::ROOT, viewport_info);

    // 2. Passiamo le coordinate LOGICHE (divise per la scala)
    let logical_width = screen_width / scale_factor;
    let logical_height = screen_height / scale_factor;

    raw_input.screen_rect = Some(egui::Rect::from_min_max(
        egui::pos2(0.0, 0.0),
        egui::pos2(logical_width, logical_height),
    ));

    // Se la GUI è chiusa, restituiamo input vuoti in modo che egui non interagisca
    if !GUI_OPEN.load(Ordering::Relaxed) {
        return raw_input;
    }

    let mouse = MOUSE_STATE.lock().unwrap();
    let current_pos = egui::pos2(
        (mouse.x as f32) / scale_factor,
        (mouse.y as f32) / scale_factor,
    );

    // 1. Movimento del Mouse
    if current_pos != state.last_mouse_pos {
        raw_input
            .events
            .push(egui::Event::PointerMoved(current_pos));
        state.last_mouse_pos = current_pos;
    }

    // 2. Click Sinistro (Transizione Su/Giù)
    if mouse.left_down != state.last_left_down {
        raw_input.events.push(egui::Event::PointerButton {
            pos: current_pos,
            button: egui::PointerButton::Primary,
            pressed: mouse.left_down, // true = appena premuto, false = appena rilasciato
            modifiers: Default::default(),
        });
        state.last_left_down = mouse.left_down;
    }

    // 3. Click Destro (Transizione Su/Giù)
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

    // 1. Inizializzazione Aggiornata
    if state_guard.is_none() {
        let gl = glow::Context::from_loader_function(|s| {
            crate::graphic::hook::get_proc_address(s) as *const _
        });
        let gl = std::sync::Arc::new(gl);
        let ctx = egui::Context::default();

        // CORREZIONE 1: Aggiunto `false` per il dithering finale
        let painter = egui_glow::Painter::new(gl, "", None, false).unwrap();

        *state_guard = Some(EguiState {
            ctx,
            painter,
            last_left_down: false,
            last_right_down: false,
            last_mouse_pos: egui::pos2(0.0, 0.0),
            // Inizializziamo la finestra al centro dello schermo
            window_pos: egui::pos2(screen_width / 2.0 - 200.0, screen_height / 2.0 - 150.0),
        });
    }

    let state = state_guard.as_mut().unwrap();
    let raw_input = gather_egui_inputs(state, screen_width, screen_height, scale_factor);

    let is_open = GUI_OPEN.load(Ordering::Relaxed);

    // Cloniamo il Context. È leggerissimo (usa Arc internamente) e ci permette
    // di modificare `state` (come state.window_pos) dentro la closure `run`.
    let ctx = state.ctx.clone();

    // CORREZIONE 2: ctx.run invece di begin_frame / end_frame
    let full_output = ctx.run(raw_input, |ctx| {
        crate::graphic::gui::render_all(ctx);
    }); // Fine di ctx.run

    // Il resto del rendering OpenGL rimane identico, full_output ora viene da ctx.run
    let clipped_primitives = state
        .ctx
        .tessellate(full_output.shapes, full_output.pixels_per_point);

    // --- INIZIO FIX GEROGLIFICI ---
    // Resettiamo lo stato di unpack dei pixel che Minecraft spesso corrompe
    // prima di far caricare le texture dei font a egui
    unsafe {
        crate::gl::ActiveTexture(crate::gl::TEXTURE0);
        crate::gl::PixelStorei(crate::gl::UNPACK_ALIGNMENT, 1); // Egui preferisce l'allineamento a 1 byte
        crate::gl::PixelStorei(crate::gl::UNPACK_ROW_LENGTH, 0); // Questo è il colpevole principale al 99%!
        crate::gl::PixelStorei(crate::gl::UNPACK_SKIP_PIXELS, 0);
        crate::gl::PixelStorei(crate::gl::UNPACK_SKIP_ROWS, 0);

        // Egui_glow salva e ripristina lo stato di blend e depth, ma per sicurezza:
        crate::gl::Disable(crate::gl::CULL_FACE);
        crate::gl::Disable(crate::gl::DEPTH_TEST);
        crate::gl::Enable(crate::gl::BLEND);
    }
    // --- FINE FIX ---

    // Ora disegniamo in sicurezza
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
