//! Input interception.
//!
//! Swaps the host's GLFW mouse/cursor/key callbacks for our own so the overlay
//! can read input and, while the GUI is open, swallow events before they reach
//! Minecraft. The logic is platform-agnostic; only locating the GLFW shared
//! library differs between Linux and Windows (see [`open_glfw_library`]).

use crate::state::{client, minecraft};
use libloading::Library;
use log::info;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// --- Public input state ----------------------------------------------------

/// Whether the overlay GUI is open. While open, input is consumed instead of
/// being forwarded to Minecraft.
pub static GUI_OPEN: AtomicBool = AtomicBool::new(false);

/// Last key pressed, or `-1` once consumed. Used by the keybind picker.
pub static LAST_KEY_PRESSED: AtomicI32 = AtomicI32::new(-1);

/// Snapshot of the mouse, updated from the GLFW callbacks and read by the UI.
pub static MOUSE_STATE: Mutex<MouseState> = Mutex::new(MouseState::NEW);

#[derive(Default, Clone, Copy)]
pub struct MouseState {
    pub x: f64,
    pub y: f64,
    pub left_down: bool,
    pub right_down: bool,
    pub left_clicked: bool,
    pub right_clicked: bool,
}

impl MouseState {
    const NEW: MouseState = MouseState {
        x: 0.0,
        y: 0.0,
        left_down: false,
        right_down: false,
        left_clicked: false,
        right_clicked: false,
    };
}

// --- GLFW constants --------------------------------------------------------

const GLFW_RELEASE: i32 = 0;
const GLFW_PRESS: i32 = 1;
const GLFW_MOUSE_BUTTON_LEFT: i32 = 0;
const GLFW_MOUSE_BUTTON_RIGHT: i32 = 1;
const GLFW_KEY_RIGHT_SHIFT: i32 = 344;

const GLFW_CURSOR: i32 = 0x0003_3001;
const GLFW_CURSOR_NORMAL: i32 = 0x0003_4001;
const GLFW_CURSOR_DISABLED: i32 = 0x0003_4003;

// --- GLFW function-pointer types -------------------------------------------

type MouseButtonFun = extern "C" fn(*mut c_void, i32, i32, i32);
type CursorPosFun = extern "C" fn(*mut c_void, f64, f64);
type KeyFun = extern "C" fn(*mut c_void, i32, i32, i32, i32);

type GetCurrentContext = extern "C" fn() -> *mut c_void;
type SetMouseButtonCallback = extern "C" fn(*mut c_void, MouseButtonFun) -> *mut c_void;
type SetCursorPosCallback = extern "C" fn(*mut c_void, CursorPosFun) -> *mut c_void;
type SetKeyCallback = extern "C" fn(*mut c_void, KeyFun) -> *mut c_void;
type SetInputMode = extern "C" fn(*mut c_void, i32, i32);
type SetCursorPos = extern "C" fn(*mut c_void, f64, f64);

// --- Installed-hook state --------------------------------------------------

/// Everything captured when the GLFW callbacks were swapped: the library (kept
/// alive so the resolved symbols stay valid), the window, the host's original
/// callbacks, and the two functions needed to toggle cursor capture.
struct GlfwHooks {
    library: Library,
    window: *mut c_void,
    original_mouse_button: *mut c_void,
    original_cursor_pos: *mut c_void,
    original_key: *mut c_void,
    set_input_mode: SetInputMode,
    set_cursor_pos: SetCursorPos,
}

// The raw pointers are only ever touched on the render/GLFW thread; the struct
// is published through `OnceLock` purely for read-only access.
unsafe impl Send for GlfwHooks {}
unsafe impl Sync for GlfwHooks {}

static HOOKS: OnceLock<GlfwHooks> = OnceLock::new();

/// Last real cursor position, stored as `f64` bits. While the GUI is open the
/// cursor reported to Minecraft is frozen here, so the camera does not spin.
static CURSOR_LOCK_X: AtomicU64 = AtomicU64::new(0);
static CURSOR_LOCK_Y: AtomicU64 = AtomicU64::new(0);

