//! Rust value-snapshot of `ServerboundMovePlayerPacket`.
//!
//! The packet is an abstract class with four subclasses — `Pos`, `PosRot`,
//! `Rot`, `StatusOnly` — chosen by which of position / rotation it carries.
//! [`MovePlayerPacket::read`] snapshots any of them; [`MovePlayerPacket::to_java`]
//! rebuilds the matching subclass.

use crate::state::mapping;
use jni::objects::{JObject, JValue};
use jni::sys::jobject;
use jni::JNIEnv;

/// Binary name of the abstract `ServerboundMovePlayerPacket`.
pub const CLASS: &str = "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket";

const POS: &str = "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket$Pos";
const POS_ROT: &str = "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket$PosRot";
const ROT: &str = "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket$Rot";
const STATUS_ONLY: &str =
    "net/minecraft/network/protocol/game/ServerboundMovePlayerPacket$StatusOnly";

/// A snapshot of a `ServerboundMovePlayerPacket`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovePlayerPacket {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
    pub horizontal_collision: bool,
    pub has_position: bool,
    pub has_rotation: bool,
}

impl MovePlayerPacket {
    /// Reads a `ServerboundMovePlayerPacket` JVM object into a snapshot.
    pub fn read(env: &mut JNIEnv, packet: &JObject) -> anyhow::Result<MovePlayerPacket> {
        Ok(MovePlayerPacket {
            x: env
                .call_method(packet, "getX", "(D)D", &[JValue::Double(0.0)])?
                .d()?,
            y: env
                .call_method(packet, "getY", "(D)D", &[JValue::Double(0.0)])?
                .d()?,
            z: env
                .call_method(packet, "getZ", "(D)D", &[JValue::Double(0.0)])?
                .d()?,
            y_rot: env
                .call_method(packet, "getYRot", "(F)F", &[JValue::Float(0.0)])?
                .f()?,
            x_rot: env
                .call_method(packet, "getXRot", "(F)F", &[JValue::Float(0.0)])?
                .f()?,
            on_ground: env.call_method(packet, "isOnGround", "()Z", &[])?.z()?,
            horizontal_collision: env
                .call_method(packet, "horizontalCollision", "()Z", &[])?
                .z()?,
            has_position: env.call_method(packet, "hasPosition", "()Z", &[])?.z()?,
            has_rotation: env.call_method(packet, "hasRotation", "()Z", &[])?.z()?,
        })
    }

    /// Builds the matching `ServerboundMovePlayerPacket` subclass from this
    /// snapshot, returning the new JVM object.
    pub fn to_java(self, env: &mut JNIEnv) -> anyhow::Result<jobject> {
        let on_ground = JValue::Bool(self.on_ground as u8);
        let collision = JValue::Bool(self.horizontal_collision as u8);

        let object = match (self.has_position, self.has_rotation) {
            (true, true) => {
                let class = mapping().resolve_class(env, POS_ROT)?;
                env.new_object(
                    &class,
                    "(DDDFFZZ)V",
                    &[
                        JValue::Double(self.x),
                        JValue::Double(self.y),
                        JValue::Double(self.z),
                        JValue::Float(self.y_rot),
                        JValue::Float(self.x_rot),
                        on_ground,
                        collision,
                    ],
                )?
            }
            (true, false) => {
                let class = mapping().resolve_class(env, POS)?;
                env.new_object(
                    &class,
                    "(DDDZZ)V",
                    &[
                        JValue::Double(self.x),
                        JValue::Double(self.y),
                        JValue::Double(self.z),
                        on_ground,
                        collision,
                    ],
                )?
            }
            (false, true) => {
                let class = mapping().resolve_class(env, ROT)?;
                env.new_object(
                    &class,
                    "(FFZZ)V",
                    &[
                        JValue::Float(self.y_rot),
                        JValue::Float(self.x_rot),
                        on_ground,
                        collision,
                    ],
                )?
            }
            (false, false) => {
                let class = mapping().resolve_class(env, STATUS_ONLY)?;
                env.new_object(&class, "(ZZ)V", &[on_ground, collision])?
            }
        };
        Ok(object.into_raw())
    }
}
