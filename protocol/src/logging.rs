//! Shared, non-panicking file-logger setup.

use std::fs::File;
use std::path::Path;

use log::LevelFilter;
use simplelog::{Config, WriteLogger};
use thiserror::Error;

/// Failure to set up file logging.
#[derive(Debug, Error)]
pub enum LoggerError {
    /// The log file could not be created.
    #[error("could not create log file: {0}")]
    CreateFile(#[from] std::io::Error),
    /// A global logger was already installed by someone else.
    #[error("a logger is already installed")]
    AlreadyInstalled(#[from] log::SetLoggerError),
}

/// Initializes a file logger without ever panicking.
///
/// A logging problem must never take the host process down, so on failure
/// the error is reported to stderr and returned to the caller instead of
/// unwinding.
pub fn init_file_logger(path: impl AsRef<Path>, level: LevelFilter) -> Result<(), LoggerError> {
    let result = (|| {
        let file = File::create(path.as_ref())?;
        WriteLogger::init(level, Config::default(), file)?;
        Ok(())
    })();
    if let Err(ref e) = result {
        eprintln!("[protocol] file logger init failed: {e}");
    }
    result
}
