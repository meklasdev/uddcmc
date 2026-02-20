use crate::module::{Module, ModuleType};
use jni::sys::{jsize, JNI_GetCreatedJavaVMs, JNI_OK};
use jni::{JNIEnv, JavaVM};
use log::error;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

#[derive(Debug)]
pub struct DarkClient {
    pub(crate) jvm: Arc<JavaVM>,
    pub(crate) modules: Arc<RwLock<HashMap<String, Arc<Mutex<ModuleType>>>>>,
}

impl DarkClient {
    pub fn instance() -> &'static DarkClient {
        static INSTANCE: OnceLock<Arc<DarkClient>> = OnceLock::new();

        INSTANCE.get_or_init(|| unsafe {
            Arc::new(DarkClient::new().unwrap_or_else(|e| {
                error!("Failed to create DarkClient: {}", e);
                panic!("Failed to create DarkClient: {}", e);
            }))
        })
    }

    pub unsafe fn new() -> anyhow::Result<Self> {
        let mut java_vm: *mut jni::sys::JavaVM = std::ptr::null_mut();
        let mut count: jsize = 0;

        if JNI_GetCreatedJavaVMs(&mut java_vm, 1, &mut count) != JNI_OK || count == 0 {
            return Err(anyhow::anyhow!("Failed to get Java VMs"));
        }

        let java_vm: Arc<JavaVM> = Arc::new(match JavaVM::from_raw(java_vm) {
            Ok(jvm) => jvm,
            Err(_) => return Err(anyhow::anyhow!("Could not get JavaVM")),
        });

        Ok(DarkClient {
            jvm: java_vm,
            modules: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn get_env(&'_ self) -> jni::errors::Result<JNIEnv<'_>> {
        //self.jvm.attach_current_thread()
        self.jvm.attach_current_thread_as_daemon()
    }

    pub fn register_module<M>(&self, module: M)
    where
        M: Module + Send + Sync + 'static,
    {
        let module: ModuleType = Box::new(module);
        let module_name = module.get_module_data().name.clone();

        self.modules
            .write()
            .unwrap()
            .insert(module_name, Arc::new(Mutex::new(module)));
    }

    pub fn tick(&self) {
        let modules = self.modules.read().unwrap();
        for module in modules.values() {
            let module = module.lock().unwrap();
            if module.get_module_data().enabled {
                match module.on_tick() {
                    Ok(_) => {}
                    Err(e) => {
                        error!(
                            "Failed to tick module {}, disabling. {}",
                            module.get_module_data().name,
                            e
                        );
                        match module.on_stop() {
                            Ok(_) => {}
                            Err(_) => {
                                error!(
                                    "Failed to stop module {} after an error when ticking",
                                    module.get_module_data().name
                                );
                                panic!(
                                    "Failed to stop module {} after an error when ticking",
                                    module.get_module_data().name
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

// Module for handling keyboard inputs
pub mod keyboard {
    pub fn start_keyboard_handler() {
        // Keyboard handling is now natively event-driven via GLFW inside graphic::input::my_key_callback
    }

    pub fn stop_keyboard_handler() {}
}
