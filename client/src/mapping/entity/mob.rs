//! Wrapper for Minecraft's `Mob` — a non-player living entity.

use crate::mapping::entity::Entity;
use crate::mapping::{JavaObject, MinecraftClassType};
use jni::objects::GlobalRef;
use std::ops::Deref;

/// A Minecraft `Mob` (non-player living entity).
#[derive(Debug, Clone)]
pub struct Mob {
    pub jni_ref: GlobalRef,
    pub entity: Entity,
}

impl Mob {
    /// Wraps an existing `Mob` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Mob {
        Mob {
            entity: Entity::new(jni_ref.clone()),
            jni_ref,
        }
    }
}

impl JavaObject for Mob {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::Mob
    }
}

impl Deref for Mob {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
