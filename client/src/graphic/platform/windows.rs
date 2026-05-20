//! Windows graphics platform glue.

use std::ffi::{c_void, CString};
use std::sync::OnceLock;

use libloading::Library;
use log::info;

use super::HookTarget;

/// Shared library that exports the OpenGL entry points.
const GL_LIBRARY: &str = "opengl32.dll";

/// Lazily-opened, process-lifetime handle to the OpenGL library.
fn gl_library() -> Option<&'static Library> {
    static LIB: OnceLock<Option<Library>> = OnceLock::new();
    // SAFETY: opening the GL library the host process already maps.
    LIB.get_or_init(|| unsafe { Library::new(GL_LIBRARY).ok() })
        .as_ref()
}

/// Resolves an OpenGL function pointer by name.
pub fn gl_proc_address(name: &str) -> *const c_void {
    let Some(lib) = gl_library() else {
        return std::ptr::null();
    };
    // SAFETY: resolving an exported GL symbol by name.
    unsafe {
        // Modern extension entry points are reachable only through
        // wglGetProcAddress; opengl32.dll itself exports just the GL 1.1 core.
        if let Ok(c_name) = CString::new(name) {
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

/// Opens the GLFW shared library, trying the names Minecraft launchers use.
pub fn open_glfw_library() -> Option<Library> {
    // SAFETY: loading the GLFW DLL the host process already maps.
    unsafe {
        Library::new("glfw.dll")
            .or_else(|_| Library::new("glfw3.dll"))
            .or_else(|_| Library::new("glfw64.dll"))
            .ok()
    }
}

/// The buffer-swap function to hook: `wglSwapBuffers` from `opengl32.dll`.
pub fn frame_hook_targets() -> Vec<HookTarget> {
    // SAFETY: resolving wglSwapBuffers from opengl32.dll by name.
    unsafe {
        let Ok(library) = Library::new(GL_LIBRARY) else {
            return Vec::new();
        };
        let address = match library.get::<unsafe extern "system" fn()>(b"wglSwapBuffers") {
            Ok(symbol) => *symbol as *const () as usize,
            Err(_) => return Vec::new(),
        };
        info!("found wglSwapBuffers in {GL_LIBRARY} at 0x{address:x}");
        // Keep the library handle alive for the process lifetime.
        std::mem::forget(library);
        vec![HookTarget {
            address,
            label: GL_LIBRARY.to_string(),
        }]
    }
}
