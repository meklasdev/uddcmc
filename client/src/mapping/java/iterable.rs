use crate::mapping::java::iterator::Iterator;
use crate::mapping::MinecraftClassType;
use crate::state::mapping;
use jni::objects::GlobalRef;
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct Iterable {
    pub jni_ref: GlobalRef,
}

impl Iterable {
    pub fn iterator(&self) -> anyhow::Result<Iterator> {
        let iterator_obj = mapping()
            .call_method(
                MinecraftClassType::Iterable,
                self.jni_ref.as_obj(),
                "iterator",
                &[],
            )?
            .l()?;

        Ok(Iterator {
            jni_ref: mapping().new_global_ref(iterator_obj)?,
        })
    }
}

impl Deref for Iterable {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
