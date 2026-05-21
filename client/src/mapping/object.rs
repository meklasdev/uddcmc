//! The [`JavaObject`] trait — shared behaviour for every Rust wrapper around a
//! live JVM object.
//!
//! It keeps JNI and [`Mapping`](crate::mapping::Mapping) calls inside the
//! `mapping` module: feature code (modules, overlay, …) works through the typed
//! wrappers and these methods, never `mapping()` directly.

use crate::mapping::MinecraftClassType;
use crate::state::mapping;
use jni::objects::GlobalRef;

/// A Rust wrapper around a JVM object.
pub trait JavaObject {
    /// The wrapped JVM object.
    fn jni_ref(&self) -> &GlobalRef;

    /// The Minecraft class this wrapper type corresponds to.
    fn class_type() -> MinecraftClassType
    where
        Self: Sized;

    /// Whether the wrapped object is an instance of the Minecraft class that
    /// `T` corresponds to — e.g. `entity.instance_of::<Player>()`.
    fn instance_of<T: JavaObject>(&self) -> bool {
        mapping()
            .in_frame(|| mapping().is_instance_of(T::class_type(), self.jni_ref().as_obj()))
            .unwrap_or(false)
    }

    /// Whether this and `other` wrap the very same JVM object.
    fn is_same<T: JavaObject>(&self, other: &T) -> bool {
        let Ok(env) = mapping().get_env() else {
            return false;
        };
        env.is_same_object(self.jni_ref().as_obj(), other.jni_ref().as_obj())
            .unwrap_or(false)
    }
}
