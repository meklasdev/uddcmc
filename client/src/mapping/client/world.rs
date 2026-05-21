use crate::mapping::block_entity::BlockEntity;
use crate::mapping::entity::Entity;
use crate::mapping::java::iterable::Iterable;
use crate::mapping::{JavaObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};
use std::ops::Deref;

#[derive(Debug)]
pub struct World {
    jni_ref: GlobalRef,
}

impl World {
    /// Wraps an existing `Level` JVM object.
    pub fn new(jni_ref: GlobalRef) -> World {
        World { jni_ref }
    }

    /// Every entity the client is currently rendering.
    pub fn get_entities(&self) -> anyhow::Result<Vec<Entity>> {
        mapping().in_frame(|| {
            let iterable_obj = mapping()
                .call_method(
                    MinecraftClassType::Level,
                    self.jni_ref.as_obj(),
                    "entitiesForRendering",
                    &[],
                )?
                .l()?;
            let iterable = Iterable::new(mapping().new_global_ref(iterable_obj)?);

            let iterator = iterable.iterator()?;
            let mut entities = Vec::new();
            while iterator.has_next()? {
                entities.push(Entity::new(iterator.next()?));
            }
            Ok(entities)
        })
    }

    /// The chunk at chunk-grid coordinates `(x, z)`, or `Ok(None)` when it is
    /// not loaded.
    pub fn get_chunk(&self, x: i32, z: i32) -> anyhow::Result<Option<LevelChunk>> {
        mapping().in_frame(|| {
            let chunk = mapping()
                .call_method(
                    MinecraftClassType::LevelReader,
                    self.jni_ref.as_obj(),
                    "getChunk",
                    &[JValue::Int(x), JValue::Int(z)],
                )?
                .l()?;
            if chunk.is_null() {
                return Ok(None);
            }
            Ok(Some(LevelChunk::new(mapping().new_global_ref(chunk)?)))
        })
    }
}

impl JavaObject for World {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::Level
    }
}

impl Deref for World {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}

/// A loaded `LevelChunk`.
#[derive(Debug)]
pub struct LevelChunk {
    pub jni_ref: GlobalRef,
}

impl LevelChunk {
    /// Wraps an existing `LevelChunk` JVM object.
    pub fn new(jni_ref: GlobalRef) -> LevelChunk {
        LevelChunk { jni_ref }
    }

    /// Every block entity currently in this chunk.
    pub fn get_block_entities(&self) -> anyhow::Result<Vec<BlockEntity>> {
        mapping().in_frame(|| {
            let map = mapping()
                .call_method(
                    MinecraftClassType::LevelChunk,
                    self.jni_ref.as_obj(),
                    "getBlockEntities",
                    &[],
                )?
                .l()?;
            if map.is_null() {
                return Ok(Vec::new());
            }

            let values = mapping()
                .call_method(MinecraftClassType::Map, &map, "values", &[])?
                .l()?;
            let iterable = Iterable::new(mapping().new_global_ref(values)?);
            let iterator = iterable.iterator()?;

            let mut block_entities = Vec::new();
            while iterator.has_next()? {
                block_entities.push(BlockEntity::new(iterator.next()?));
            }
            Ok(block_entities)
        })
    }
}

impl JavaObject for LevelChunk {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::LevelChunk
    }
}

impl Deref for LevelChunk {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