fn cursor_lock() -> (f64, f64) {
    (
        f64::from_bits(CURSOR_LOCK_X.load(Ordering::Relaxed)),
        f64::from_bits(CURSOR_LOCK_Y.load(Ordering::Relaxed)),
    )
}

fn set_cursor_lock(x: f64, y: f64) {
    CURSOR_LOCK_X.store(x.to_bits(), Ordering::Relaxed);
    CURSOR_LOCK_Y.store(y.to_bits(), Ordering::Relaxed);
}

// --- Callbacks -------------------------------------------------------------

extern "C" fn on_mouse_button(window: *mut c_void, button: i32, action: i32, mods: i32) {
    if let Ok(mut state) = MOUSE_STATE.lock() {
        match (button, action) {
            (GLFW_MOUSE_BUTTON_LEFT, GLFW_PRESS) => {
                state.left_down = true;
                state.left_clicked = true;
            }
            (GLFW_MOUSE_BUTTON_LEFT, GLFW_RELEASE) => state.left_down = false,
            (GLFW_MOUSE_BUTTON_RIGHT, GLFW_PRESS) => {
                state.right_down = true;
                state.right_clicked = true;
            }
            (GLFW_MOUSE_BUTTON_RIGHT, GLFW_RELEASE) => state.right_down = false,
            _ => {}
        }
    }

    // While the GUI is open, swallow the event instead of forwarding it.
    if GUI_OPEN.load(Ordering::Relaxed) {
        return;
    }
    if let Some(hooks) = HOOKS.get() {
        if !hooks.original_mouse_button.is_null() {
            let original: MouseButtonFun =
                unsafe { std::mem::transmute(hooks.original_mouse_button) };
            original(window, button, action, mods);
        }
    }
}

extern "C" fn on_cursor_pos(window: *mut c_void, x: f64, y: f64) {
    if let Ok(mut state) = MOUSE_STATE.lock() {
        state.x = x;
        state.y = y;
    }

    // While the GUI is open, feed Minecraft the frozen position so dragging the
    // overlay does not move the camera; otherwise track the real cursor.
    let (send_x, send_y) = if GUI_OPEN.load(Ordering::Relaxed) {
        cursor_lock()
    } else {
        set_cursor_lock(x, y);
        (x, y)
    };

    if let Some(hooks) = HOOKS.get() {
        if !hooks.original_cursor_pos.is_null() {
            let original: CursorPosFun = unsafe { std::mem::transmute(hooks.original_cursor_pos) };
            original(window, send_x, send_y);
        }
    }
}

extern "C" fn on_key(window: *mut c_void, key: i32, scancode: i32, action: i32, mods: i32) {
    if action == GLFW_PRESS {
        LAST_KEY_PRESSED.store(key, Ordering::Relaxed);
    }

    if key == GLFW_KEY_RIGHT_SHIFT && action == GLFW_PRESS {
        toggle_gui();
    }

    // While the GUI is open, swallow the event instead of forwarding it.
    if GUI_OPEN.load(Ordering::Relaxed) {
        return;
    }

    if action == GLFW_PRESS {
        handle_module_keybind(key);
    }

    if let Some(hooks) = HOOKS.get() {
        if !hooks.original_key.is_null() {
            let original: KeyFun = unsafe { std::mem::transmute(hooks.original_key) };
            original(window, key, scancode, action, mods);
        }
    }
}

/// Toggles the overlay GUI and the matching cursor-capture mode.
fn toggle_gui() {
    // `fetch_xor` returns the previous value; the new state is its negation.
    let open = !GUI_OPEN.fetch_xor(true, Ordering::Relaxed);

    let Some(hooks) = HOOKS.get() else {
        return;
    };
    if hooks.window.is_null() {
        return;
    }

    if open {
        // GUI open: release the cursor.
        (hooks.set_input_mode)(hooks.window, GLFW_CURSOR, GLFW_CURSOR_NORMAL);
    } else {
        // GUI closed: restore the cursor where Minecraft last had it, then
        // re-capture it — this avoids a camera jump on the next mouse move.
        let (lock_x, lock_y) = cursor_lock();
        (hooks.set_cursor_pos)(hooks.window, lock_x, lock_y);
        (hooks.set_input_mode)(hooks.window, GLFW_CURSOR, GLFW_CURSOR_DISABLED);
    }
}

