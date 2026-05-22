//! Wrappers for Minecraft's `Options` and `OptionInstance`.

use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};

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

    /// The brightness (gamma) option.
    pub fn gamma(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field("gamma", FieldType::Object(MinecraftClassType::OptionInstance))?
                .l()?;
            Ok(OptionInstance::new(mapping().new_global_ref(option)?))
        })
    }

    /// The mouse-sensitivity option.
    pub fn sensitivity(&self) -> anyhow::Result<OptionInstance> {
        self.in_frame(|| {
            let option = self
                .get_field(
                    "sensitivity",
                    FieldType::Object(MinecraftClassType::OptionInstance),
                )?
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

    /// The current value, read as a `double` — the option's boxed value is
    /// unwrapped through `Double.doubleValue()`.
    pub fn get_double(&self) -> anyhow::Result<f64> {
        self.in_frame(|| {
            let value = self.call_method("get", &[])?.l()?;
            Ok(mapping()
                .call_method(MinecraftClassType::Double, &value, "doubleValue", &[])?
                .d()?)
        })
    }

    /// Overwrites the option's stored `double` value, writing the private
    /// `value` field directly.
    ///
    /// Unlike `OptionInstance.set`, this bypasses the option's `validateValue`
    /// codec — which is what lets gamma be pushed past its normal `0..1` slider
    /// range, the basis of the Fullbright module.
    pub fn force_double(&self, value: f64) -> anyhow::Result<()> {
        self.in_frame(|| {
            let mut env = mapping().get_env()?;
            let double_class = env.find_class("java/lang/Double")?;
            let boxed = env.new_object(&double_class, "(D)V", &[JValue::Double(value)])?;
            self.set_field(
                "value",
                FieldType::Object(MinecraftClassType::Object),
                JValue::Object(&boxed),
            )
        })
    }
}
