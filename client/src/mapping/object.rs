//! The [`MappedObject`] trait — shared behaviour for every Rust wrapper around
//! a live JVM object.
//!
//! Implemented for every wrapper via `#[derive(MappedObject)]`. It keeps JNI
//! and [`Mapping`](crate::mapping::Mapping) calls inside the `mapping` module:
//! a wrapper method calls `self.call_method(...)` — the Minecraft class is
//! filled in from the wrapper's own type — and feature code only ever sees the
//! typed wrappers and the high-level helpers (`instance_of`, `is_same`,
//! `equals`).

use crate::mapping::{FieldType, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue, JValueOwned};

/// A Rust wrapper around a JVM object.
pub trait MappedObject {
    /// The wrapped JVM object.
    fn jni_ref(&self) -> &GlobalRef;

    /// The Minecraft class this wrapper type corresponds to.
    fn class_type() -> MinecraftClassType;

    /// Calls an instance method on the wrapped object, resolved against this
    /// wrapper's [`class_type`](MappedObject::class_type).
    fn call_method(&self, name: &str, args: &[JValue]) -> anyhow::Result<JValueOwned<'static>>
    where
        Self: Sized,
    {
        mapping().call_method(Self::class_type(), self.jni_ref().as_obj(), name, args)
    }

    /// Reads an instance field of the wrapped object.
    fn get_field(&self, name: &str, field_type: FieldType) -> anyhow::Result<JValueOwned<'static>>
    where
        Self: Sized,
    {
        mapping().get_field(Self::class_type(), self.jni_ref().as_obj(), name, field_type)
    }

    /// Writes an instance field of the wrapped object.
    fn set_field(&self, name: &str, field_type: FieldType, value: JValue) -> anyhow::Result<()>
    where
        Self: Sized,
    {
        mapping().set_field(
            Self::class_type(),
            self.jni_ref().as_obj(),
            name,
            field_type,
            value,
        )
    }

    /// Runs `f` inside a fresh JNI local-reference frame — see
    /// [`Mapping::in_frame`](crate::mapping::Mapping::in_frame).
    fn in_frame<R>(&self, f: impl FnOnce() -> anyhow::Result<R>) -> anyhow::Result<R> {
        mapping().in_frame(f)
    }

    /// Whether the wrapped object is an instance of the Minecraft class that
    /// `T` corresponds to — e.g. `entity.instance_of::<Player>()`.
    fn instance_of<T: MappedObject>(&self) -> bool {
        mapping()
            .in_frame(|| mapping().is_instance_of(T::class_type(), self.jni_ref().as_obj()))
            .unwrap_or(false)
    }

    /// Whether this and `other` wrap the very same JVM object (JNI identity).
    fn is_same<T: MappedObject>(&self, other: &T) -> bool {
        let Ok(env) = mapping().get_env() else {
            return false;
        };
        env.is_same_object(self.jni_ref().as_obj(), other.jni_ref().as_obj())
            .unwrap_or(false)
    }

    /// Java `Object.equals` between this and `other`. A failed JNI call — or a
    /// JVM that is unreachable — yields `false`.
    fn equals<T: MappedObject>(&self, other: &T) -> bool
    where
        Self: Sized,
    {
        self.in_frame(|| {
            Ok(self
                .call_method("equals", &[JValue::Object(other.jni_ref().as_obj())])?
                .z()?)
        })
        .unwrap_or(false)
    }
}
