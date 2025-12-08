use crate::client::DarkClient;
use crate::mapping::client::minecraft::Minecraft;
use cfg_if::cfg_if;
use log::info;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Once;

static LAST_TICK: AtomicI32 = AtomicI32::new(0);
static INIT: Once = Once::new();

cfg_if! {
    if #[cfg(target_os = "linux")] {
        use std::ffi::CString;
        use ilhook::x64::{Hooker, Registers, CallbackOption, HookFlags, HookType};

        // JmpBackRoutine signature: unsafe extern "win64" fn(*mut Registers, usize)
        // Note: "win64" ABI is available on x86_64 Linux in Rust.
        unsafe extern "win64" fn my_swap_buffers_hook(_regs: *mut Registers, _user_data: usize) {
            check_tick();
        }
    } else if #[cfg(target_os = "windows")] {
        use ilhook::x64::{Hooker, Registers, CallbackOption, HookFlags, HookType};

        unsafe extern "win64" fn my_swap_buffers_hook(_regs: *mut Registers, _user_data: usize) {
            check_tick();
        }
    }
}

fn check_tick() {
    let client = DarkClient::instance();
    // Try to get env without attaching if possible, or attach as daemon.
    let _env = match client.jvm.attach_current_thread_as_daemon() {
        Ok(env) => env,
        Err(_) => return,
    };

    let minecraft = Minecraft::instance();

    let tick_count = match minecraft.player.entity.get_tick_count() {
        Ok(t) => t,
        Err(_) => return,
    };

    let last_tick = LAST_TICK.load(Ordering::Relaxed);

    if tick_count > last_tick {
        LAST_TICK.store(tick_count, Ordering::Relaxed);
        client.tick();
    }
}

pub fn install_hooks() -> anyhow::Result<()> {
    cfg_if! {
        if #[cfg(target_os = "linux")] {
            unsafe {
                let lib_name = CString::new("libGL.so.1")?;
                let symbol_name = CString::new("glXSwapBuffers")?;

                let lib = libc::dlopen(lib_name.as_ptr(), libc::RTLD_LAZY);
                if lib.is_null() {
                     return Err(anyhow::anyhow!("Failed to load libGL.so.1"));
                }

                let target_addr = libc::dlsym(lib, symbol_name.as_ptr()) as usize;

                if target_addr == 0 {
                     return Err(anyhow::anyhow!("Failed to find glXSwapBuffers"));
                }

                // ilhook usage with JmpBack
                let hooker = Hooker::new(
                    target_addr,
                    HookType::JmpBack(my_swap_buffers_hook),
                    CallbackOption::None,
                    0, // user_data
                    HookFlags::empty()
                );

                let hook = hooker.hook();
                let hook = hook?;

                // We don't need trampoline for JmpBack hook as it jumps back automatically.

                // We need to keep the hook alive?
                Box::leak(Box::new(hook));

                info!("glXSwapBuffers hooked with ilhook (JmpBack)!");
            }
        } else if #[cfg(target_os = "windows")] {
             unsafe {
                let lib = libloading::Library::new("opengl32.dll")?;
                let func: libloading::Symbol<unsafe extern "system" fn()> = lib.get(b"wglSwapBuffers")?;
                let target_addr = func.into_raw().into_raw() as usize;

                // ilhook usage with JmpBack
                let hooker = Hooker::new(
                    target_addr,
                    HookType::JmpBack(my_swap_buffers_hook),
                    CallbackOption::None,
                    0, // user_data
                    HookFlags::empty()
                );

                let hook = hooker.hook();
                let hook = hook.unwrap();

                Box::leak(Box::new(hook));
                // Leak library to keep it loaded
                Box::leak(Box::new(lib));

                info!("wglSwapBuffers hooked with ilhook (JmpBack)!");
            }
        }
    }
    Ok(())
}
