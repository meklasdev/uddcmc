//! Plain-value wrappers for Minecraft's immutable coordinate types.
//!
//! `Vec3` and `BlockPos` are immutable value classes in Minecraft, so they are
//! read **once** into plain Rust fields rather than kept as live JNI handles:
//! the accessors are then infallible, allocate no JNI references and cannot go
//! stale mid-use. Re-read from the source object whenever a fresh value is
//! needed.

use crate::mapping::{FieldType, MinecraftClassType as Cls};
use crate::state::mapping;
use jni::objects::{JObject, JValue};
use jni::JNIEnv;

/// An immutable 3D vector — a snapshot of Minecraft's `Vec3`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    /// A vector from explicit components.
    pub const fn new(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 { x, y, z }
    }

    /// Snapshots a JVM `Vec3` object by reading its `x` / `y` / `z` fields.
    pub fn read(obj: &JObject) -> anyhow::Result<Vec3> {
        let mapping = mapping();
        Ok(Vec3 {
            x: mapping
                .get_field(Cls::Vec3, obj, "x", FieldType::Double)?
                .d()?,
            y: mapping
                .get_field(Cls::Vec3, obj, "y", FieldType::Double)?
                .d()?,
            z: mapping
                .get_field(Cls::Vec3, obj, "z", FieldType::Double)?
                .d()?,
        })
    }

    pub const fn x(&self) -> f64 {
        self.x
    }

    pub const fn y(&self) -> f64 {
        self.y
    }

    pub const fn z(&self) -> f64 {
        self.z
    }

    /// Builds a fresh JVM `Vec3` object from this value.
    pub fn to_java<'e>(self, env: &mut JNIEnv<'e>) -> anyhow::Result<JObject<'e>> {
        let class = mapping().resolve_class(env, Cls::Vec3.get_name())?;
        Ok(env.new_object(
            &class,
            "(DDD)V",
            &[
                JValue::Double(self.x),
                JValue::Double(self.y),
                JValue::Double(self.z),
            ],
        )?)
    }
}

/// An immutable block coordinate — a snapshot of Minecraft's `BlockPos`
/// (a `Vec3i`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockPos {
    x: i32,
    y: i32,
    z: i32,
}

impl BlockPos {
    /// A block position from explicit components.
    pub const fn new(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    /// Snapshots a JVM `BlockPos` via its `getX` / `getY` / `getZ` accessors.
    pub fn read(obj: &JObject) -> anyhow::Result<BlockPos> {
        let mapping = mapping();
        Ok(BlockPos {
            x: mapping.call_method(Cls::BlockPos, obj, "getX", &[])?.i()?,
            y: mapping.call_method(Cls::BlockPos, obj, "getY", &[])?.i()?,
            z: mapping.call_method(Cls::BlockPos, obj, "getZ", &[])?.i()?,
        })
    }

    pub const fn x(&self) -> i32 {
        self.x
    }

    pub const fn y(&self) -> i32 {
        self.y
    }

    pub const fn z(&self) -> i32 {
        self.z
    }

    /// This position offset by `(dx, dy, dz)`.
    pub const fn offset(&self, dx: i32, dy: i32, dz: i32) -> BlockPos {
        BlockPos::new(self.x + dx, self.y + dy, self.z + dz)
    }

    /// Builds a fresh JVM `BlockPos` object from this value.
    pub fn to_java<'e>(self, env: &mut JNIEnv<'e>) -> anyhow::Result<JObject<'e>> {
        let class = mapping().resolve_class(env, Cls::BlockPos.get_name())?;
        Ok(env.new_object(
            &class,
            "(III)V",
            &[
                JValue::Int(self.x),
                JValue::Int(self.y),
                JValue::Int(self.z),
            ],
        )?)
    }
}
