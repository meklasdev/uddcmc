use crate::mapping::entity::Entity;
use crate::mapping::java::iterable::Iterable;
use crate::mapping::{FieldType, GameContext, Mapping, MinecraftClassType};
use jni::objects::GlobalRef;
use std::ops::Deref;

#[derive(Debug)]
pub struct World {
    jni_ref: GlobalRef,
}

impl GameContext for World {}

impl World {
    pub fn new(minecraft: &GlobalRef, mapping: &Mapping) -> anyhow::Result<World> {
        let world_obj = mapping
            .get_field(
                MinecraftClassType::Minecraft,
                minecraft.as_obj(),
                "level",
                FieldType::Object(MinecraftClassType::Level, mapping),
            )?
            .l()?;

        Ok(World {
            jni_ref: mapping.new_global_ref(world_obj)?,
        })
    }

    pub fn get_entities(&self) -> anyhow::Result<Vec<Entity>> {
        let mapping = self.mapping();

        let iterable_obj = mapping
            .call_method(
                MinecraftClassType::Level,
                self.jni_ref.as_obj(),
                "entitiesForRendering",
                &[],
            )?
            .l()?;

        let iterable = Iterable {
            jni_ref: mapping.new_global_ref(iterable_obj)?,
        };

        let iterator = iterable.iterator()?;
        let mut entities = Vec::new();

        while iterator.has_next()? {
            let entity_obj = iterator.next()?;
            entities.push(Entity::new(entity_obj));
        }

        Ok(entities)
    }
}

impl Deref for World {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
