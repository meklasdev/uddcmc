//! UI-agnostic core shared by the GUI and the TUI front-ends.
//!
//! Both front-ends own an [`InjectorApp`], drive it, and only render its
//! state. Injection runs on a worker thread so neither front-end blocks.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::inject::{self, ProgressStep};
use crate::platform::{find_minecraft_processes, ProcessInfo};

/// Where an injection attempt currently stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectionStatus {
    /// Idle and ready.
    Idle,
    /// A process scan is running.
    Scanning,
    /// Initializing state
    Initializing,
    /// Detecting JVM state
    DetectingJvm,
    /// Loading agent state
    LoadingAgent,
    /// Connecting client state
    ConnectingClient,
    /// Finished state
    Finished(u32),
    /// The last action failed, with a human-readable reason.
    Failed(String),
}

impl InjectionStatus {
    /// Short human-readable line suitable for a status bar.
    pub fn message(&self) -> String {
        match self {
            InjectionStatus::Idle => "Pick a Minecraft instance and inject.".to_string(),
            InjectionStatus::Scanning => "Searching Minecraft process...".to_string(),
            InjectionStatus::Initializing => "Initializing loader core...".to_string(),
            InjectionStatus::DetectingJvm => "Detecting target Java Virtual Machine...".to_string(),
            InjectionStatus::LoadingAgent => "Loading agent library...".to_string(),
            InjectionStatus::ConnectingClient => "Connecting client framework...".to_string(),
            InjectionStatus::Finished(pid) => format!("Successfully injected into process {pid}!"),
            InjectionStatus::Failed(reason) => format!("Error: {reason}"),
        }
    }
}

/// Front-end-independent injector state machine.
pub struct InjectorApp {
    processes: Vec<ProcessInfo>,
    selected_pid: Option<u32>,
    status: InjectionStatus,
    /// Result channel of an in-flight injection, with its target pid.
    pending: Option<(u32, Receiver<ProgressStep>)>,
}

impl InjectorApp {
    /// Creates an idle app with no scan results.
    pub fn new() -> Self {
        Self {
            processes: Vec::new(),
            selected_pid: None,
            status: InjectionStatus::Idle,
            pending: None,
        }
    }

    /// The processes found by the last [`scan`](Self::scan).
    pub fn processes(&self) -> &[ProcessInfo] {
        &self.processes
    }

    /// The currently selected process id, if any.
    pub fn selected_pid(&self) -> Option<u32> {
        self.selected_pid
    }

    /// The current status.
    pub fn status(&self) -> &InjectionStatus {
        &self.status
    }

    /// Whether an injection is currently running.
    pub fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Selects a process by pid, if it is in the current list.
    pub fn select(&mut self, pid: u32) {
        if self.processes.iter().any(|p| p.pid == pid) {
            self.selected_pid = Some(pid);
        }
    }

    /// Rescans for Minecraft processes. Keeps the current selection if it is
    /// still present, otherwise selects the first result.
    pub fn scan(&mut self) {
        if self.is_busy() {
            return;
        }
        self.status = InjectionStatus::Scanning;
        self.processes = find_minecraft_processes();

        let kept = self
            .selected_pid
            .is_some_and(|pid| self.processes.iter().any(|p| p.pid == pid));
        if !kept {
            self.selected_pid = self.processes.first().map(|p| p.pid);
        }
        self.status = InjectionStatus::Idle;
    }

    /// Starts injecting into the selected process on a worker thread. A no-op
    /// when nothing is selected or an injection is already running.
    pub fn start_injection(&mut self) {
        if self.is_busy() {
            return;
        }
        let Some(pid) = self.selected_pid else {
            return;
        };

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            match inject::inject_with_progress(pid, &tx) {
                Ok(()) => {
                    tx.send(ProgressStep::Finished).ok();
                }
                Err(e) => {
                    tx.send(ProgressStep::Failed(e.to_string())).ok();
                }
            }
        });
        self.pending = Some((pid, rx));
        self.status = InjectionStatus::Initializing;
    }

    /// Polls the injection worker. Front-ends call this once per frame/loop;
    /// returns `true` when the status changed so the GUI knows to repaint.
    pub fn poll(&mut self) -> bool {
        let (pid, rx) = match &self.pending {
            Some((p, r)) => (*p, r),
            None => return false,
        };
        let mut changed = false;
        let mut should_clear = false;

        loop {
            match rx.try_recv() {
                Ok(step) => {
                    match step {
                        ProgressStep::Initializing => {
                            self.status = InjectionStatus::Initializing;
                        }
                        ProgressStep::DetectingJvm => {
                            self.status = InjectionStatus::DetectingJvm;
                        }
                        ProgressStep::LoadingAgent => {
                            self.status = InjectionStatus::LoadingAgent;
                        }
                        ProgressStep::ConnectingClient => {
                            self.status = InjectionStatus::ConnectingClient;
                        }
                        ProgressStep::Finished => {
                            self.status = InjectionStatus::Finished(pid);
                            should_clear = true;
                        }
                        ProgressStep::Failed(reason) => {
                            self.status = InjectionStatus::Failed(reason);
                            should_clear = true;
                        }
                    }
                    changed = true;
                    if should_clear {
                        break;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.status = InjectionStatus::Failed("injection worker stopped unexpectedly".to_string());
                    should_clear = true;
                    changed = true;
                    break;
                }
            }
        }

        if should_clear {
            self.pending = None;
        }
        changed
    }
}

impl Default for InjectorApp {
    fn default() -> Self {
        Self::new()
    }
}
