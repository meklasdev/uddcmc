//! Wrapper for Minecraft's `Component` — a piece of rich (chat / display) text.

use crate::mapping::{JavaObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;
use std::ops::Deref;

/// A Minecraft `Component`.
#[derive(Debug, Clone)]
pub struct Component {
    pub jni_ref: GlobalRef,
}

impl Component {
    /// Wraps an existing `Component` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Component {
        Component { jni_ref }
    }

    /// The component flattened to plain text.
    pub fn get_string(&self) -> anyhow::Result<String> {
        mapping().in_frame(|| {
            let string = mapping()
                .call_method(
                    MinecraftClassType::Component,
                    self.jni_ref.as_obj(),
                    "getString",
                    &[],
                )?
                .l()?;
            mapping().get_string(string)
        })
    }
}

impl JavaObject for Component {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::Component
    }
}

impl Deref for Component {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
