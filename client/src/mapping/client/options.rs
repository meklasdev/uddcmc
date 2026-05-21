//! Wrappers for Minecraft's `Options` and `OptionInstance`.

use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;

/// Minecraft's game `Options`.
#[derive(Debug, MappedObject)]
#[mapped(class = Options)]
pub struct Options {
    jni_ref: GlobalRef,
}

impl Options {
    /// Wraps an existing `Options` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Options {
        Options { jni_ref }
    }

    /// The field-of-view option.
    pub fn fov(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field("fov", FieldType::Object(MinecraftClassType::OptionInstance))?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }
}

/// A single Minecraft `OptionInstance` — one configurable game option.
#[derive(Debug, MappedObject)]
#[mapped(class = OptionInstance)]
pub struct OptionInstance {
    jni_ref: GlobalRef,
}

impl OptionInstance {
    /// Wraps an existing `OptionInstance` JVM object.
    pub fn new(jni_ref: GlobalRef) -> OptionInstance {
        OptionInstance { jni_ref }
    }

    /// The current value, read as an `int` — the option's boxed value is
    /// unwrapped through `Integer.intValue()`.
    pub fn get_int(&self) -> anyhow::Result<i32> {
        self.in_frame(|| {
            let value = self.call_method("get", &[])?.l()?;
            Ok(mapping()
                .call_method(MinecraftClassType::Integer, &value, "intValue", &[])?
                .i()?)
        })
    }
}
