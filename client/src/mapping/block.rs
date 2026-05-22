//! Block-interaction support — the six block-face directions.

use crate::mapping::{FieldType, MinecraftClassType};
use crate::state::mapping;
use jni::objects::JObject;

/// One of Minecraft's six block faces, mirroring `net.minecraft.core.Direction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Direction {
    /// All six faces.
    pub const ALL: [Direction; 6] = [
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ];

    /// The unit block offset along this face.
    pub fn offset(self) -> (i32, i32, i32) {
        match self {
            Direction::Down => (0, -1, 0),
            Direction::Up => (0, 1, 0),
            Direction::North => (0, 0, -1),
            Direction::South => (0, 0, 1),
            Direction::West => (-1, 0, 0),
            Direction::East => (1, 0, 0),
        }
    }

    /// The opposing face.
    pub fn opposite(self) -> Direction {
        match self {
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
            Direction::East => Direction::West,
        }
    }

    /// The name of the matching `Direction` enum constant.
    fn jvm_name(self) -> &'static str {
        match self {
            Direction::Down => "DOWN",
            Direction::Up => "UP",
            Direction::North => "NORTH",
            Direction::South => "SOUTH",
            Direction::West => "WEST",
            Direction::East => "EAST",
        }
    }

    /// The JVM `Direction` enum value for this face. Use only inside a JNI
    /// local-reference frame — the returned reference is frame-scoped.
    pub fn to_java(self) -> anyhow::Result<JObject<'static>> {
        Ok(mapping()
            .get_static_field(
                MinecraftClassType::Direction,
                self.jvm_name(),
                FieldType::Object(MinecraftClassType::Direction),
            )?
            .l()?)
    }
}
