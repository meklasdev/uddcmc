pub struct ProcessInfo {
    pub pid: u32,
    pub info: String, // Window Title (Windows) or Partial Arguments (Linux)
}

pub const AGENT_NAME: &str = "libagent_loader";
pub const LIBRARY_NAME: &str = "libclient";
pub const SOCKET_ADDRESS: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7878);

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use self::unix::inject;
#[cfg(windows)]
pub use self::windows::inject;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use sysinfo::System;

pub fn find_minecraft_processes() -> Vec<ProcessInfo> {
    let mut sys = System::new_all();
    sys.refresh_all();
    let mut processes: Vec<ProcessInfo> = Vec::new();

    for (pid, process) in sys.processes() {
        let name = process.name().to_string_lossy().to_lowercase();

        if name == "java" || name == "javaw" || name == "javaw.exe" || name == "java.exe" {
            let cmd: Vec<String> = process
                .cmd()
                .iter()
                .map(|arg| arg.to_string_lossy().to_string())
                .collect();
            let cmd_string = cmd.join(" ");

            if cmd_string.contains("minecraft") {
                let info = if let Some(idx) = cmd.iter().position(|r| r.contains("--version")) {
                    cmd.get(idx + 1)
                        .cloned()
                        .unwrap_or_else(|| "Unknown Version".to_string())
                } else {
                    "Minecraft Instance".to_string()
                };

                processes.push(ProcessInfo {
                    pid: pid.as_u32(),
                    info,
                });
            }
        }
    }

    processes.sort_by(|a, b| a.pid.cmp(&b.pid));

    processes
}
