use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::Entity;
use crate::mapping::{JavaObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct MultiPlayerGameMode {
    pub jni_ref: GlobalRef,
}

impl MultiPlayerGameMode {
    pub fn new(jni_ref: GlobalRef) -> Self {
        Self { jni_ref }
    }

    /// Attacks `target` on behalf of the local player.
    pub fn attack(&self, player: &LocalPlayer, target: &Entity) -> anyhow::Result<()> {
        mapping().call_method(
            MinecraftClassType::MultiPlayerGameMode,
            self.jni_ref.as_obj(),
            "attack",
            &[
                JValue::Object(player.jni_ref.as_obj()),
                JValue::Object(target.jni_ref.as_obj()),
            ],
        )?;

        Ok(())
    }
}

impl JavaObject for MultiPlayerGameMode {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::MultiPlayerGameMode
    }
}

impl Deref for MultiPlayerGameMode {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
