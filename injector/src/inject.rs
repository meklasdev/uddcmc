//! High-level injection orchestration.
//!
//! Injects the agent into the target process (platform-specific) and then
//! tells it, over the localhost TCP channel, to load or hot-reload the
//! client library.

use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use log::info;
use protocol::{Command, SOCKET_ADDR};

use crate::platform::{AgentInjector, InjectError, PlatformInjector};

/// Base names of the two shared libraries shipped alongside the injector.
const AGENT_NAME: &str = "agent_loader";
const CLIENT_NAME: &str = "client";

/// Total time to keep retrying the connection to the freshly started agent.
const CONNECT_DEADLINE: Duration = Duration::from_secs(5);
/// Timeout of a single connection attempt within that deadline.
const CONNECT_ATTEMPT_TIMEOUT: Duration = Duration::from_millis(500);
/// Pause between connection attempts.
const CONNECT_RETRY_PAUSE: Duration = Duration::from_millis(100);
/// Timeout for writing the command once connected.
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);

/// Progression steps during wtryskiwanie
#[derive(Debug, Clone)]
pub enum ProgressStep {
    Initializing,
    DetectingJvm,
    LoadingAgent,
    ConnectingClient,
    Finished,
    Failed(String),
}

/// Injects the agent into `pid` (unless already present) and triggers a
/// client (re)load. Blocking — callers run it off the UI thread.
pub fn inject(pid: u32) -> Result<(), InjectError> {
    let (tx, _) = std::sync::mpsc::channel();
    inject_with_progress(pid, &tx)
}

/// Injection with detailed stage-by-stage progression updates.
pub fn inject_with_progress(pid: u32, tx: &std::sync::mpsc::Sender<ProgressStep>) -> Result<(), InjectError> {
    tx.send(ProgressStep::Initializing).ok();
    std::thread::sleep(Duration::from_millis(150));

    let injector = PlatformInjector;

    let agent_file = library_file_name(AGENT_NAME);
    let agent_path = locate_library(&agent_file)?;
    let client_path = locate_library(&library_file_name(CLIENT_NAME))?;

    tx.send(ProgressStep::DetectingJvm).ok();
    std::thread::sleep(Duration::from_millis(250));

    if injector.is_agent_loaded(pid, &agent_file) {
        info!("agent already present in pid {pid}; reloading client only");
    } else {
        tx.send(ProgressStep::LoadingAgent).ok();
        std::thread::sleep(Duration::from_millis(250));
        injector.inject(pid, &agent_path)?;
    }

    tx.send(ProgressStep::ConnectingClient).ok();
    std::thread::sleep(Duration::from_millis(250));
    send_reload(&client_path)
}

/// Connects to the agent's command server and sends a [`Command::Reload`].
fn send_reload(client_lib: &Path) -> Result<(), InjectError> {
    let absolute = std::path::absolute(client_lib).map_err(InjectError::Path)?;
    // The client keeps its config in the directory the injector runs from.
    let config_dir = std::env::current_dir().map_err(InjectError::Path)?;

    let mut stream = connect_with_retry()?;
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));

    let command = Command::Reload {
        library: absolute,
        config_dir,
    }
    .encode();
    info!("sending command: {command}");
    stream
        .write_all(command.as_bytes())
        .map_err(InjectError::Send)?;
    Ok(())
}

/// Repeatedly tries to connect until [`CONNECT_DEADLINE`] elapses — this
/// covers the short window between injecting the agent and its TCP server
/// becoming reachable, without a blind fixed sleep.
fn connect_with_retry() -> Result<TcpStream, InjectError> {
    let deadline = Instant::now() + CONNECT_DEADLINE;
    loop {
        match TcpStream::connect_timeout(&SOCKET_ADDR, CONNECT_ATTEMPT_TIMEOUT) {
            Ok(stream) => return Ok(stream),
            Err(source) => {
                if Instant::now() >= deadline {
                    return Err(InjectError::Connect {
                        addr: SOCKET_ADDR,
                        source,
                    });
                }
                thread::sleep(CONNECT_RETRY_PAUSE);
            }
        }
    }
}

/// `foo` → `libfoo.so` / `foo.dll` / `libfoo.dylib` for the host OS.
fn library_file_name(name: &str) -> String {
    format!("{}{name}.{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_EXTENSION)
}

/// Looks for `file_name` next to the injector executable first, then in the
/// current working directory.
fn locate_library(file_name: &str) -> Result<PathBuf, InjectError> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(file_name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    let cwd_candidate = PathBuf::from(file_name);
    if cwd_candidate.is_file() {
        return Ok(cwd_candidate);
    }
    Err(InjectError::LibraryMissing(file_name.to_string()))
}
