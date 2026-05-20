//! macOS — and any other non-Linux/Windows target — agent injection stub.
//!
//! Injection is not implemented here yet. The seam exists so that adding
//! real macOS support (for example via `task_for_pid` plus a Mach thread,
//! or `DYLD_INSERT_LIBRARIES` for launch-time injection) means editing only
//! this file; nothing else in the crate is platform-aware.

use std::path::Path;

use super::{AgentInjector, InjectError};

/// Placeholder injector that reports the platform as unsupported.
#[derive(Debug, Default)]
pub struct PlatformInjector;

impl AgentInjector for PlatformInjector {
    fn is_agent_loaded(&self, _pid: u32, _agent_file: &str) -> bool {
        false
    }

    fn inject(&self, _pid: u32, _agent_path: &Path) -> Result<(), InjectError> {
        Err(InjectError::Unsupported)
    }
}

/// Whether the process runs as root (macOS is a Unix, so `geteuid` applies).
pub fn is_elevated() -> bool {
    // SAFETY: `geteuid` takes no arguments and never fails.
    unsafe { libc::geteuid() == 0 }
}
