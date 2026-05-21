use crate::mapping::component::Component;
use crate::mapping::entity::living::LivingEntity;
use crate::mapping::math::Vec3;
use crate::mapping::{FieldType, MappedObject};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};

pub mod living;
pub mod mob;
pub mod player;

#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Entity)]
pub struct Entity {
    jni_ref: GlobalRef,
}

impl Entity {
    /// Wraps an existing `Entity` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Entity {
        Entity { jni_ref }
    }

    /// Views this entity as a [`LivingEntity`], or `None` when it is not a
    /// living entity — calling health methods on a non-living entity would fail.
    pub fn as_living(&self) -> Option<LivingEntity> {
        self.instance_of::<LivingEntity>()
            .then(|| LivingEntity::new(self.jni_ref().clone()))
    }

    /// The entity's network id.
    pub fn id(&self) -> anyhow::Result<i32> {
        Ok(self.call_method("getId", &[])?.i()?)
    }

    /// The entity's world position (feet).
    pub fn get_position(&self) -> anyhow::Result<Vec3> {
        self.in_frame(|| {
            let vec3 = self.call_method("position", &[])?.l()?;
            Vec3::read(&vec3)
        })
    }

    /// The entity's eye position — the origin to aim rotations from.
    pub fn get_eye_position(&self) -> anyhow::Result<Vec3> {
        self.in_frame(|| {
            let vec3 = self.call_method("getEyePosition", &[])?.l()?;
            Vec3::read(&vec3)
        })
    }

    /// The entity's yaw, in degrees.
    pub fn get_yaw(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getYRot", &[])?.f()?)
    }

    /// The entity's pitch, in degrees.
    pub fn get_pitch(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getXRot", &[])?.f()?)
    }

    /// Squared distance from this entity to a world point.
    pub fn distance_to_sqr(&self, x: f64, y: f64, z: f64) -> anyhow::Result<f64> {
        Ok(self
            .call_method(
                "distanceToSqr",
                &[JValue::Double(x), JValue::Double(y), JValue::Double(z)],
            )?
            .d()?)
    }

    /// Collision-box width.
    pub fn bb_width(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getBbWidth", &[])?.f()?)
    }

    /// Collision-box height.
    pub fn bb_height(&self) -> anyhow::Result<f32> {
        Ok(self.call_method("getBbHeight", &[])?.f()?)
    }

    /// Whether the entity is currently sprinting.
    pub fn is_sprinting(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isSprinting", &[])?.z()?)
    }

    pub fn set_invulnerable(&self, value: bool) -> anyhow::Result<()> {
        self.call_method("setInvulnerable", &[JValue::from(value)])?;
        Ok(())
    }

    /// Sets the entity's yaw and pitch, in degrees. The previous-tick rotation
    /// (`yRotO` / `xRotO`) is written too, so Minecraft renders the camera
    /// exactly at this rotation instead of interpolating toward it — the
    /// per-frame rotation system supplies the smoothing itself.
    pub fn set_rotation(&self, yaw: f32, pitch: f32) -> anyhow::Result<()> {
        self.call_method("setYRot", &[JValue::Float(yaw)])?;
        self.call_method("setXRot", &[JValue::Float(pitch)])?;
        self.set_field("yRotO", FieldType::Float, JValue::Float(yaw))?;
        self.set_field("xRotO", FieldType::Float, JValue::Float(pitch))?;
        Ok(())
    }

    pub fn get_fall_distance(&self) -> anyhow::Result<f64> {
        Ok(self.get_field("fallDistance", FieldType::Double)?.d()?)
    }

    pub fn reset_fall_distance(&self) -> anyhow::Result<()> {
        Ok(self.call_method("resetFallDistance", &[])?.v()?)
    }

    /// The entity's display name, as a [`Component`].
    pub fn get_name(&self) -> anyhow::Result<Component> {
        self.in_frame(|| {
            let component = self.call_method("getName", &[])?.l()?;
            Ok(Component::new(mapping().new_global_ref(component)?))
        })
    }

    pub fn get_tick_count(&self) -> anyhow::Result<i32> {
        Ok(self.get_field("tickCount", FieldType::Int)?.i()?)
    }
}
