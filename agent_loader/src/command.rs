//! Command parsing and dispatch for a single client connection.

use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::time::Duration;

use log::{error, info};
use protocol::Command;

use crate::library;

/// Maximum time spent waiting for a command line before giving up.
const READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Reads one command off `stream` and executes it. Runs on its own thread,
/// so a slow reload never blocks later connections.
pub fn handle_connection(stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if let Err(e) = reader.read_line(&mut line) {
        error!("failed to read command: {e}");
        return;
    }

    match Command::decode(&line) {
        Ok(Command::Reload(path)) => {
            info!("reload command received: {}", path.display());
            if let Err(e) = library::reload(&path) {
                error!("reload failed: {e}");
            }
        }
        Err(e) => error!("ignoring invalid command {:?}: {e}", line.trim()),
    }
}
