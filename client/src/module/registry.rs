//! The module registry — every registered module, keyed by name.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use log::error;

use crate::module::{Module, ModuleType};

/// A shared, lockable handle to one module.
pub type ModuleHandle = Arc<Mutex<ModuleType>>;

/// Holds every registered module. Backed by a `DashMap` so the render thread
/// can read it each frame without contending on a single global lock.
#[derive(Default)]
pub struct ModuleRegistry {
    modules: DashMap<String, ModuleHandle>,
}

impl ModuleRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a module under its declared name.
    pub fn register<M>(&self, module: M)
    where
        M: Module + Send + Sync + 'static,
    {
        let module: ModuleType = Box::new(module);
        let name = module.get_module_data().name.clone();
        self.modules.insert(name, Arc::new(Mutex::new(module)));
    }

    /// A handle to one module by name.
    pub fn get(&self, name: &str) -> Option<ModuleHandle> {
        self.modules
            .get(name)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Handles to every module. Snapshotted, so the caller holds no shard
    /// locks while it works with the modules.
    pub fn handles(&self) -> Vec<ModuleHandle> {
        self.modules
            .iter()
            .map(|entry| Arc::clone(entry.value()))
            .collect()
    }

    /// Every module keyed by name — an owned snapshot.
    pub fn by_name(&self) -> HashMap<String, ModuleHandle> {
        self.modules
            .iter()
            .map(|entry| (entry.key().clone(), Arc::clone(entry.value())))
            .collect()
    }

    /// Ticks every enabled module once. A module whose tick fails is stopped
    /// rather than aborting the whole pass.
    pub fn tick(&self) {
        for handle in self.handles() {
            let Ok(module) = handle.lock() else {
                continue;
            };
            if !module.get_module_data().enabled {
                continue;
            }
            if let Err(e) = module.on_tick() {
                let name = &module.get_module_data().name;
                error!("module '{name}' tick failed, stopping it: {e}");
                if let Err(e) = module.on_stop() {
                    error!("module '{name}' also failed to stop: {e}");
                }
            }
        }
    }
}
