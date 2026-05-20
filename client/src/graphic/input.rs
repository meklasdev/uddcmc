use libc::c_void;
use log::{error, info};
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

// Global Input State
pub static GUI_OPEN: AtomicBool = AtomicBool::new(false);
pub static LAST_KEY_PRESSED: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

lazy_static::lazy_static! {
    pub static ref MOUSE_STATE: Mutex<MouseState> = Mutex::new(MouseState::default());
}

#[derive(Default, Clone, Copy)]
pub struct MouseState {
    pub x: f64,
    pub y: f64,
    pub left_down: bool,
    pub right_down: bool,
    pub left_clicked: bool,
    pub right_clicked: bool,
}

#[cfg(target_os = "linux")]
mod linux_input {
    use super::*;
    use std::sync::Once;

    // Original callbacks and GLFW state
    static mut ORIGINAL_MOUSE_BTN: *mut c_void = std::ptr::null_mut();
    static mut ORIGINAL_CURSOR_POS: *mut c_void = std::ptr::null_mut();
    static mut ORIGINAL_KEY_CB: *mut c_void = std::ptr::null_mut();

    static mut GLFW_WINDOW: *mut c_void = std::ptr::null_mut();
    static mut GLFW_SET_INPUT_MODE: Option<extern "C" fn(*mut c_void, i32, i32)> = None;
    static mut GLFW_SET_CURSOR_POS: Option<extern "C" fn(*mut c_void, f64, f64)> = None;

    // To prevent massive camera spins when un-grabbing
    static mut C_LOCK_X: f64 = 0.0;
    static mut C_LOCK_Y: f64 = 0.0;

    // Type definitions for GLFW callbacks
    type GlfwMouseButtonFun = extern "C" fn(*mut c_void, i32, i32, i32);
    type GlfwCursorPosFun = extern "C" fn(*mut c_void, f64, f64);
    type GlfwKeyFun = extern "C" fn(*mut c_void, i32, i32, i32, i32);

    extern "C" fn my_mouse_button_callback(
        window: *mut c_void,
        button: i32,
        action: i32,
        mods: i32,
    ) {
        if action == 1 {
            // GLFW_PRESS
            if button == 0 {
                // GLFW_MOUSE_BUTTON_LEFT
                if let Ok(mut state) = MOUSE_STATE.lock() {
                    state.left_down = true;
                    state.left_clicked = true;
                }
            } else if button == 1 {
                // GLFW_MOUSE_BUTTON_RIGHT
                if let Ok(mut state) = MOUSE_STATE.lock() {
                    state.right_down = true;
                    state.right_clicked = true;
                }
            }
        } else if action == 0 {
            // GLFW_RELEASE
            if button == 0 {
                if let Ok(mut state) = MOUSE_STATE.lock() {
                    state.left_down = false;
                }
            } else if button == 1 {
                if let Ok(mut state) = MOUSE_STATE.lock() {
                    state.right_down = false;
                }
            }
        }

        // If GUI is open, consume the event (don't pass to Minecraft)
        if GUI_OPEN.load(Ordering::Relaxed) {
            return;
        }

        // Pass to original
        unsafe {
            if !ORIGINAL_MOUSE_BTN.is_null() {
                let orig: GlfwMouseButtonFun = std::mem::transmute(ORIGINAL_MOUSE_BTN);
                orig(window, button, action, mods);
            }
        }
    }

    extern "C" fn my_cursor_pos_callback(window: *mut c_void, xpos: f64, ypos: f64) {
        if let Ok(mut state) = MOUSE_STATE.lock() {
            state.x = xpos;
            state.y = ypos;
        }

        // If GUI is open, we freeze the position sent to Minecraft to the last known position
        let mut send_x = xpos;
        let mut send_y = ypos;

        if GUI_OPEN.load(Ordering::Relaxed) {
            unsafe {
                send_x = C_LOCK_X;
                send_y = C_LOCK_Y;
            }
        } else {
            unsafe {
                C_LOCK_X = xpos;
                C_LOCK_Y = ypos;
            }
        }

        unsafe {
            if !ORIGINAL_CURSOR_POS.is_null() {
                let orig: GlfwCursorPosFun = std::mem::transmute(ORIGINAL_CURSOR_POS);
                orig(window, send_x, send_y);
            }
        }
    }

