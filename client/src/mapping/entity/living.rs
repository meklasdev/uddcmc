//! Wrapper for Minecraft's `LivingEntity`.

use crate::mapping::entity::Entity;
use crate::mapping::{JavaObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;
use std::ops::Deref;

/// A Minecraft `LivingEntity` — an [`Entity`] that also carries health.
#[derive(Debug, Clone)]
pub struct LivingEntity {
    pub jni_ref: GlobalRef,
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
        Ok(mapping()
            .call_method(
                MinecraftClassType::LivingEntity,
                self.jni_ref.as_obj(),
                "getHealth",
                &[],
            )?
            .f()?)
    }

    /// Maximum health.
    pub fn get_max_health(&self) -> anyhow::Result<f32> {
        Ok(mapping()
            .call_method(
                MinecraftClassType::LivingEntity,
                self.jni_ref.as_obj(),
                "getMaxHealth",
                &[],
            )?
            .f()?)
    }
}

impl JavaObject for LivingEntity {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::LivingEntity
    }
}

impl Deref for LivingEntity {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
