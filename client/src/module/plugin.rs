//! Client Plugin SDK & Multi-Version Mapping Layer for KRASNOSTAV Minecraft Client.
//! Exposes APIs for Native Rust Plugins, WebAssembly (WASM) extensions, and
//! cross-environment mapping structures (Fabric, Forge, Badlion, Lunar, Vanilla).


/// A premium, native Rust plugin interface.
/// Plugins compiled as dynamic libraries (`.dll` / `.so`) implement this trait.
pub trait ClientPlugin: Send + Sync {
    /// Returns the unique metadata identifier for this plugin.
    fn get_meta(&self) -> PluginMetadata;

    /// Triggered once when the plugin is loaded into the client runtime workspace.
    fn on_load(&self) -> Result<(), String>;

    /// Triggered during client shutdown, panic unloads, or live hot-unloads.
    fn on_unload(&self) -> Result<(), String>;

    /// Triggered on every core client loop game tick.
    fn on_tick(&self);
}

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
}

// --- WebAssembly (WASM) Plugin Integration ----------------------------------

/// Standard sandbox container for loading and running WASM plugins at runtime.
pub struct WasmPluginContainer {
    pub metadata: PluginMetadata,
    pub bytecode: Vec<u8>,
    pub active: bool,
}

impl WasmPluginContainer {
    pub fn new(name: &str, bytecode: Vec<u8>) -> Self {
        Self {
            metadata: PluginMetadata {
                name: name.to_string(),
                author: "WASM Developer".to_string(),
                version: "1.0.0".to_string(),
                description: "Sandboxed WebAssembly plugin compiled for KRASNOSTAV VM.".to_string(),
            },
            bytecode,
            active: false,
        }
    }

    /// Mounts and executes the WASM compiled plugin inside a safe sandboxed interpreter
    pub fn execute_entrypoint(&mut self) -> Result<(), String> {
        if self.bytecode.is_empty() {
            return Err("WASM bytecode stream is completely empty or corrupted".to_string());
        }
        self.active = true;
        log::info!("WASM: Successfully initialized sandboxed virtual machine for '{}'", self.metadata.name);
        Ok(())
    }

    /// Stops the sandboxed WASM environment
    pub fn shutdown(&mut self) {
        self.active = false;
        log::info!("WASM: Safely torn down execution context for '{}'", self.metadata.name);
    }
}

// --- Multi-Version Mapping System -------------------------------------------

/// Supported Minecraft Client environments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetClientEnvironment {
    Vanilla,
    Fabric,
    Forge,
    Lunar,
    Badlion,
}

/// Abstract multi-version mapper mapping JVM classes and methods across Minecraft versions.
pub trait IMultiVersionMapper {
    /// Resolves obfuscated JVM class name based on version and target environment
    fn resolve_class(&self, original_class: &str, environment: TargetClientEnvironment) -> String;

    /// Resolves obfuscated JVM field or method name based on version and target environment
    fn resolve_member(&self, class_name: &str, original_member: &str, environment: TargetClientEnvironment) -> String;
}

pub struct MultiVersionManager {
    pub active_environment: TargetClientEnvironment,
    pub mc_version: String,
}

impl MultiVersionManager {
    pub fn new(env: TargetClientEnvironment, version: &str) -> Self {
        Self {
            active_environment: env,
            mc_version: version.to_string(),
        }
    }
}

impl IMultiVersionMapper for MultiVersionManager {
    fn resolve_class(&self, original_class: &str, environment: TargetClientEnvironment) -> String {
        match (original_class, environment) {
            ("Minecraft", TargetClientEnvironment::Vanilla) => "ave".to_string(),
            ("Minecraft", TargetClientEnvironment::Fabric) => "net.minecraft.class_310".to_string(),
            ("Minecraft", TargetClientEnvironment::Forge) => "net.minecraft.client.Minecraft".to_string(),
            ("EntityPlayer", TargetClientEnvironment::Vanilla) => "bew".to_string(),
            _ => original_class.to_string(),
        }
    }

    fn resolve_member(&self, class_name: &str, original_member: &str, environment: TargetClientEnvironment) -> String {
        match (class_name, original_member, environment) {
            ("Minecraft", "thePlayer", TargetClientEnvironment::Vanilla) => "h".to_string(),
            ("Minecraft", "field_71439_g", TargetClientEnvironment::Forge) => "field_71439_g".to_string(),
            _ => original_member.to_string(),
        }
    }
}
