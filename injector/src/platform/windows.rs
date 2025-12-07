use crate::platform::{AGENT_NAME, LIBRARY_NAME, SOCKET_ADDRESS};
use dll_syringe::process::{OwnedProcess, Process};
use dll_syringe::Syringe;
use log::{error, info};
use std::io::Write;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use std::{io, path, thread};

pub fn inject(pid: u32) -> Result<(), io::Error> {
    let target_process = OwnedProcess::from_pid(pid)
        .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e))?;

    let syringe = Syringe::for_process(target_process);

    let loader_dll_name = format!("{}.dll", AGENT_NAME);
    let loader_path = PathBuf::from(&loader_dll_name);
    let lib_path = PathBuf::from(format!("{}.dll", LIBRARY_NAME));

    let abs_loader_path = std::fs::canonicalize(&loader_path)
        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e))?;

    let is_loaded = syringe
        .process()
        .find_module_by_name(&loader_dll_name)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
        .is_some();

    // Check if agent_loader is already loaded
    if !is_loaded {
        info!(
            "Injecting {} into PID {}...",
            abs_loader_path.display(),
            pid
        );

        match syringe.inject(&abs_loader_path) {
            Ok(module) => {
                info!("The DLL is successfully injected! {:?}", module);
            }
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Injection failed: {}", e),
                ));
            }
        }

        // Wait a moment for complete initialization
        thread::sleep(Duration::from_millis(1000));
    } else {
        info!("Agent Loader already loaded");
    }

    // Send a reload command to agent_loader
    match TcpStream::connect_timeout(&SOCKET_ADDRESS, Duration::from_secs(5)) {
        Ok(mut stream) => {
            let lib_abs_path = match path::absolute(&lib_path) {
                Ok(p) => p,
                Err(e) => {
                    error!("Unable to get absolute path: {:?}", e);
                    return Err(e);
                }
            };

            info!("Connected to {}. Sending reload command", SOCKET_ADDRESS);

            let lib_abs_path = lib_abs_path.to_string_lossy();
            let lib_abs_path = lib_abs_path.trim_matches(|c| c == '"' || c == '\'');
            // Send the command with the absolute path of the library
            let command = format!("reload {}", lib_abs_path);
            info!("Command: {}", command);

            if let Err(e) = stream.write(command.as_bytes()) {
                error!("Unable to send reload command: {:?}", e);
            }
        }
        Err(e) => {
            error!("Unable to connect to server: {:?}", e);
        }
    }

    Ok(())
}
