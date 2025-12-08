use crate::mapping::java::iterator::Iterator;
use crate::mapping::{GameContext, MinecraftClassType};
use jni::objects::GlobalRef;
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct Iterable {
    pub jni_ref: GlobalRef,
}

impl GameContext for Iterable {}

impl Iterable {
    pub fn iterator(&self) -> anyhow::Result<Iterator> {
        let mapping = self.mapping();

        let iterator_obj = mapping
            .call_method(
                MinecraftClassType::Iterable,
                self.jni_ref.as_obj(),
                "iterator",
                &[],
            )?
            .l()?;

        Ok(Iterator {
            jni_ref: mapping.new_global_ref(iterator_obj)?,
        })
    }
}

impl Deref for Iterable {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
