use crate::mapping::method::MethodName;
use crate::mapping::{JavaObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;
use jni::sys::jlong;
use std::ops::Deref;

#[derive(Debug)]
pub struct Window {
    pub jni_ref: GlobalRef,
}

impl Window {
    pub fn new(minecraft: &GlobalRef) -> anyhow::Result<Window> {
        mapping().in_frame(|| {
            let window_obj = mapping()
                .call_method(
                    MinecraftClassType::Minecraft,
                    minecraft.as_obj(),
                    "getWindow",
                    &[],
                )?
                .l()?;
            Ok(Window {
                jni_ref: mapping().new_global_ref(window_obj)?,
            })
        })
    }

    /// The native GLFW window handle.
    pub fn get_window(&self) -> anyhow::Result<jlong> {
        let mapping = mapping();
        Ok(mapping
            .call_method(
                MinecraftClassType::Window,
                self.jni_ref.as_obj(),
                MethodName::WindowGetWindow.get_name(mapping.get_version()),
                &[],
            )?
            .j()?)
    }
}

impl JavaObject for Window {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::Window
    }
}

impl Deref for Window {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