/// Toggles any module whose keybind matches `key`, when in-world.
fn handle_module_keybind(key: i32) {
    let minecraft = minecraft();
    if !minecraft.current_screen_is_null() || !minecraft.in_world() {
        return;
    }

    for handle in client().modules.handles() {
        let Ok(mut module) = handle.lock() else {
            continue;
        };
        if module.get_module_data().key_bind as i32 != key {
            continue;
        }

        let enabled = !module.get_module_data().enabled;
        info!(
            "{} {}",
            module.get_module_data().name(),
            if enabled { "enabled" } else { "disabled" }
        );
        if enabled {
            let _ = module.on_start();
        } else {
            let _ = module.on_stop();
        }
        module.get_module_data_mut().set_enabled(enabled);
    }
}

// --- Lifecycle -------------------------------------------------------------

/// Installs the GLFW input hooks once the window exists. Cheap to call every
/// frame: it returns immediately after the first success and only retries
/// while the window is not yet available.
pub fn init() {
    if HOOKS.get().is_some() {
        return;
    }
    if let Some(hooks) = install_glfw_hooks() {
        let _ = HOOKS.set(hooks);
    }
}

/// Resolves the GLFW symbols, swaps in our callbacks, and captures the state
/// needed to restore them. Returns `None` until the window is ready.
fn install_glfw_hooks() -> Option<GlfwHooks> {
    let library = crate::graphic::platform::open_glfw_library()?;

    unsafe {
        let get_context = *library
            .get::<GetCurrentContext>(b"glfwGetCurrentContext")
            .ok()?;
        let set_mouse_button = *library
            .get::<SetMouseButtonCallback>(b"glfwSetMouseButtonCallback")
            .ok()?;
        let set_cursor_pos_cb = *library
            .get::<SetCursorPosCallback>(b"glfwSetCursorPosCallback")
            .ok()?;
        let set_key = *library.get::<SetKeyCallback>(b"glfwSetKeyCallback").ok()?;
        let set_input_mode = *library.get::<SetInputMode>(b"glfwSetInputMode").ok()?;
        let set_cursor_pos = *library.get::<SetCursorPos>(b"glfwSetCursorPos").ok()?;

        let window = get_context();
        if window.is_null() {
            return None;
        }

        info!("GLFW window acquired; installing input callbacks.");

        let original_mouse_button = set_mouse_button(window, on_mouse_button);
        let original_cursor_pos = set_cursor_pos_cb(window, on_cursor_pos);
        let original_key = set_key(window, on_key);

        // If the GUI was toggled on before hooks existed, release the cursor.
        if GUI_OPEN.load(Ordering::Relaxed) {
            set_input_mode(window, GLFW_CURSOR, GLFW_CURSOR_NORMAL);
        }

        Some(GlfwHooks {
            library,
            window,
            original_mouse_button,
            original_cursor_pos,
            original_key,
            set_input_mode,
            set_cursor_pos,
        })
    }
}

/// Restores the host's original GLFW callbacks and cursor capture. Safe to
/// call even if the hooks were never installed.
pub fn cleanup() {
    let Some(hooks) = HOOKS.get() else {
        return;
    };
    if hooks.window.is_null() {
        return;
    }

    type RestoreCallback = extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
    let restorations: [(&[u8], *mut c_void); 3] = [
        (b"glfwSetMouseButtonCallback", hooks.original_mouse_button),
        (b"glfwSetCursorPosCallback", hooks.original_cursor_pos),
        (b"glfwSetKeyCallback", hooks.original_key),
    ];
    unsafe {
        for (name, original) in restorations {
            if let Ok(restore) = hooks.library.get::<RestoreCallback>(name) {
                restore(hooks.window, original);
            }
        }
    }

    // Leave Minecraft with its cursor captured, as it expects in-world.
    (hooks.set_input_mode)(hooks.window, GLFW_CURSOR, GLFW_CURSOR_DISABLED);
    info!("GLFW input callbacks restored.");
}