    extern "C" fn my_key_callback(
        window: *mut c_void,
        key: i32,
        scancode: i32,
        action: i32,
        mods: i32,
    ) {
        if action == 1 {
            LAST_KEY_PRESSED.store(key, Ordering::Relaxed);
        }

        if key == 344 /* Right Shift */ && action == 1 {
            // Toggle GUI
            let current = GUI_OPEN.load(Ordering::Relaxed);
            let next = !current;
            GUI_OPEN.store(next, Ordering::Relaxed);

            // Toggle mouse visibility
            unsafe {
                if let Some(set_mode) = GLFW_SET_INPUT_MODE {
                    if !GLFW_WINDOW.is_null() {
                        if next {
                            // GUI Open -> Normal Pointer
                            set_mode(GLFW_WINDOW, 0x00033001, 0x00034001);
                        } else {
                            // GUI Closed -> Disabled / Captured Pointer
                            if let Some(set_cursor_pos) = GLFW_SET_CURSOR_POS {
                                set_cursor_pos(GLFW_WINDOW, C_LOCK_X, C_LOCK_Y);
                            }
                            set_mode(GLFW_WINDOW, 0x00033001, 0x00034003);
                        }
                    }
                }
            }
        }

        if GUI_OPEN.load(Ordering::Relaxed) {
            return;
        }

        // --- Module Toggling ---
        // Only trigger on action == 1 (GLFW_PRESS) to prevent duplicates or release triggers
        if action == 1 {
            let minecraft = crate::mapping::client::minecraft::Minecraft::instance();
            if minecraft.current_screen_is_null() && minecraft.get_player().is_ok() {
                let client = crate::client::DarkClient::instance();
                if let Ok(modules) = client.modules.read() {
                    for module in modules.values() {
                        let mut module = module.lock().unwrap();
                        let module_data = module.get_module_data();

                        if module_data.key_bind as i32 == key {
                            let enabled = !module_data.enabled;
                            log::info!(
                                "{} {}",
                                module_data.name,
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
                }
            }
        }

        unsafe {
            if !ORIGINAL_KEY_CB.is_null() {
                let orig: GlfwKeyFun = std::mem::transmute(ORIGINAL_KEY_CB);
                orig(window, key, scancode, action, mods);
            }
        }
    }

    pub fn init_glfw_hooks() {
        static HOOKED_ONCE: Once = Once::new();
        HOOKED_ONCE.call_once(|| {
            unsafe {
                let path = crate::graphic::hook::find_library_path("libglfw.so")
                    .unwrap_or_else(|| "libglfw.so".to_string());
                let libglfw = libc::dlopen(CString::new(path).unwrap().as_ptr(), libc::RTLD_LAZY);
                if libglfw.is_null() {
                    error!("Could not open libglfw.so to hook inputs.");
                    return;
                }

                // Get function pointers
                let get_current_context: extern "C" fn() -> *mut c_void =
                    std::mem::transmute(libc::dlsym(
                        libglfw,
                        CString::new("glfwGetCurrentContext").unwrap().as_ptr(),
                    ));
                let set_mouse_button: extern "C" fn(
                    *mut c_void,
                    GlfwMouseButtonFun,
                ) -> *mut c_void = std::mem::transmute(libc::dlsym(
                    libglfw,
                    CString::new("glfwSetMouseButtonCallback").unwrap().as_ptr(),
                ));
                let set_cursor_pos: extern "C" fn(*mut c_void, GlfwCursorPosFun) -> *mut c_void =
                    std::mem::transmute(libc::dlsym(
                        libglfw,
                        CString::new("glfwSetCursorPosCallback").unwrap().as_ptr(),
                    ));
                let set_key_cb: extern "C" fn(*mut c_void, GlfwKeyFun) -> *mut c_void =
                    std::mem::transmute(libc::dlsym(
                        libglfw,
                        CString::new("glfwSetKeyCallback").unwrap().as_ptr(),
                    ));
                let set_input_mode: extern "C" fn(*mut c_void, i32, i32) = std::mem::transmute(
                    libc::dlsym(libglfw, CString::new("glfwSetInputMode").unwrap().as_ptr()),
                );
                let set_cursor_pos_func: extern "C" fn(*mut c_void, f64, f64) = std::mem::transmute(
                    libc::dlsym(libglfw, CString::new("glfwSetCursorPos").unwrap().as_ptr()),
                );

                let window = get_current_context();
                if window.is_null() {
                    error!("No glfw window context found yet.");
                    return;
                }

                info!("Successfully got GLFW window. Placing callback overrides...");

                // Swap callbacks and store original
                ORIGINAL_MOUSE_BTN = set_mouse_button(window, my_mouse_button_callback);
                ORIGINAL_CURSOR_POS = set_cursor_pos(window, my_cursor_pos_callback);
                ORIGINAL_KEY_CB = set_key_cb(window, my_key_callback);

                // Store globally so we can toggle grab state later
                GLFW_WINDOW = window;
                GLFW_SET_INPUT_MODE = Some(set_input_mode);
                GLFW_SET_CURSOR_POS = Some(set_cursor_pos_func);

                // Assuming GUI is open by default: ungrab mouse right away
                if GUI_OPEN.load(Ordering::Relaxed) {
                    set_input_mode(window, 0x00033001, 0x00034001);
                }
            }
        });
    }

    pub fn cleanup_glfw_hooks() {
        unsafe {
            if !GLFW_WINDOW.is_null() && !ORIGINAL_CURSOR_POS.is_null() {
                // We need to re-find the functions since we don't store them,
                // but we can just use dlsym again.
                if let Some(path) = crate::graphic::hook::find_library_path("libglfw.so") {
                    let libglfw =
                        libc::dlopen(CString::new(path).unwrap().as_ptr(), libc::RTLD_LAZY);
                    if !libglfw.is_null() {
                        let set_mouse_button: extern "C" fn(*mut c_void, *mut c_void) =
                            std::mem::transmute(libc::dlsym(
                                libglfw,
                                CString::new("glfwSetMouseButtonCallback").unwrap().as_ptr(),
                            ));
                        let set_cursor_pos: extern "C" fn(*mut c_void, *mut c_void) =
                            std::mem::transmute(libc::dlsym(
                                libglfw,
                                CString::new("glfwSetCursorPosCallback").unwrap().as_ptr(),
                            ));
                        let set_key_cb: extern "C" fn(*mut c_void, *mut c_void) =
                            std::mem::transmute(libc::dlsym(
                                libglfw,
                                CString::new("glfwSetKeyCallback").unwrap().as_ptr(),
                            ));

                        // Restore original callbacks
                        set_mouse_button(GLFW_WINDOW, ORIGINAL_MOUSE_BTN);
                        set_cursor_pos(GLFW_WINDOW, ORIGINAL_CURSOR_POS);
                        set_key_cb(GLFW_WINDOW, ORIGINAL_KEY_CB);

                        // Ensure mouse is ungrabbed (normal) if we panic'd while GUI was open
                        if let Some(set_mode) = GLFW_SET_INPUT_MODE {
                            set_mode(GLFW_WINDOW, 0x00033001, 0x00034003); // 0x00034003 is GLFW_CURSOR_DISABLED (Minecraft default)
                        }
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod windows_input {
    use super::*;
    use libloading::Library;
    use std::ffi::c_void;
    use std::sync::Once;

    static mut ORIGINAL_MOUSE_BTN: *mut c_void = std::ptr::null_mut();
    static mut ORIGINAL_CURSOR_POS: *mut c_void = std::ptr::null_mut();
    static mut ORIGINAL_KEY_CB: *mut c_void = std::ptr::null_mut();

    static mut GLFW_WINDOW: *mut c_void = std::ptr::null_mut();
    static mut GLFW_SET_INPUT_MODE: Option<extern "C" fn(*mut c_void, i32, i32)> = None;
    static mut GLFW_SET_CURSOR_POS: Option<extern "C" fn(*mut c_void, f64, f64)> = None;

    static mut C_LOCK_X: f64 = 0.0;
    static mut C_LOCK_Y: f64 = 0.0;

    type GlfwMouseButtonFun = extern "C" fn(*mut c_void, i32, i32, i32);
    type GlfwCursorPosFun = extern "C" fn(*mut c_void, f64, f64);
    type GlfwKeyFun = extern "C" fn(*mut c_void, i32, i32, i32, i32);

    extern "C" fn my_mouse_button_callback(
        window: *mut c_void,
        button: i32,
        action: i32,
        mods: i32,
    ) {
        if action == 1 {
            if button == 0 {
                if let Ok(mut state) = MOUSE_STATE.lock() {
                    state.left_down = true;
                    state.left_clicked = true;
                }
            } else if button == 1 {
                if let Ok(mut state) = MOUSE_STATE.lock() {
                    state.right_down = true;
                    state.right_clicked = true;
                }
            }
        } else if action == 0 {
            if button == 0 {
                if let Ok(mut state) = MOUSE_STATE.lock() {
                    state.left_down = false;
                }
            } else if button == 1 {
                if let Ok(mut state) = MOUSE_STATE.lock() {
                    state.right_down = false;
                }
            }
        }
        if GUI_OPEN.load(Ordering::Relaxed) {
            return;
        }
        unsafe {
            if !ORIGINAL_MOUSE_BTN.is_null() {
                let orig: GlfwMouseButtonFun = std::mem::transmute(ORIGINAL_MOUSE_BTN);
                orig(window, button, action, mods);
            }
        }
    }

    extern "C" fn my_cursor_pos_callback(window: *mut c_void, xpos: f64, ypos: f64) {
        if let Ok(mut state) = MOUSE_STATE.lock() {
            state.x = xpos;
            state.y = ypos;
        }
        let mut send_x = xpos;
        let mut send_y = ypos;

        if GUI_OPEN.load(Ordering::Relaxed) {
            unsafe {
                send_x = C_LOCK_X;
                send_y = C_LOCK_Y;
            }
        } else {
            unsafe {
                C_LOCK_X = xpos;
                C_LOCK_Y = ypos;
            }
        }

        unsafe {
            if !ORIGINAL_CURSOR_POS.is_null() {
                let orig: GlfwCursorPosFun = std::mem::transmute(ORIGINAL_CURSOR_POS);
                orig(window, send_x, send_y);
            }
        }
    }

    extern "C" fn my_key_callback(
        window: *mut c_void,
        key: i32,
        scancode: i32,
        action: i32,
        mods: i32,
    ) {
        if key == 344 && action == 1 {
            let next = !GUI_OPEN.load(Ordering::Relaxed);
            GUI_OPEN.store(next, Ordering::Relaxed);
            unsafe {
                if let Some(set_mode) = GLFW_SET_INPUT_MODE {
                    if !GLFW_WINDOW.is_null() {
                        if next {
                            set_mode(GLFW_WINDOW, 0x00033001, 0x00034001);
                        } else {
                            if let Some(set_cursor_pos) = GLFW_SET_CURSOR_POS {
                                set_cursor_pos(GLFW_WINDOW, C_LOCK_X, C_LOCK_Y);
                            }
                            set_mode(GLFW_WINDOW, 0x00033001, 0x00034003);
                        }
                    }
                }
            }
        }
        if GUI_OPEN.load(Ordering::Relaxed) {
            return;
        }

        // --- Module Toggling ---
        if action == 1 {
            let minecraft = crate::mapping::client::minecraft::Minecraft::instance();
            if minecraft.current_screen_is_null() && minecraft.get_player().is_ok() {
                let client = crate::client::DarkClient::instance();
                if let Ok(modules) = client.modules.read() {
                    for module in modules.values() {
                        let mut module = module.lock().unwrap();
                        let module_data = module.get_module_data();

                        if module_data.key_bind as i32 == key {
                            let enabled = !module_data.enabled;
                            log::info!(
                                "{} {}",
                                module_data.name,
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
                }
            }
        }

        unsafe {
            if !ORIGINAL_KEY_CB.is_null() {
                let orig: GlfwKeyFun = std::mem::transmute(ORIGINAL_KEY_CB);
                orig(window, key, scancode, action, mods);
            }
        }
    }

    static mut GLFW_LIB: Option<Library> = None;

    pub fn init_glfw_hooks() {
        static HOOKED_ONCE: Once = Once::new();
        HOOKED_ONCE.call_once(|| unsafe {
            let lib_result = Library::new("glfw.dll")
                .or_else(|_| Library::new("glfw3.dll"))
                .or_else(|_| Library::new("glfw64.dll"));

            let libglfw = match lib_result {
                Ok(l) => l,
                Err(_) => {
                    error!("Could not load glfw.dll on Windows.");
                    return;
                }
            };

            let get_current_context: libloading::Symbol<extern "C" fn() -> *mut c_void> =
                match libglfw.get(b"glfwGetCurrentContext") {
                    Ok(sym) => sym,
                    Err(_) => return,
                };
            let set_mouse_button: libloading::Symbol<
                extern "C" fn(*mut c_void, GlfwMouseButtonFun) -> *mut c_void,
            > = match libglfw.get(b"glfwSetMouseButtonCallback") {
                Ok(sym) => sym,
                Err(_) => return,
            };
            let set_cursor_pos: libloading::Symbol<
                extern "C" fn(*mut c_void, GlfwCursorPosFun) -> *mut c_void,
            > = match libglfw.get(b"glfwSetCursorPosCallback") {
                Ok(sym) => sym,
                Err(_) => return,
            };
            let set_key_cb: libloading::Symbol<
                extern "C" fn(*mut c_void, GlfwKeyFun) -> *mut c_void,
            > = match libglfw.get(b"glfwSetKeyCallback") {
                Ok(sym) => sym,
                Err(_) => return,
            };
            let set_input_mode: libloading::Symbol<extern "C" fn(*mut c_void, i32, i32)> =
                match libglfw.get(b"glfwSetInputMode") {
                    Ok(sym) => sym,
                    Err(_) => return,
                };

            let window = get_current_context();
            if window.is_null() {
                return;
            }

            info!("Successfully got Windows GLFW window. Modifying Hooks...");

            ORIGINAL_MOUSE_BTN = set_mouse_button(window, my_mouse_button_callback);
            ORIGINAL_CURSOR_POS = set_cursor_pos(window, my_cursor_pos_callback);
            ORIGINAL_KEY_CB = set_key_cb(window, my_key_callback);

            GLFW_WINDOW = window;
            GLFW_SET_INPUT_MODE = Some(*set_input_mode);

            if GUI_OPEN.load(Ordering::Relaxed) {
                (*set_input_mode)(window, 0x00033001, 0x00034001);
            }

            GLFW_LIB = Some(libglfw);
        });
    }

    pub fn cleanup_glfw_hooks() {
        unsafe {
            if !GLFW_WINDOW.is_null() && !ORIGINAL_CURSOR_POS.is_null() {
                if let Some(libglfw) = &GLFW_LIB {
                    if let Ok(set_mouse_button) = libglfw
                        .get::<extern "C" fn(*mut c_void, *mut c_void)>(
                            b"glfwSetMouseButtonCallback",
                        )
                    {
                        set_mouse_button(GLFW_WINDOW, ORIGINAL_MOUSE_BTN);
                    }
                    if let Ok(set_cursor_pos) = libglfw
                        .get::<extern "C" fn(*mut c_void, *mut c_void)>(b"glfwSetCursorPosCallback")
                    {
                        set_cursor_pos(GLFW_WINDOW, ORIGINAL_CURSOR_POS);
                    }
                    if let Ok(set_key_cb) = libglfw
                        .get::<extern "C" fn(*mut c_void, *mut c_void)>(b"glfwSetKeyCallback")
                    {
                        set_key_cb(GLFW_WINDOW, ORIGINAL_KEY_CB);
                    }
                    if let Some(set_mode) = GLFW_SET_INPUT_MODE {
                        set_mode(GLFW_WINDOW, 0x00033001, 0x00034003);
                    }
                }
            }
        }
    }
}

pub fn init() {
    #[cfg(target_os = "linux")]
    {
        linux_input::init_glfw_hooks();
    }
    #[cfg(target_os = "windows")]
    {
        windows_input::init_glfw_hooks();
    }
}

pub fn cleanup() {
    #[cfg(target_os = "linux")]
    {
        linux_input::cleanup_glfw_hooks();
    }
    #[cfg(target_os = "windows")]
    {
        windows_input::cleanup_glfw_hooks();
    }
}
