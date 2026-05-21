//! Wrapper for Minecraft's `Mob` — a non-player living entity.

use crate::mapping::entity::Entity;
use crate::mapping::MappedObject;
use jni::objects::GlobalRef;

/// A Minecraft `Mob` (non-player living entity).
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Mob)]
pub struct Mob {
    jni_ref: GlobalRef,
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
