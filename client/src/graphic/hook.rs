//! Frame hook.
//!
//! Intercepts the host's buffer-swap call so the overlay renders and the
//! client ticks exactly once per frame. OpenGL entry-point resolution — the
//! loader behind the `gl` and `glow` bindings — lives here too.

use crate::client::DarkClient;
use crate::mapping::client::minecraft::Minecraft;
use crate::{gl, RUNNING};
use ilhook::x64::{CallbackOption, HookFlags, HookPoint, HookType, Hooker, Registers};
use log::info;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};

static LAST_TICK: AtomicI32 = AtomicI32::new(0);
static GL_LOADED: AtomicBool = AtomicBool::new(false);

/// Wrapper that lets an `ilhook` handle cross thread boundaries. Dropping the
/// inner `HookPoint` restores the patched bytes, i.e. removes the hook.
pub struct HookHandle(HookPoint);

unsafe impl Send for HookHandle {}
unsafe impl Sync for HookHandle {}

/// The single active buffer-swap hook. Re-injection replaces it: the old
/// `HookHandle` drops and unhooks itself. `None` means nothing is installed.
static GLOBAL_HOOK: OnceLock<Mutex<Option<HookHandle>>> = OnceLock::new();

fn global_hook() -> &'static Mutex<Option<HookHandle>> {
    GLOBAL_HOOK.get_or_init(|| Mutex::new(None))
}

// --- OpenGL function resolution -------------------------------------------

/// Shared library that exports the OpenGL entry points on this platform.
const GL_LIBRARY: &str = if cfg!(target_os = "windows") {
    "opengl32.dll"
} else {
    "libGL.so.1"
};

/// Lazily-opened, process-lifetime handle to the OpenGL library.
fn gl_library() -> Option<&'static libloading::Library> {
    static LIB: OnceLock<Option<libloading::Library>> = OnceLock::new();
    LIB.get_or_init(|| unsafe { libloading::Library::new(GL_LIBRARY).ok() })
        .as_ref()
}

/// Resolves an OpenGL function pointer by name — the loader fed to `gl` and
/// `glow`. Returns null when the symbol cannot be found.
pub fn get_proc_address(name: &str) -> *const c_void {
    let Some(lib) = gl_library() else {
        return std::ptr::null();
    };
    unsafe {
        // On Windows, modern extension entry points are reachable only through
        // wglGetProcAddress; opengl32.dll itself exports just the GL 1.1 core.
        #[cfg(target_os = "windows")]
        if let Ok(c_name) = std::ffi::CString::new(name) {
            type WglGetProcAddress = unsafe extern "system" fn(*const i8) -> *const c_void;
            if let Ok(wgl) = lib.get::<WglGetProcAddress>(b"wglGetProcAddress") {
                let ptr = wgl(c_name.as_ptr());
                if !ptr.is_null() {
                    return ptr;
                }
            }
        }
        match lib.get::<unsafe extern "system" fn()>(name.as_bytes()) {
            Ok(symbol) => *symbol as *const c_void,
            Err(_) => std::ptr::null(),
        }
    }
}

// --- Per-frame logic -------------------------------------------------------

/// `ilhook` trampoline for the host's buffer-swap function.
unsafe extern "win64" fn swap_buffers_hook(_registers: *mut Registers, _user_data: usize) {
    on_frame();
}

/// Runs once per graphical frame, from the buffer-swap hook.
unsafe fn on_frame() {
    // If `cleanup_client()` has run, `RUNNING` is false — bail immediately:
    // no tick, no draw, the hooks are about to be torn down.
    if !RUNNING.load(Ordering::SeqCst) {
        return;
    }

    // Resolve the GL function pointers once, on the live render context.
    if !GL_LOADED.load(Ordering::Relaxed) {
        gl::load_with(get_proc_address);
        GL_LOADED.store(true, Ordering::Relaxed);
    }

    check_tick();
    render_overlay();
}

/// Renders the egui overlay for the current frame.
unsafe fn render_overlay() {
    if Minecraft::instance().get_player().is_err() {
        return;
    }

    // Install the input hooks lazily — they need the live GLFW window.
    crate::graphic::input::init();

    crate::graphic::ui_engine::render_egui_ui();
}

/// Detects a new game tick by watching the player's tick counter, and ticks
/// every enabled module when one is observed.
fn check_tick() {
    let client = DarkClient::instance();
    // Attach as a daemon so this render thread can make JNI calls.
    if client.jvm.attach_current_thread_as_daemon().is_err() {
        return;
    }

    let minecraft = Minecraft::instance();

    let tick_count = match minecraft.get_player() {
        Ok(player) => match player.entity.get_tick_count() {
            Ok(count) => count,
            Err(_) => return,
        },
        Err(_) => return,
    };

    if tick_count > LAST_TICK.load(Ordering::Relaxed) {
        LAST_TICK.store(tick_count, Ordering::Relaxed);
        client.tick();
    }
}

