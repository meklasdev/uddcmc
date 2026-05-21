//! The module registry — every registered module, keyed by [`ModuleId`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use log::error;

use crate::module::{Module, ModuleId, ModuleType};
use crate::net::packet::Packet;

/// A shared, lockable handle to one module.
pub type ModuleHandle = Arc<Mutex<ModuleType>>;

/// Holds every registered module. Backed by a `DashMap` so the render thread
/// can read it each frame without contending on a single global lock.
#[derive(Default)]
pub struct ModuleRegistry {
    modules: DashMap<ModuleId, ModuleHandle>,
}

impl ModuleRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a module under its [`ModuleId`].
    pub fn register<M>(&self, module: M)
    where
        M: Module + Send + Sync + 'static,
    {
        let module: ModuleType = Box::new(module);
        let id = module.get_module_data().id;
        self.modules.insert(id, Arc::new(Mutex::new(module)));
    }

    /// A handle to one module by id.
    pub fn get(&self, id: ModuleId) -> Option<ModuleHandle> {
        self.modules.get(&id).map(|entry| Arc::clone(entry.value()))
    }

    /// Handles to every module. Snapshotted, so the caller holds no shard
    /// locks while it works with the modules.
    pub fn handles(&self) -> Vec<ModuleHandle> {
        self.modules
            .iter()
            .map(|entry| Arc::clone(entry.value()))
            .collect()
    }

    /// Every module keyed by id — an owned snapshot.
    pub fn by_id(&self) -> HashMap<ModuleId, ModuleHandle> {
        self.modules
            .iter()
            .map(|entry| (*entry.key(), Arc::clone(entry.value())))
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
                let name = module.get_module_data().name();
                error!("module '{name}' tick failed, stopping it: {e}");
                if let Err(e) = module.on_stop() {
                    error!("module '{name}' also failed to stop: {e}");
                }
            }
        }
    }

    /// Offers `packet` to every enabled module's `handle_packet`. Called from
    /// the connection's packet dispatch, on the Netty thread.
    pub fn handle_packet(&self, packet: &mut Packet) {
        for handle in self.handles() {
            let Ok(module) = handle.lock() else {
                continue;
            };
            if module.get_module_data().enabled {
                module.handle_packet(packet);
            }
        }
    }
}
