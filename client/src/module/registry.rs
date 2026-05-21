//! The module registry — every registered module, keyed by [`ModuleId`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use log::error;

use crate::module::{KeyboardKey, Module, ModuleId, ModuleSetting, ModuleType};
use crate::net::packet::{Packet, PacketAction};

/// A shared, lockable handle to one module.
pub type ModuleHandle = Arc<Mutex<ModuleType>>;

/// A module's factory defaults — the keybind and settings it was registered
/// with. Kept so "Reset Settings" can restore them.
struct ModuleDefaults {
    key_bind: KeyboardKey,
    settings: Vec<ModuleSetting>,
}

/// Holds every registered module. Backed by a `DashMap` so the render thread
/// can read it each frame without contending on a single global lock.
#[derive(Default)]
pub struct ModuleRegistry {
    modules: DashMap<ModuleId, ModuleHandle>,
    /// Factory defaults captured at registration, keyed like `modules`.
    defaults: DashMap<ModuleId, ModuleDefaults>,
}

impl ModuleRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a module under its [`ModuleId`], capturing its factory
    /// defaults so they can be restored later.
    pub fn register<M>(&self, module: M)
    where
        M: Module + Send + Sync + 'static,
    {
        let module: ModuleType = Box::new(module);
        let data = module.get_module_data();
        let id = data.id;
        self.defaults.insert(
            id,
            ModuleDefaults {
                key_bind: data.key_bind,
                settings: data.settings.clone(),
            },
        );
        self.modules.insert(id, Arc::new(Mutex::new(module)));
    }

    /// Restores every module to its factory defaults: default keybind, default
    /// setting values, and disabled. Backs the GUI's "Reset Settings" button.
    pub fn reset_settings(&self) {
        for entry in self.modules.iter() {
            let Some(defaults) = self.defaults.get(entry.key()) else {
                continue;
            };
            let Ok(mut module) = entry.value().lock() else {
                continue;
            };
            if module.get_module_data().enabled {
                let _ = module.on_stop();
            }
            let data = module.get_module_data_mut();
            data.key_bind = defaults.key_bind;
            data.settings = defaults.settings.clone();
            data.enabled = false;
        }
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
    /// the connection's packet dispatch, on the Netty thread. Returns
    /// [`PacketAction::Cancel`] as soon as any module asks to drop the packet
    /// (the remaining modules are then skipped), otherwise
    /// [`PacketAction::Forward`].
    pub fn handle_packet(&self, packet: &mut Packet) -> PacketAction {
        for handle in self.handles() {
            let Ok(module) = handle.lock() else {
                continue;
            };
            if !module.get_module_data().enabled {
                continue;
            }
            if module.handle_packet(packet) == PacketAction::Cancel {
                return PacketAction::Cancel;
            }
        }
        PacketAction::Forward
    }
}
