//! The `Mob` wrapper — a non-player living entity.

use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::mapping::MappedObject;
use jni::objects::GlobalRef;

/// A Minecraft `Mob` (non-player living entity). Its behaviour comes from
/// [`EntityRef`] and [`LivingEntityRef`].
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Mob)]
pub struct Mob {
    jni_ref: GlobalRef,
}

impl Mob {
    /// Wraps an existing `Mob` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Mob {
        Mob { jni_ref }
    }
}

impl EntityRef for Mob {}
impl LivingEntityRef for Mob {}
