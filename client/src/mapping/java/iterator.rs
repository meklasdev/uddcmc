use crate::mapping::MappedObject;
use crate::state::mapping;
use jni::objects::GlobalRef;

#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Iterator)]
pub struct Iterator {
    jni_ref: GlobalRef,
}

impl Iterator {
    /// Wraps an existing `java.util.Iterator` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Iterator {
        Iterator { jni_ref }
    }

    pub fn has_next(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("hasNext", &[])?.z()?)
    }

    pub fn next(&self) -> anyhow::Result<GlobalRef> {
        self.in_frame(|| {
            let next_obj = self.call_method("next", &[])?.l()?;
            mapping().new_global_ref(next_obj)
        })
    }
}
