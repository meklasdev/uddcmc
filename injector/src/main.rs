mod app;
mod gui;
mod inject;
mod platform;
mod tui;

use log::{error, LevelFilter};

fn main() {
    if !platform::is_elevated() {
        eprintln!("{}", elevation_hint());
        return;
    }

    if let Err(e) = protocol::init_file_logger("app.log", LevelFilter::Debug) {
        eprintln!("continuing without file logging: {e}");
    }

    if std::env::args().any(|arg| arg == "--tui") {
        tui::run_tui();
        return;
    }

    if let Err(e) = gui::run() {
        error!("GUI terminated with an error: {e}");
        eprintln!("Error: {e}");
    }
}

/// Platform-specific hint shown when the injector lacks the privileges it
/// needs to attach to another process.
fn elevation_hint() -> &'static str {
    #[cfg(windows)]
    {
        "This program must run as Administrator (right click → Run as administrator)."
    }
    #[cfg(not(windows))]
    {
        "This program must run with root privileges: sudo ./injector"
    }
}
