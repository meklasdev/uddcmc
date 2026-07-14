//! Thread-safe architecture for the Lua-based Scripting Engine.
//!
//! Exposes a registerSetting and event-hook hook-in structure to run
//! custom scripts (like krasnostav_pingspoof.lua) inside the native Rust framework.

use std::sync::{Arc, Mutex};
use crate::module::{Module, ModuleData, ModuleId, KeyboardKey, ModuleCategory};
use crate::net::packet::{Packet, PacketAction};

/// A script module executing dynamic Lua scripts.
pub struct LuaScriptModule {
    pub data: ModuleData,
    pub script_name: String,
    pub packet_send_handler: Arc<Mutex<Option<Box<dyn Fn(&mut Packet) -> PacketAction + Send + Sync>>>>,
    pub update_handler: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
}

impl std::fmt::Debug for LuaScriptModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LuaScriptModule")
            .field("script_name", &self.script_name)
            .field("data", &self.data)
            .finish()
    }
}

impl LuaScriptModule {
    pub fn new(name: &str, description: &str, category: ModuleCategory) -> Self {
        Self {
            data: ModuleData {
                id: ModuleId::Freecam, // Reuse/placeholder or custom identifier mapped dynamically
                description: description.to_string(),
                category,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: Vec::new(),
            },
            script_name: name.to_string(),
            packet_send_handler: Arc::new(Mutex::new(None)),
            update_handler: Arc::new(Mutex::new(None)),
        }
    }
}

impl Module for LuaScriptModule {
    fn get_module_data(&self) -> &ModuleData {
        &self.data
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.data
    }

    fn on_start(&self) -> anyhow::Result<()> {
        log::info!("Lua script '{}' enabled", self.script_name);
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        log::info!("Lua script '{}' disabled", self.script_name);
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        if let Some(ref handler) = *self.update_handler.lock().unwrap() {
            handler();
        }
        Ok(())
    }

    fn handle_packet(&self, packet: &mut Packet) -> PacketAction {
        if let Some(ref handler) = *self.packet_send_handler.lock().unwrap() {
            return handler(packet);
        }
        PacketAction::Forward
    }
}
