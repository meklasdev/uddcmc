//! Global client state.
//!
//! Two things live for the whole life of the injected client: the JNI
//! [`Mapping`] bridge and the running game / module [`Client`]. Each is built
//! once by [`init`] and reached afterwards through a free accessor.
//!
//! Initialization order is explicit (see [`init`]): the mapping is built
//! first because the game wrappers resolve their classes through it. The
//! accessors `expect` the state to exist — using one before `init` is a
//! programmer error, not a runtime condition, so it panics with a clear
//! message instead of returning an `Option` every caller would unwrap.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use jni::JNIEnv;
use log::error;
use thiserror::Error;

use crate::mapping::client::minecraft::Minecraft;
use crate::mapping::Mapping;
use crate::module::{Module, ModuleType};

/// The JNI mapping bridge. Built first; the game wrappers depend on it.
static MAPPING: OnceLock<Mapping> = OnceLock::new();
/// The running game and module state.
static CLIENT: OnceLock<Client> = OnceLock::new();

/// Something that went wrong bringing the client up.
#[derive(Debug, Error)]
pub enum ClientError {
    /// [`init`] was called more than once.
    #[error("the client is already initialized")]
    AlreadyInitialized,
    /// A construction step failed.
    #[error(transparent)]
    Init(#[from] anyhow::Error),
}

/// Initializes the global client state. Must be called exactly once, before
/// any accessor is used: it builds the mapping, then the game/module state.
pub fn init() -> Result<(), ClientError> {
    MAPPING
        .set(Mapping::new()?)
        .map_err(|_| ClientError::AlreadyInitialized)?;
    CLIENT
        .set(Client::new()?)
        .map_err(|_| ClientError::AlreadyInitialized)?;
    Ok(())
}

/// The JNI mapping bridge. Valid once [`init`] has succeeded.
#[inline]
pub fn mapping() -> &'static Mapping {
    MAPPING.get().expect("mapping() used before client init")
}

/// The global client. Valid once [`init`] has succeeded.
#[inline]
pub fn client() -> &'static Client {
    CLIENT.get().expect("client() used before client init")
}

/// The running Minecraft game. Valid once [`init`] has succeeded.
#[inline]
pub fn minecraft() -> &'static Minecraft {
    &client().minecraft
}

/// Attaches the current thread to the JVM and returns a JNI environment.
pub fn env() -> anyhow::Result<JNIEnv<'static>> {
    mapping().get_env()
}

/// Registered modules, keyed by name.
type ModuleMap = RwLock<HashMap<String, Arc<Mutex<ModuleType>>>>;

/// The running game and module state.
pub struct Client {
    minecraft: Minecraft,
    /// Registered modules, keyed by name.
    pub modules: ModuleMap,
}

impl Client {
    /// Builds the client. The [`MAPPING`] global must already be set.
    fn new() -> Result<Client, ClientError> {
        Ok(Client {
            minecraft: Minecraft::new()?,
            modules: RwLock::new(HashMap::new()),
        })
    }

    /// Registers a module under its name.
    pub fn register_module<M>(&self, module: M)
    where
        M: Module + Send + Sync + 'static,
    {
        let module: ModuleType = Box::new(module);
        let name = module.get_module_data().name.clone();
        if let Ok(mut modules) = self.modules.write() {
            modules.insert(name, Arc::new(Mutex::new(module)));
        }
    }

    /// Ticks every enabled module once. A module whose tick fails is stopped
    /// rather than aborting the whole pass.
    pub fn tick(&self) {
        let Ok(modules) = self.modules.read() else {
            return;
        };
        for module in modules.values() {
            let Ok(module) = module.lock() else {
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
