use crate::client::DarkClient;
use crate::mapping::client::minecraft::Minecraft;
use cfg_if::cfg_if;
use ilhook::x64::HookPoint;
use log::info;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::{gl, RUNNING};

static LAST_TICK: AtomicI32 = AtomicI32::new(0);
static GL_LOADED: AtomicBool = AtomicBool::new(false);

// We create a wrapper to bypass the compiler's safety checks
pub struct HookHandle(HookPoint);

unsafe impl Send for HookHandle {}
unsafe impl Sync for HookHandle {}

// Global storage for the active hook.
// We use a Mutex to be able to modify (remove) it at runtime.
// Note: The exact type depends on what hooker.hook() returns.
// For ilhook-rs, the `Hook` object handles unhooking when it is dropped.
// We update the global storage to use this specific type, not dyn Any
static GLOBAL_HOOK: OnceLock<Mutex<Option<HookHandle>>> = OnceLock::new();

fn get_global_hook() -> &'static Mutex<Option<HookHandle>> {
    GLOBAL_HOOK.get_or_init(|| Mutex::new(None))
}

cfg_if! {
    if #[cfg(target_os = "linux")] {
        use std::ffi::CString;
        use ilhook::x64::{Hooker, Registers, CallbackOption, HookFlags, HookType};
        use libc::c_void;

        // Helper to load OpenGL functions on Linux
        fn get_proc_address(addr: &str) -> *const c_void {
            unsafe {
                let s = CString::new(addr).unwrap();
                // Try first with glXGetProcAddress if available, otherwise dlsym
                // Here we use a simplified approach assuming libGL is loaded
                let lib = libc::dlopen(CString::new("libGL.so.1").unwrap().as_ptr(), libc::RTLD_LAZY);
                if !lib.is_null() {
                    libc::dlsym(lib, s.as_ptr())
                } else {
                    std::ptr::null()
                }
            }
        }

        unsafe extern "win64" fn my_swap_buffers_hook(_regs: *mut Registers, _user_data: usize) {
            on_frame();
        }

    } else if #[cfg(target_os = "windows")] {
        use ilhook::x64::{Hooker, Registers, CallbackOption, HookFlags, HookType};
        use libloading::os::windows::{Library, Symbol};
        use std::ffi::CString;

        // Helper to load OpenGL functions on Windows
        // In a real context, we should have loaded opengl32.dll statically or lazy
        // Here we make a "dirty" but functional attempt for injection
        fn get_proc_address(addr: &str) -> *const std::ffi::c_void {
            unsafe {
                // Robust method: try wglGetProcAddress, then GetProcAddress
                let c_str = CString::new(addr).unwrap();

                let lib = libloading::Library::new("opengl32.dll");
                if let Ok(l) = lib {
                     // First try wglGetProcAddress (for modern extensions)
                     let wgl_get: Result<libloading::Symbol<unsafe extern "system" fn(*const i8) -> *const std::ffi::c_void>, _> = l.get(b"wglGetProcAddress");
                     if let Ok(wgl) = wgl_get {
                         let ptr = wgl(c_str.as_ptr());
                         if !ptr.is_null() {
                             return ptr;
                         }
                     }
                     // Fallback to GetProcAddress (for base GL 1.1 functions)
                     let func: Result<libloading::Symbol<unsafe extern "system" fn()>, _> = l.get(c_str.as_bytes());
                     if let Ok(f) = func {
                         return *f as *const std::ffi::c_void;
                     }
                }
                std::ptr::null()
            }
        }

        unsafe extern "win64" fn my_swap_buffers_hook(_regs: *mut Registers, _user_data: usize) {
            on_frame();
        }
    }
}

const CONTEXT_PROFILE_MASK: u32 = 0x9126;
const CONTEXT_CORE_PROFILE_BIT: u32 = 0x00000001;

// Function called every graphical frame
unsafe fn on_frame() {
    // === PANIC CHECK ===
    // If cleanup_client() has been called, RUNNING becomes false.
    // If it is false, we exit IMMEDIATELY. We don't tick, we don't draw.
    if !RUNNING.load(Ordering::SeqCst) {
        return;
    }

    // Initialize GL pointers...
    if !GL_LOADED.load(Ordering::Relaxed) {
        gl::load_with(|s| get_proc_address(s));
        GL_LOADED.store(true, Ordering::Relaxed);
    }

    check_tick();
    render_overlay();
}

use crate::graphic::render::Renderer;

// === RENDERING LOGIC ===
unsafe fn render_overlay() {
    // Skip if we are rendering on the Egui window
    if crate::gui::IS_EGUI_THREAD.with(|f| f.get()) {
        return;
    }

    if crate::mapping::client::minecraft::Minecraft::instance()
        .get_player()
        .is_err()
    {
        return;
    }

    // Initialize inputs when we are on the valid OpenGL thread context
    crate::graphic::input::init();

    // Initialize the renderer, which backs up OpenGL state
    let mut renderer = Renderer::new();

    // Call our new custom OpenGL UI system
    crate::graphic::ui::render_gui(&mut renderer);

    // the state is restored automatically when `renderer` goes out of scope and drops
}

