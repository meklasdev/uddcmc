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

    /// The melee attack strength, `0.0..=1.0` — `1.0` once the attack cooldown
    /// has fully recharged. Below `1.0` the next hit deals reduced damage.
    pub fn attack_strength_scale(&self) -> anyhow::Result<f32> {
        Ok(self
            .call_method("getAttackStrengthScale", &[JValue::Float(0.5)])?
            .f()?)
    }

    /// Plays the main-hand swing animation (and sends it to the server).
    pub fn swing(&self) -> anyhow::Result<()> {
        self.in_frame(|| {
            let hand = mapping()
                .get_static_field(
                    MinecraftClassType::InteractionHand,
                    "MAIN_HAND",
                    FieldType::Object(MinecraftClassType::InteractionHand),
                )?
                .l()?;
            self.call_method("swing", &[JValue::Object(&hand)])?;
            Ok(())
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

    /// Sets the creative-fly speed (vanilla default `0.05`).
    pub fn set_flying_speed(&self, speed: f32) -> anyhow::Result<()> {
        self.call_method("setFlyingSpeed", &[JValue::Float(speed)])?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_may_fly(&self) -> anyhow::Result<bool> {
        Ok(self.get_field("mayfly", FieldType::Boolean)?.z()?)
    }
}
