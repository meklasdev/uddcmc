//! Wrappers for Minecraft's player classes: `Player`, `LocalPlayer` and the
//! `Abilities` they carry.

use crate::mapping::entity::Entity;
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};
use jni::sys::jboolean;

/// Any Minecraft `Player` entity (local or remote).
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Player)]
pub struct Player {
    jni_ref: GlobalRef,
    pub entity: Entity,
}

impl Player {
    /// Wraps an existing `Player` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Player {
        Player {
            entity: Entity::new(jni_ref.clone()),
            jni_ref,
        }
    }
}

/// The client's own `LocalPlayer`.
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = LocalPlayer)]
pub struct LocalPlayer {
    jni_ref: GlobalRef,
    pub abilities: Abilities,
    pub entity: Entity,
}

#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Abilities)]
pub struct Abilities {
    jni_ref: GlobalRef,
}

impl LocalPlayer {
    /// Wraps an existing `LocalPlayer` JVM object.
    pub fn new(player_ref: GlobalRef) -> anyhow::Result<Self> {
        Ok(Self {
            abilities: Abilities::new(player_ref.clone())?,
            entity: Entity::new(player_ref.clone()),
            jni_ref: player_ref,
        })
    }
}

impl Abilities {
    pub fn new(player: GlobalRef) -> anyhow::Result<Self> {
        mapping().in_frame(|| {
            let jni_ref = mapping()
                .call_method(MinecraftClassType::Player, &player, "getAbilities", &[])?
                .l()?;
            Ok(Self {
                jni_ref: mapping().new_global_ref(jni_ref)?,
            })
        })
    }

    pub fn fly(&self, value: bool) -> anyhow::Result<()> {
        let value: jboolean = if value { 1 } else { 0 };
        self.set_field("flying", FieldType::Boolean, JValue::Bool(value))?;
        self.set_field("mayfly", FieldType::Boolean, JValue::Bool(value))?;
        Ok(())
    }

    /// Whether the player is currently flying.
    pub fn is_flying(&self) -> anyhow::Result<bool> {
        Ok(self.get_field("flying", FieldType::Boolean)?.z()?)
    }

    #[allow(dead_code)]
    pub fn get_may_fly(&self) -> anyhow::Result<bool> {
        Ok(self.get_field("mayfly", FieldType::Boolean)?.z()?)
    }
}
