//! Shared IPC contract between the `injector` and the `agent_loader`.
//!
//! The two processes speak a tiny line-based protocol over a localhost TCP
//! socket. Keeping the wire format, the socket address and the logger setup
//! in a single crate stops the two ends from silently drifting apart.

mod command;
mod logging;

pub use command::{Command, ProtocolError};
pub use logging::{init_file_logger, LoggerError};

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

/// Address the `agent_loader` command server binds and the `injector`
/// connects to. Localhost-only by design — the channel is never exposed off
/// the machine.
pub const SOCKET_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 7878);
