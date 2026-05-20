//! Linux agent injection via `ptrace`.

use std::path::Path;

use log::{info, warn};
use proc_maps::get_process_maps;
use ptrace_inject::{Injector, Process};

use super::{AgentInjector, InjectError};

/// Injects shared libraries into a running process with `ptrace`.
#[derive(Debug, Default)]
pub struct PlatformInjector;

impl AgentInjector for PlatformInjector {
    fn is_agent_loaded(&self, pid: u32, agent_file: &str) -> bool {
        match get_process_maps(pid as i32) {
            Ok(maps) => maps
                .iter()
                .filter_map(|m| m.filename())
                .filter_map(|p| p.file_name())
                .any(|name| name == agent_file),
            Err(e) => {
                warn!("could not read memory maps for pid {pid}: {e}");
                false
            }
        }
    }

    fn inject(&self, pid: u32, agent_path: &Path) -> Result<(), InjectError> {
        if !Path::new(&format!("/proc/{pid}")).exists() {
            return Err(InjectError::ProcessGone(pid));
        }

        let process = Process::get(pid).map_err(|e| InjectError::Attach {
            pid,
            source: e.to_string().into(),
        })?;
        let mut injector = Injector::attach(process).map_err(|e| InjectError::Attach {
            pid,
            source: e.to_string().into(),
        })?;
        injector
            .inject(agent_path)
            .map_err(|e| InjectError::Inject(e.to_string().into()))?;

        info!("agent injected into pid {pid}");
        Ok(())
    }
}

/// Whether the process runs as root.
pub fn is_elevated() -> bool {
    // SAFETY: `geteuid` takes no arguments and never fails.
    unsafe { libc::geteuid() == 0 }
}