fn check_tick() {
    let client = DarkClient::instance();
    // Try to get env without attaching if possible, or attach as daemon.
    let _env = match client.jvm.attach_current_thread_as_daemon() {
        Ok(env) => env,
        Err(_) => return,
    };

    let minecraft = Minecraft::instance();

    let player = match minecraft.get_player() {
        Ok(p) => p,
        Err(_) => return,
    };

    let tick_count = match player.entity.get_tick_count() {
        Ok(t) => t,
        Err(_) => return,
    };

    let last_tick = LAST_TICK.load(Ordering::Relaxed);

    if tick_count > last_tick {
        LAST_TICK.store(tick_count, Ordering::Relaxed);
        client.tick();
    }
}

pub fn find_library_path(partial_name: &str) -> Option<String> {
    if let Ok(file) = File::open("/proc/self/maps") {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(l) = line {
                // Look for a line containing the name (e.g. "liblwjgl_opengl.so")
                if l.contains(partial_name) && l.contains(".so") {
                    // The format is: address perms offset dev inode PATH
                    // We take the last part of the string
                    if let Some(path) = l.split_whitespace().last() {
                        return Some(path.to_string());
                    }
                }
            }
        }
    }
    None
}

pub fn install_hooks() -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    unsafe {
        let mut targets = Vec::new();

        if let Some(path) = find_library_path("libglfw.so") {
            info!("Found GLFW library: {}", path);
            targets.push((path, "glfwSwapBuffers"));
        } else if let Some(path) = find_library_path("liblwjgl.so") {
            info!("Found LWJGL library (Legacy): {}", path);
            targets.push((path, "glXSwapBuffers"));
        } else {
            info!("No specific library found, trying system libGL...");
            targets.push(("libGL.so.1".to_string(), "glXSwapBuffers"));
        }

        let mut hooked_count = 0;

        for (lib_path, func_name) in targets {
            let c_lib_path = CString::new(lib_path.clone())?;
            let c_func_name = CString::new(func_name)?;

            let lib = libc::dlopen(c_lib_path.as_ptr(), libc::RTLD_LAZY);

            if !lib.is_null() {
                let target_addr = libc::dlsym(lib, c_func_name.as_ptr()) as usize;

                if target_addr != 0 {
                    info!("Found {} in {} at 0x{:x}", func_name, lib_path, target_addr);

                    let hooker = Hooker::new(
                        target_addr,
                        HookType::JmpBack(my_swap_buffers_hook),
                        CallbackOption::None,
                        0,
                        HookFlags::empty(),
                    );

                    match hooker.hook() {
                        Ok(hook) => {
                            // Get the global lock
                            let mut guard = get_global_hook().lock().unwrap();

                            // If there was an old hook, we overwrite it (triggering automatic unhook)
                            if guard.is_some() {
                                info!("Detected old hook, removing and replacing...");
                            }

                            // Wrap the hook in our "Thread Safe" wrapper
                            *guard = Some(HookHandle(hook));

                            hooked_count += 1;
                            info!(">>> HOOK ACTIVE AND SAVED ON: {} <<<", lib_path);

                            // Exit the loop, one active hook is enough
                            break;
                        }
                        Err(e) => {
                            info!("Error installing hook on {}: {:?}", lib_path, e);
                        }
                    }
                }
            }
        }

        if hooked_count == 0 {
            // We could return an error here, BUT if we are re-injecting and something went wrong
            // with the static flag, we might want to "pretend" everything is fine.
            // However, for now we leave the error if count is 0.
            return Err(anyhow::anyhow!("Failed to hook any candidate libraries!"));
        }
    }

    #[cfg(target_os = "windows")]
    unsafe {
        use libloading::Library;
        let lib_name = "opengl32.dll";
        let func_name = "wglSwapBuffers";

        let lib = match Library::new(lib_name) {
            Ok(l) => l,
            Err(e) => {
                info!("Error loading opengl32: {}", e);
                return Err(anyhow::anyhow!("Failed to hook opengl32"));
            }
        };

        let target_addr: libloading::Symbol<unsafe extern "system" fn()> =
            match lib.get(func_name.as_bytes()) {
                Ok(s) => s,
                Err(e) => {
                    info!("wglSwapBuffers missing: {}", e);
                    return Err(anyhow::anyhow!("Failed to hook wglSwapBuffers"));
                }
            };

        let target_addr_val = *target_addr as *const () as usize;
        info!(
            "Found {} in {} at 0x{:x}",
            func_name, lib_name, target_addr_val
        );

        let hooker = Hooker::new(
            target_addr_val,
            HookType::JmpBack(my_swap_buffers_hook),
            CallbackOption::None,
            0,
            HookFlags::empty(),
        );

        match hooker.hook() {
            Ok(hook) => {
                let mut guard = get_global_hook().lock().unwrap();
                if guard.is_some() {
                    info!("Detected old hook, removing...");
                }
                *guard = Some(HookHandle(hook));
                info!(">>> HOOK ACTIVE AND SAVED ON: {} <<<", lib_name);
            }
            Err(e) => {
                info!("Hook error on {}: {:?}", lib_name, e);
                return Err(anyhow::anyhow!("Failed to hook wglSwapBuffers!"));
            }
        }

        // Keep the library handle alive
        std::mem::forget(lib);
    }
    Ok(())
}

pub fn uninstall_hooks() {
    // Take the lock
    let mut guard = get_global_hook().lock().unwrap();

    if guard.is_some() {
        info!("Physical hook removal in progress...");
        // By setting to None, the Wrapper is destroyed.
        // The Wrapper destroys the internal HookPoint.
        // The internal HookPoint restores the original memory bytes.
        *guard = None;
        info!("Hook removed successfully. Memory cleaned.");
    } else {
        info!("No active hook to remove.");
    }
}
