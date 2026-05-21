//! Wrapper for Minecraft's `GameRenderer`.

use crate::mapping::client::camera::Camera;
use crate::mapping::{JavaObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;
use std::ops::Deref;

/// Minecraft's `GameRenderer`.
#[derive(Debug)]
pub struct GameRenderer {
    pub jni_ref: GlobalRef,
}

impl GameRenderer {
    /// Wraps an existing `GameRenderer` JVM object.
    pub fn new(jni_ref: GlobalRef) -> GameRenderer {
        GameRenderer { jni_ref }
    }

    /// The main render [`Camera`].
    pub fn get_main_camera(&self) -> anyhow::Result<Camera> {
        mapping().in_frame(|| {
            let camera = mapping()
                .call_method(
                    MinecraftClassType::GameRenderer,
                    self.jni_ref.as_obj(),
                    "getMainCamera",
                    &[],
                )?
                .l()?;
            Ok(Camera::new(mapping().new_global_ref(camera)?))
        })
    }
}

impl JavaObject for GameRenderer {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::GameRenderer
    }
}

impl Deref for GameRenderer {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