// --- Hook installation -----------------------------------------------------

/// Finds the on-disk path of a loaded shared object by scanning the process's
/// memory map. Linux-only — used to hook the exact GLFW the host loaded.
#[cfg(target_os = "linux")]
pub fn find_library_path(partial_name: &str) -> Option<String> {
    use std::io::{BufRead, BufReader};

    let file = std::fs::File::open("/proc/self/maps").ok()?;
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        // Format: address perms offset dev inode PATH
        if line.contains(partial_name) && line.contains(".so") {
            if let Some(path) = line.split_whitespace().last() {
                return Some(path.to_string());
            }
        }
    }
    None
}

/// Installs the buffer-swap hook that drives the overlay. Idempotent across
/// re-injection: the previous hook is dropped (and thus removed) first.
fn store_hook(hook: HookPoint, label: &str) {
    let mut guard = global_hook().lock().unwrap();
    if guard.is_some() {
        info!("Replacing the existing buffer-swap hook.");
    }
    *guard = Some(HookHandle(hook));
    info!(">>> HOOK ACTIVE ON: {} <<<", label);
}

/// Builds an `ilhook` `Hooker` for `target_addr` routed to [`swap_buffers_hook`].
fn hooker_for(target_addr: usize) -> Hooker {
    Hooker::new(
        target_addr,
        HookType::JmpBack(swap_buffers_hook),
        CallbackOption::None,
        0,
        HookFlags::empty(),
    )
}

#[cfg(target_os = "linux")]
pub fn install_hooks() -> anyhow::Result<()> {
    use std::ffi::CString;

    // Candidate (library, exported swap function) pairs, most specific first.
    let mut targets: Vec<(String, &str)> = Vec::new();
    if let Some(path) = find_library_path("libglfw.so") {
        info!("Found GLFW library: {}", path);
        targets.push((path, "glfwSwapBuffers"));
    } else if let Some(path) = find_library_path("liblwjgl.so") {
        info!("Found LWJGL library (legacy): {}", path);
        targets.push((path, "glXSwapBuffers"));
    } else {
        info!("No specific library found, falling back to system libGL.");
        targets.push(("libGL.so.1".to_string(), "glXSwapBuffers"));
    }

    for (lib_path, func_name) in targets {
        let c_lib_path = CString::new(lib_path.clone())?;
        let c_func_name = CString::new(func_name)?;

        unsafe {
            let lib = libc::dlopen(c_lib_path.as_ptr(), libc::RTLD_LAZY);
            if lib.is_null() {
                continue;
            }
            let target_addr = libc::dlsym(lib, c_func_name.as_ptr()) as usize;
            if target_addr == 0 {
                continue;
            }
            info!("Found {} in {} at 0x{:x}", func_name, lib_path, target_addr);

            match hooker_for(target_addr).hook() {
                Ok(hook) => {
                    store_hook(hook, &lib_path);
                    return Ok(());
                }
                Err(e) => info!("Error installing hook on {}: {:?}", lib_path, e),
            }
        }
    }

    Err(anyhow::anyhow!("Failed to hook any candidate libraries!"))
}

#[cfg(target_os = "windows")]
pub fn install_hooks() -> anyhow::Result<()> {
    use libloading::Library;

    const LIB_NAME: &str = "opengl32.dll";
    const FUNC_NAME: &[u8] = b"wglSwapBuffers";

    unsafe {
        let lib = Library::new(LIB_NAME)
            .map_err(|e| anyhow::anyhow!("Failed to load {}: {}", LIB_NAME, e))?;

        let swap_buffers: libloading::Symbol<unsafe extern "system" fn()> = lib
            .get(FUNC_NAME)
            .map_err(|e| anyhow::anyhow!("wglSwapBuffers missing: {}", e))?;

        let target_addr = *swap_buffers as *const () as usize;
        info!("Found wglSwapBuffers in {} at 0x{:x}", LIB_NAME, target_addr);

        let hook = hooker_for(target_addr)
            .hook()
            .map_err(|e| anyhow::anyhow!("Failed to hook wglSwapBuffers: {:?}", e))?;
        store_hook(hook, LIB_NAME);

        // Keep the library handle alive for the process lifetime.
        std::mem::forget(lib);
    }
    Ok(())
}

/// Removes the active buffer-swap hook, restoring the original bytes.
pub fn uninstall_hooks() {
    let mut guard = global_hook().lock().unwrap();
    if guard.take().is_some() {
        info!("Buffer-swap hook removed; memory restored.");
    } else {
        info!("No active hook to remove.");
    }
}
