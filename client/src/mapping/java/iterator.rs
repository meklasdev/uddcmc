use crate::mapping::MinecraftClassType;
use crate::state::mapping;
use jni::objects::GlobalRef;
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct Iterator {
    pub jni_ref: GlobalRef,
}

impl Iterator {
    pub fn has_next(&self) -> anyhow::Result<bool> {
        Ok(mapping()
            .call_method(
                MinecraftClassType::Iterator,
                self.jni_ref.as_obj(),
                "hasNext",
                &[],
            )?
            .z()?)
    }

    pub fn next(&self) -> anyhow::Result<GlobalRef> {
        let next_obj = mapping()
            .call_method(
                MinecraftClassType::Iterator,
                self.jni_ref.as_obj(),
                "next",
                &[],
            )?
            .l()?;

        mapping().new_global_ref(next_obj)
    }
}

impl Deref for Iterator {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
