use crate::mapping::java::iterator::Iterator;
use crate::mapping::MappedObject;
use crate::state::mapping;
use jni::objects::GlobalRef;

#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Iterable)]
pub struct Iterable {
    jni_ref: GlobalRef,
}

impl Iterable {
    /// Wraps an existing `java.lang.Iterable` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Iterable {
        Iterable { jni_ref }
    }

    pub fn iterator(&self) -> anyhow::Result<Iterator> {
        self.in_frame(|| {
            let iterator_obj = self.call_method("iterator", &[])?.l()?;
            Ok(Iterator::new(mapping().new_global_ref(iterator_obj)?))
        })
    }
}
