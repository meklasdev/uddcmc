//! Rust value-snapshots of the Minecraft packets DarkClient reads or rewrites.
//!
//! Each packet is read once into a plain Rust struct (`read`), modified freely
//! by the dispatch, then rebuilt into a JVM object (`to_java`). All JNI uses
//! explicit descriptors — no reflection, no overload resolution.

pub mod move_player;

use move_player::MovePlayerPacket;

/// A packet passing through the connection, in a form modules can `match` on.
/// The dispatch wraps the raw JVM packet into this enum, lets each enabled
/// module's `handle_packet` modify it, then rebuilds the JVM object.
#[derive(Debug, Clone, PartialEq)]
pub enum Packet {
    /// An outbound `ServerboundMovePlayerPacket`.
    MovePlayer(MovePlayerPacket),
}
