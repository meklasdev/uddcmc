#![cfg_attr(debug_assertions, allow(dead_code))]

extern crate jni;
mod client;
mod graphic;
mod mapping;
mod module;

pub mod gl {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use crate::client::DarkClient;
use crate::graphic::hook::{install_hooks, uninstall_hooks};
use crate::mapping::client::minecraft::Minecraft;
use crate::module::combat::mobaura::MobAuraModule;
use log::{error, info, LevelFilter};
use module::combat::aimbot::AimbotModule;
use module::combat::killaura::KillAuraModule;
use module::movement::fly::FlyModule;
use simplelog::{Config, WriteLogger};
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

// Flag to control if the client is running
pub static RUNNING: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub extern "C" fn initialize_client() {
    // Make sure we can't initialize more than once
    if RUNNING.swap(true, Ordering::SeqCst) {
        info!("Client already initialized");
        return;
    }

    // Initialize the logger
    match WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create("dark_client.log").unwrap(),
    ) {
        Ok(_) => info!("Logger initialized"),
        Err(e) => eprintln!("Error during logger initialization: {:?}", e),
    }

    // Set up a custom panic hook to guarantee we release mouse/keyboard hooks
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        error!("DarkClient Panicked! Attempting to unhook inputs...");
        cleanup_client();
        default_hook(panic_info);
    }));

    thread::spawn(|| {
        info!("Starting DarkClient...");
        let minecraft = Minecraft::instance();

        register_modules();

        // Install hooks
        if let Err(e) = install_hooks() {
            error!("Failed to install hooks: {}", e);
        }

        match minecraft.get_player() {
            Ok(player) => {
                if let Ok(pos) = player.entity.get_position() {
                    info!("Initial Player position: {:?}", pos);
                }
            }
            Err(_) => info!("Client initialized, but player is not in-world yet."),
        }
    });
}

// Cleanup function for agent_loader
#[no_mangle]
pub extern "C" fn cleanup_client() {
    info!("Client cleanup in progress...");

    // Set the execution flag to false
    RUNNING.store(false, Ordering::SeqCst);

    // Remove the hooks
    uninstall_hooks();

    // Unlock GLFW Input / Restore callbacks if GUI was open
    crate::graphic::input::cleanup();

    // Clean up other resources if necessary
    info!("Client cleanup completed");
}

fn register_modules() {
    let client = DarkClient::instance();

    client.register_module(FlyModule::new());
    client.register_module(KillAuraModule::new());
    client.register_module(MobAuraModule::new());
    client.register_module(AimbotModule::new());
}
