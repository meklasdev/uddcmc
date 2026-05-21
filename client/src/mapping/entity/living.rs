//! Wrapper for Minecraft's `LivingEntity`.

use crate::mapping::entity::Entity;
use crate::mapping::MappedObject;
use jni::objects::GlobalRef;

/// A Minecraft `LivingEntity` — an [`Entity`] that also carries health.
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = LivingEntity)]
pub struct LivingEntity {
    jni_ref: GlobalRef,
    pub entity: Entity,
}

impl LivingEntity {
    /// Wraps an existing `LivingEntity` JVM object.
    pub fn new(jni_ref: GlobalRef) -> LivingEntity {
        LivingEntity {
            entity: Entity::new(jni_ref.clone()),
            jni_ref,
        }
    }

    /// Current health.
    pub fn get_health(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getHealth", &[])?.f()?)
    }

    /// Maximum health.
    pub fn get_max_health(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getMaxHealth", &[])?.f()?)
    }
}
