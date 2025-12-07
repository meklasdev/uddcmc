use crate::platform::{AGENT_NAME, LIBRARY_NAME, SOCKET_ADDRESS};
use log::{error, info};
use proc_maps::get_process_maps;
use ptrace_inject::{Injector, Process};
use std::io::{Error, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;
use std::{path, thread};

pub fn inject(pid: u32) -> Result<(), Error> {
    // First time: load the agent_loader
    let loader_path = PathBuf::from(format!("{}.so", AGENT_NAME));
    let lib_path = PathBuf::from(format!("{}.so", LIBRARY_NAME));

    if !find_library(pid, format!("{}.so", AGENT_NAME).as_str()) {
        info!("Loading Agent Loader");

        let proc = match Process::get(pid) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to get Process for pid {}: {:?}", pid, e);
                return Err(Error::new(
                    std::io::ErrorKind::Other,
                    format!("Process::get failed: {:?}", e),
                ));
            }
        };

        match Injector::attach(proc) {
            Ok(mut injector) => match injector.inject(&loader_path) {
                Ok(_) => {
                    info!(
                        "Successfully injected library: {}",
                        loader_path.to_string_lossy()
                    );
                }
                Err(e) => {
                    error!("Injection failed: {:?}", e);
                    return Err(Error::new(std::io::ErrorKind::Other, e.to_string()));
                }
            },
            Err(e) => {
                error!("Failed to attach to pid {}: {:?}", pid, e);
                return Err(Error::new(std::io::ErrorKind::Other, e.to_string()));
            }
        }

        // Wait a moment for complete initialization
        thread::sleep(Duration::from_millis(500));
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

fn find_library(pid: u32, lib_name: &str) -> bool {
    let maps = get_process_maps(pid as i32).ok();
    if maps.is_none() {
        error!("Failed to get process maps");
        return false;
    }
    let maps = maps.unwrap();

    for map in maps {
        if let Some(path) = map.filename() {
            if path.ends_with(lib_name) {
                // Library loaded
                return true;
            }
        }
    }
    false
}
