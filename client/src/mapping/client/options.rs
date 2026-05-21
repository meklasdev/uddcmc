//! Wrappers for Minecraft's `Options` and `OptionInstance`.

use crate::mapping::{FieldType, JavaObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;
use std::ops::Deref;

/// Minecraft's game `Options`.
#[derive(Debug)]
pub struct Options {
    pub jni_ref: GlobalRef,
}

impl Options {
    /// Wraps an existing `Options` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Options {
        Options { jni_ref }
    }

    /// The field-of-view option.
    pub fn fov(&self) -> anyhow::Result<OptionInstance> {
        mapping().in_frame(|| {
            let option = mapping()
                .get_field(
                    MinecraftClassType::Options,
                    self.jni_ref.as_obj(),
                    "fov",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }
}

impl JavaObject for Options {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::Options
    }
}

impl Deref for Options {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}

/// A single Minecraft `OptionInstance` — one configurable game option.
#[derive(Debug)]
pub struct OptionInstance {
    pub jni_ref: GlobalRef,
}

impl OptionInstance {
    /// Wraps an existing `OptionInstance` JVM object.
    pub fn new(jni_ref: GlobalRef) -> OptionInstance {
        OptionInstance { jni_ref }
    }

    /// The current value, read as an `int` — the option's boxed value is
    /// unwrapped through `Integer.intValue()`.
    pub fn get_int(&self) -> anyhow::Result<i32> {
        mapping().in_frame(|| {
            let value = mapping()
                .call_method(
                    MinecraftClassType::OptionInstance,
                    self.jni_ref.as_obj(),
                    "get",
                    &[],
                )?
                .l()?;
            Ok(mapping()
                .call_method(MinecraftClassType::Integer, &value, "intValue", &[])?
                .i()?)
        })
    }
}

impl JavaObject for OptionInstance {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::OptionInstance
    }
}

impl Deref for OptionInstance {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
