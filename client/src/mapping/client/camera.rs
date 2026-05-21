//! Wrapper for Minecraft's render `Camera`.

use crate::mapping::math::Vec3;
use crate::mapping::{FieldType, JavaObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;
use std::ops::Deref;

/// Minecraft's render `Camera`.
///
/// Position and rotation are read from the plain fields rather than getter
/// methods: field names are far stabler than method names across versions.
#[derive(Debug, Clone)]
pub struct Camera {
    pub jni_ref: GlobalRef,
}

impl Camera {
    /// Wraps an existing `Camera` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Camera {
        Camera { jni_ref }
    }

    /// World-space position of the camera.
    pub fn position(&self) -> anyhow::Result<Vec3> {
        mapping().in_frame(|| {
            let vec3 = mapping()
                .get_field(
                    MinecraftClassType::Camera,
                    self.jni_ref.as_obj(),
                    "position",
                    FieldType::Object(MinecraftClassType::Vec3),
                )?
                .l()?;
            Vec3::read(&vec3)
        })
    }

    /// Yaw, in degrees (the `yRot` field).
    pub fn yaw(&self) -> anyhow::Result<f32> {
        Ok(mapping()
            .get_field(
                MinecraftClassType::Camera,
                self.jni_ref.as_obj(),
                "yRot",
                FieldType::Float,
            )?
            .f()?)
    }

    /// Pitch, in degrees (the `xRot` field).
    pub fn pitch(&self) -> anyhow::Result<f32> {
        Ok(mapping()
            .get_field(
                MinecraftClassType::Camera,
                self.jni_ref.as_obj(),
                "xRot",
                FieldType::Float,
            )?
            .f()?)
    }
}

impl JavaObject for Camera {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::Camera
    }
}

impl Deref for Camera {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
