use crate::mapping::entity::Entity;
use crate::mapping::java::iterable::Iterable;
use crate::mapping::MinecraftClassType;
use crate::state::mapping;
use jni::objects::GlobalRef;
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
