//! Wrapper for Minecraft's render `Camera`.

use crate::mapping::math::Vec3;
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use jni::objects::GlobalRef;

/// Minecraft's render `Camera`.
///
/// Position and rotation are read from the plain fields rather than getter
/// methods: field names are far stabler than method names across versions.
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Camera)]
pub struct Camera {
    jni_ref: GlobalRef,
}

impl Camera {
    /// Wraps an existing `Camera` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Camera {
        Camera { jni_ref }
    }

    /// World-space position of the camera.
    pub fn position(&self) -> anyhow::Result<Vec3> {
        self.in_frame(|| {
            let vec3 = self
                .get_field("position", FieldType::Object(MinecraftClassType::Vec3))?
                .l()?;
            Vec3::read(&vec3)
        })
    }

    /// Yaw, in degrees (the `yRot` field).
    pub fn yaw(&self) -> anyhow::Result<f32> {
        Ok(self.get_field("yRot", FieldType::Float)?.f()?)
    }

    /// Pitch, in degrees (the `xRot` field).
    pub fn pitch(&self) -> anyhow::Result<f32> {
        Ok(self.get_field("xRot", FieldType::Float)?.f()?)
    }

    /// Vertical FOV, in degrees, actually used to render the current frame.
    /// `Camera.fov` is recomputed each frame by `setupAndRender`, already
    /// folding in the player's `getFieldOfViewModifier`, `fovEffectScale`,
    /// and the water / lava / dying tweaks — so the ESP just reads it back
    /// instead of recomputing the formula.
    pub fn fov(&self) -> anyhow::Result<f32> {
        Ok(self.get_field("fov", FieldType::Float)?.f()?)
    }
}
