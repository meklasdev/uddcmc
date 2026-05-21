//! Wrapper for Minecraft's `Component` — a piece of rich (chat / display) text.

use crate::mapping::MappedObject;
use crate::state::mapping;
use jni::objects::GlobalRef;

/// A Minecraft `Component`.
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Component)]
pub struct Component {
    jni_ref: GlobalRef,
}

impl Component {
    /// Wraps an existing `Component` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Component {
        Component { jni_ref }
    }

    /// The component flattened to plain text.
    pub fn get_string(&self) -> anyhow::Result<String> {
        self.in_frame(|| {
            let string = self.call_method("getString", &[])?.l()?;
            mapping().get_string(string)
        })
    }
}
