//! Wrapper for Minecraft's `PlayerInfo` — the client-side record of a player's
//! game mode, latency, skin, etc. — and a `GameType.SPECTATOR` accessor.
//!
//! Freecam writes the local player's `PlayerInfo.gameMode` to `SPECTATOR` so
//! `Player.isSpectator()` returns true, which makes `Player.tick` keep
//! `noPhysics` set every tick (no view-blocking overlay).

use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};

/// A Minecraft `PlayerInfo` — the client-side record of a connected player.
#[derive(Debug, MappedObject)]
#[mapped(class = PlayerInfo)]
pub struct PlayerInfo {
    jni_ref: GlobalRef,
}

impl PlayerInfo {
    /// Wraps an existing `PlayerInfo` JVM object.
    pub fn new(jni_ref: GlobalRef) -> PlayerInfo {
        PlayerInfo { jni_ref }
    }

    /// The current game mode (`PlayerInfo.gameMode`) as a fresh global ref.
    pub fn get_game_mode(&self) -> anyhow::Result<GlobalRef> {
        self.in_frame(|| {
            let mode = self
                .get_field("gameMode", FieldType::Object(MinecraftClassType::GameType))?
                .l()?;
            mapping().new_global_ref(mode)
        })
    }

    /// Sets `PlayerInfo.gameMode` to the given `GameType` enum value.
    pub fn set_game_mode(&self, mode: &GlobalRef) -> anyhow::Result<()> {
        self.set_field(
            "gameMode",
            FieldType::Object(MinecraftClassType::GameType),
            JValue::Object(mode.as_obj()),
        )?;
        Ok(())
    }
}

/// The `GameType.SPECTATOR` enum constant.
pub fn game_type_spectator() -> anyhow::Result<GlobalRef> {
    mapping().in_frame(|| {
        let obj = mapping()
            .get_static_field(
                MinecraftClassType::GameType,
                "SPECTATOR",
                FieldType::Object(MinecraftClassType::GameType),
            )?
            .l()?;
        mapping().new_global_ref(obj)
    })
}
