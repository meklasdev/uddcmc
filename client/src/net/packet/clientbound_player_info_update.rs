//! `ClientboundPlayerInfoUpdatePacket` — held by reference rather than
//! snapshotted.
//!
//! Freecam needs to *queue* these inbound packets while it owns the local
//! player's game mode, and re-deliver them when it stops. The packet's
//! contents (batch of multi-field entries) are messy to snapshot and rebuild,
//! so this variant simply keeps the original JVM object and forwards the same
//! object back through the listener on replay.

use crate::mapping::MinecraftClassType;
use jni::objects::GlobalRef;

/// Class of `ClientboundPlayerInfoUpdatePacket`.
pub const CLASS_TYPE: MinecraftClassType = MinecraftClassType::ClientboundPlayerInfoUpdatePacket;

/// A captured `ClientboundPlayerInfoUpdatePacket`, held verbatim.
#[derive(Debug, Clone)]
pub struct ClientboundPlayerInfoUpdatePacket {
    pub jni_ref: GlobalRef,
}

/// Pass-through variant: it is never mutated, so the dispatch must never
/// observe it as "changed" (which would force a rebuild). Treat any two
/// captures as equal — the value is opaque to comparison anyway.
impl PartialEq for ClientboundPlayerInfoUpdatePacket {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
