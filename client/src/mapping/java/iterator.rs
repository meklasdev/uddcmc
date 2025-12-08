use crate::mapping::{GameContext, MinecraftClassType};
use jni::objects::GlobalRef;
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct Iterator {
    pub jni_ref: GlobalRef,
}

impl GameContext for Iterator {}

impl Iterator {
    pub fn has_next(&self) -> anyhow::Result<bool> {
        let mapping = self.mapping();

        Ok(mapping
            .call_method(
                MinecraftClassType::Iterator,
                self.jni_ref.as_obj(),
                "hasNext",
                &[],
            )?
            .z()?)
    }

    pub fn next(&self) -> anyhow::Result<GlobalRef> {
        let mapping = self.mapping();

        let next_obj = mapping
            .call_method(
                MinecraftClassType::Iterator,
                self.jni_ref.as_obj(),
                "next",
                &[],
            )?
            .l()?;

        mapping.new_global_ref(next_obj)
    }
}

impl Deref for Iterator {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
