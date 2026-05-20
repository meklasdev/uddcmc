//! Windows agent injection via `dll-syringe`.

use std::path::Path;

use dll_syringe::process::{OwnedProcess, Process};
use dll_syringe::Syringe;
use log::info;

use super::{AgentInjector, InjectError};

/// Injects DLLs into a running process with `dll-syringe`.
#[derive(Debug, Default)]
pub struct PlatformInjector;

impl AgentInjector for PlatformInjector {
    fn is_agent_loaded(&self, pid: u32, agent_file: &str) -> bool {
        let Ok(process) = OwnedProcess::from_pid(pid) else {
            return false;
        };
        matches!(process.find_module_by_name(agent_file), Ok(Some(_)))
    }

    fn inject(&self, pid: u32, agent_path: &Path) -> Result<(), InjectError> {
        let process = OwnedProcess::from_pid(pid).map_err(|_| InjectError::ProcessGone(pid))?;
        let syringe = Syringe::for_process(process);
        syringe
            .inject(agent_path)
            .map_err(|e| InjectError::Inject(e.to_string().into()))?;

        info!("agent injected into pid {pid}");
        Ok(())
    }
}

/// Whether the process runs with Administrator privileges.
pub fn is_elevated() -> bool {
    is_elevated::is_elevated()
}
