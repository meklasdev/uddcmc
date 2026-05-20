use crate::mapping::entity::Entity;
use crate::mapping::java::iterable::Iterable;
use crate::mapping::{FieldType, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;
use std::ops::Deref;

#[derive(Debug)]
pub struct World {
    jni_ref: GlobalRef,
}

impl World {
    pub fn new(minecraft: &GlobalRef) -> anyhow::Result<World> {
        let world_obj = mapping()
            .get_field(
                MinecraftClassType::Minecraft,
                minecraft.as_obj(),
                "level",
                FieldType::Object(MinecraftClassType::Level),
            )?
            .l()?;

        Ok(World {
            jni_ref: mapping().new_global_ref(world_obj)?,
        })
    }

    pub fn get_entities(&self) -> anyhow::Result<Vec<Entity>> {
        let iterable_obj = mapping()
            .call_method(
                MinecraftClassType::Level,
                self.jni_ref.as_obj(),
                "entitiesForRendering",
                &[],
            )?
            .l()?;

        let iterable = Iterable {
            jni_ref: mapping().new_global_ref(iterable_obj)?,
        };

        let iterator = iterable.iterator()?;
        let mut entities = Vec::new();

        while iterator.has_next()? {
            entities.push(Entity::new(iterator.next()?));
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
