#![cfg_attr(debug_assertions, allow(dead_code))]

extern crate jni;
mod client;
mod gui;
mod mapping;
mod module;
mod hook;

use crate::client::keyboard::{start_keyboard_handler, stop_keyboard_handler};
use crate::client::DarkClient;
use crate::gui::start_gui;
use crate::mapping::client::minecraft::Minecraft;
use module::combat::aimbot::AimbotModule;
use module::combat::killaura::KillAuraModule;
use module::movement::fly::FlyModule;
use log::{error, info, LevelFilter};
use simplelog::{Config, WriteLogger};
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;
use crate::module::combat::mobaura::MobAuraModule;

static GUI_THREAD: OnceLock<Mutex<Option<thread::JoinHandle<()>>>> = OnceLock::new();

// Flag to control if the client is running
static RUNNING: AtomicBool = AtomicBool::new(false);

fn gui_thread() -> &'static Mutex<Option<thread::JoinHandle<()>>> {
    GUI_THREAD.get_or_init(|| Mutex::new(None))
}

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

    thread::spawn(|| {
        info!("Starting DarkClient...");
        let minecraft = Minecraft::instance();

        register_modules(minecraft);

        start_keyboard_handler();

        // Install hooks
        if let Err(e) = hook::install_hooks() {
            error!("Failed to install hooks: {}", e);
        }

        let gui_handle = thread::spawn(move || match start_gui() {
            Ok(_) => info!("GUI thread started"),
            Err(e) => error!("Error while starting GUI thread: {}", e),
        });

        // Memorize the thread handle in a thread-safe way
        let mut gui_lock = gui_thread().lock().unwrap();
        *gui_lock = Some(gui_handle);

        info!(
            "Player position: {:?}",
            minecraft.player.entity.get_position()
        );
    });
}

// Cleanup function for agent_loader
#[no_mangle]
pub extern "C" fn cleanup_client() {
    info!("Client cleanup in progress...");

    // Set the execution flag to false
    RUNNING.store(false, Ordering::SeqCst);

    // Stop the keyboard handler
    stop_keyboard_handler();

    let gui_handle = {
        let mut gui_lock = gui_thread().lock().unwrap();
        gui_lock.take()
    };

    if let Some(handle) = gui_handle {
        // Give a short timeout for waiting
        if let Err(e) = handle.join() {
            error!("Error while waiting for gui thread: {:?}", e);
        }
    }

    // Clean up other resources if necessary
    info!("Client cleanup completed");
}

fn register_modules(minecraft: &'static Minecraft) {
    let client = DarkClient::instance();

    client.register_module(FlyModule::new(minecraft.player.clone()));
    client.register_module(KillAuraModule::new());
    client.register_module(MobAuraModule::new());
    client.register_module(AimbotModule::new());
}
