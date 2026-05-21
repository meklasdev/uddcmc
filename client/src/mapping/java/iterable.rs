use crate::mapping::java::iterator::Iterator;
use crate::mapping::{JavaObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct Iterable {
    pub jni_ref: GlobalRef,
}

impl Iterable {
    /// Wraps an existing `java.lang.Iterable` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Iterable {
        Iterable { jni_ref }
    }

    pub fn iterator(&self) -> anyhow::Result<Iterator> {
        mapping().in_frame(|| {
            let iterator_obj = mapping()
                .call_method(
                    MinecraftClassType::Iterable,
                    self.jni_ref.as_obj(),
                    "iterator",
                    &[],
                )?
                .l()?;
            Ok(Iterator::new(mapping().new_global_ref(iterator_obj)?))
        })
    }
}

impl JavaObject for Iterable {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::Iterable
    }
}

impl Deref for Iterable {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
