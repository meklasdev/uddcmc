mod app;
mod gui;
mod inject;
mod platform;
mod tui;

use log::{error, LevelFilter};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // `--list` only reads process info, so it needs no elevation.
    if args.iter().any(|a| a == "--list") {
        for proc in platform::find_minecraft_processes() {
            println!("{}\t{}", proc.pid, proc.info);
        }
        return;
    }

    if !platform::is_elevated() {
        eprintln!("{}", elevation_hint());
        std::process::exit(1);
    }

    let _ = protocol::init_file_logger("app.log", LevelFilter::Debug);

    // `--inject <pid>` — headless injection, for scripting and the e2e harness.
    if let Some(pid) = injection_target(&args) {
        std::process::exit(run_headless_injection(pid));
    }

    if args.iter().any(|a| a == "--tui") {
        tui::run_tui();
        return;
    }

    if let Err(e) = gui::run() {
        error!("GUI terminated with an error: {e}");
        eprintln!("Error: {e}");
    }
}

/// Parses a `--inject <pid>` argument pair, if present.
fn injection_target(args: &[String]) -> Option<u32> {
    let index = args.iter().position(|a| a == "--inject")?;
    args.get(index + 1)?.parse().ok()
}

/// Injects into `pid` without a UI; returns a process exit code.
fn run_headless_injection(pid: u32) -> i32 {
    match inject::inject(pid) {
        Ok(()) => {
            println!("injected into process {pid}");
            0
        }
        Err(e) => {
            eprintln!("injection into process {pid} failed: {e}");
            1
        }
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
