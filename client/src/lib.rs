// The framework exposes deliberate toolkit API (animation, theming,
// notification, mapping helpers) that is not all wired up to a caller yet.
#![allow(dead_code)]

extern crate jni;
mod graphic;
mod mapping;
mod module;
mod state;

pub mod gl {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use crate::graphic::hook::{install_hooks, uninstall_hooks};
use crate::module::combat::aimbot::AimbotModule;
use crate::module::combat::killaura::KillAuraModule;
use crate::module::combat::mobaura::MobAuraModule;
use crate::module::movement::fly::FlyModule;
use crate::module::render::chest_esp::ChestEspModule;
use crate::module::render::mob_esp::MobEspModule;
use crate::module::render::player_esp::PlayerEspModule;
use crate::state::{client, init};
use log::{error, info, LevelFilter};
use simplelog::{Config, WriteLogger};
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

/// Cleared by [`cleanup_client`]; gates the frame hook and background loops.
pub static RUNNING: AtomicBool = AtomicBool::new(false);

/// Entry point called by the agent loader once the library is loaded.
#[no_mangle]
pub extern "C" fn initialize_client() {
    // Make sure we can't initialize more than once.
    if RUNNING.swap(true, Ordering::SeqCst) {
        info!("Client already initialized");
        return;
    }

    match WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create("dark_client.log").unwrap(),
    ) {
        Ok(_) => info!("Logger initialized"),
        Err(e) => eprintln!("Error during logger initialization: {:?}", e),
    }

    // Custom panic hook: guarantee the input / render hooks are released.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        error!("DarkClient panicked! Restoring input/render hooks…");
        cleanup_client();
        default_hook(panic_info);
    }));

    thread::spawn(|| {
        info!("Starting DarkClient…");

        // Fixed, straight-line startup order: build the global state, then
        // register modules, then install the hooks last so the frame hook
        // never observes an uninitialized client.
        if let Err(e) = init() {
            error!("Client initialization failed: {e}");
            return;
        }
        register_modules();
        if let Err(e) = install_hooks() {
            error!("Failed to install hooks: {e}");
        }
        info!("DarkClient started");
    });
}

/// Cleanup entry point called by the agent loader before unload.
#[no_mangle]
pub extern "C" fn cleanup_client() {
    info!("Client cleanup in progress…");
    RUNNING.store(false, Ordering::SeqCst);
    uninstall_hooks();
    crate::graphic::input::cleanup();
    info!("Client cleanup completed");
}

/// Registers the built-in modules with the client.
fn register_modules() {
    let client = client();
    client.register_module(FlyModule::new());
    client.register_module(KillAuraModule::new());
    client.register_module(MobAuraModule::new());
    client.register_module(AimbotModule::new());
    client.register_module(PlayerEspModule::new());
    client.register_module(MobEspModule::new());
    client.register_module(ChestEspModule::new());
}
