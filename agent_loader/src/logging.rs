//! Agent logger setup.

use log::LevelFilter;

/// Path of the agent's log file, created in the JVM's working directory.
const LOG_FILE: &str = "agent_loader.log";

/// Initializes file logging. Never panics — a logging failure must not stop
/// the agent from loading.
pub fn init() {
    if let Err(e) = protocol::init_file_logger(LOG_FILE, LevelFilter::Debug) {
        eprintln!("[agent_loader] file logging disabled: {e}");
    }
}
