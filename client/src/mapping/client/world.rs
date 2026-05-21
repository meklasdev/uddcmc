use crate::mapping::block_entity::BlockEntity;
use crate::mapping::entity::Entity;
use crate::mapping::java::iterable::Iterable;
use crate::mapping::{MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};

#[derive(Debug, MappedObject)]
#[mapped(class = Level)]
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
        self.in_frame(|| {
            let iterable_obj = self.call_method("entitiesForRendering", &[])?.l()?;
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
        self.in_frame(|| {
            let chunk = self
                .call_method("getChunk", &[JValue::Int(x), JValue::Int(z)])?
                .l()?;
            if chunk.is_null() {
                return Ok(None);
            }
            Ok(Some(LevelChunk::new(mapping().new_global_ref(chunk)?)))
        })
    }
}

/// A loaded `LevelChunk`.
#[derive(Debug, MappedObject)]
#[mapped(class = LevelChunk)]
pub struct LevelChunk {
    jni_ref: GlobalRef,
}

impl LevelChunk {
    /// Wraps an existing `LevelChunk` JVM object.
    pub fn new(jni_ref: GlobalRef) -> LevelChunk {
        LevelChunk { jni_ref }
    }

    /// Every block entity currently in this chunk.
    pub fn get_block_entities(&self) -> anyhow::Result<Vec<BlockEntity>> {
        self.in_frame(|| {
            let map = self.call_method("getBlockEntities", &[])?.l()?;
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
