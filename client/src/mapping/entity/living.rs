//! The `LivingEntity` wrapper.

use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::MappedObject;
use jni::objects::GlobalRef;

/// A Minecraft `LivingEntity` — an [`Entity`](super::Entity) that also carries
/// health. Its behaviour comes from [`EntityRef`] and [`LivingEntityRef`].
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = LivingEntity)]
pub struct LivingEntity {
    jni_ref: GlobalRef,
}

impl LivingEntity {
    /// Wraps an existing `LivingEntity` JVM object.
    pub fn new(jni_ref: GlobalRef) -> LivingEntity {
        LivingEntity { jni_ref }
    }
}

impl EntityRef for LivingEntity {}
impl LivingEntityRef for LivingEntity {}
