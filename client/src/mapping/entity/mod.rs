use crate::mapping::component::Component;
use crate::mapping::entity::living::LivingEntity;
use crate::mapping::math::Vec3;
use crate::mapping::{FieldType, JavaObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};
use std::ops::Deref;

pub mod living;
pub mod mob;
pub mod player;

#[derive(Debug, Clone)]
pub struct Entity {
    pub jni_ref: GlobalRef,
}

impl Entity {
    /// Wraps an existing `Entity` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Entity {
        Entity { jni_ref }
    }

    /// Views this entity as a [`LivingEntity`] — valid for players and mobs.
    pub fn as_living(&self) -> LivingEntity {
        LivingEntity::new(self.jni_ref.clone())
    }

    /// The entity's network id.
    pub fn id(&self) -> anyhow::Result<i32> {
        Ok(mapping()
            .call_method(
                MinecraftClassType::Entity,
                self.jni_ref.as_obj(),
                "getId",
                &[],
            )?
            .i()?)
    }

    /// The entity's world position.
    pub fn get_position(&self) -> anyhow::Result<Vec3> {
        mapping().in_frame(|| {
            let vec3 = mapping()
                .call_method(
                    MinecraftClassType::Entity,
                    self.jni_ref.as_obj(),
                    "position",
                    &[],
                )?
                .l()?;
            Vec3::read(&vec3)
        })
    }

    /// Squared distance from this entity to a world point.
    pub fn distance_to_sqr(&self, x: f64, y: f64, z: f64) -> anyhow::Result<f64> {
        Ok(mapping()
            .call_method(
                MinecraftClassType::Entity,
                self.jni_ref.as_obj(),
                "distanceToSqr",
                &[JValue::Double(x), JValue::Double(y), JValue::Double(z)],
            )?
            .d()?)
    }

    /// Collision-box width.
    pub fn bb_width(&self) -> anyhow::Result<f32> {
        Ok(mapping()
            .call_method(
                MinecraftClassType::Entity,
                self.jni_ref.as_obj(),
                "getBbWidth",
                &[],
            )?
            .f()?)
    }

    /// Collision-box height.
    pub fn bb_height(&self) -> anyhow::Result<f32> {
        Ok(mapping()
            .call_method(
                MinecraftClassType::Entity,
                self.jni_ref.as_obj(),
                "getBbHeight",
                &[],
            )?
            .f()?)
    }

    /// Whether the entity is currently sprinting.
    pub fn is_sprinting(&self) -> anyhow::Result<bool> {
        Ok(mapping()
            .call_method(
                MinecraftClassType::Entity,
                self.jni_ref.as_obj(),
                "isSprinting",
                &[],
            )?
            .z()?)
    }

    pub fn set_invulnerable(&self, value: bool) -> anyhow::Result<()> {
        mapping().call_method(
            MinecraftClassType::Entity,
            self.jni_ref.as_obj(),
            "setInvulnerable",
            &[JValue::from(value)],
        )?;

        Ok(())
    }

    /// Sets the entity's yaw (`yRot`), in degrees.
    pub fn set_yaw(&self, yaw: f32) -> anyhow::Result<()> {
        mapping().set_field(
            MinecraftClassType::Entity,
            self.jni_ref.as_obj(),
            "yRot",
            FieldType::Float,
            JValue::Float(yaw),
        )
    }

    /// Sets the entity's pitch (`xRot`), in degrees.
    pub fn set_pitch(&self, pitch: f32) -> anyhow::Result<()> {
        mapping().set_field(
            MinecraftClassType::Entity,
            self.jni_ref.as_obj(),
            "xRot",
            FieldType::Float,
            JValue::Float(pitch),
        )
    }

    pub fn get_fall_distance(&self) -> anyhow::Result<f64> {
        Ok(mapping()
            .get_field(
                MinecraftClassType::Entity,
                self.jni_ref.as_obj(),
                "fallDistance",
                FieldType::Double,
            )?
            .d()?)
    }

    pub fn reset_fall_distance(&self) -> anyhow::Result<()> {
        Ok(mapping()
            .call_method(
                MinecraftClassType::Entity,
                self.jni_ref.as_obj(),
                "resetFallDistance",
                &[],
            )?
            .v()?)
    }

    /// The entity's display name, as a [`Component`].
    pub fn get_name(&self) -> anyhow::Result<Component> {
        mapping().in_frame(|| {
            let component = mapping()
                .call_method(
                    MinecraftClassType::Entity,
                    self.jni_ref.as_obj(),
                    "getName",
                    &[],
                )?
                .l()?;
            Ok(Component::new(mapping().new_global_ref(component)?))
        })
    }

    pub fn get_tick_count(&self) -> anyhow::Result<i32> {
        Ok(mapping()
            .get_field(
                MinecraftClassType::Entity,
                self.jni_ref.as_obj(),
                "tickCount",
                FieldType::Int,
            )?
            .i()?)
    }
}

impl JavaObject for Entity {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::Entity
    }
}

impl Deref for Entity {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
