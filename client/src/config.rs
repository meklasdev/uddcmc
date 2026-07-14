//! Persistent module configuration and user settings profiles.
//!
//! Handles multiple profiles (creating, switching, deleting), accent color presets,
//! GUI preferences, performance constraints, and provides atomic file writes
//! to protect against config corruption.

use crate::module::{KeyboardKey, ModuleCategory, ModuleId, ModuleSetting};
use crate::state::client;
use crate::graphic::theme::{AccentPreset, set_accent};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{LazyLock, Mutex};

/// Category-panel positions, keyed by category — updated as the user drags a
/// panel, and persisted so the layout survives a re-injection.
static PANEL_POS: LazyLock<DashMap<ModuleCategory, [f32; 2]>> = LazyLock::new(DashMap::new);

/// Which modules have their settings panel expanded in the GUI, keyed by id.
static EXPANDED: LazyLock<DashMap<ModuleId, bool>> = LazyLock::new(DashMap::new);

// --- User Settings Globals -------------------------------------------------

static ACTIVE_PROFILE: Mutex<String> = Mutex::new(String::new());
static ACCENT_PRESET: Mutex<AccentPreset> = Mutex::new(AccentPreset::Emerald);
static GUI_SCALE: Mutex<f32> = Mutex::new(1.0);
static GUI_OPACITY: Mutex<f32> = Mutex::new(1.0);
static PERF_LIMIT_FPS: AtomicI32 = AtomicI32::new(60);
static PROFILES_LIST: Mutex<Vec<String>> = Mutex::new(Vec::new());

// --- Getters & Setters -----------------------------------------------------

pub fn active_profile() -> String {
    let guard = ACTIVE_PROFILE.lock().unwrap();
    if guard.is_empty() {
        "Default".to_string()
    } else {
        guard.clone()
    }
}

pub fn set_active_profile_name(name: String) {
    *ACTIVE_PROFILE.lock().unwrap() = name;
}

pub fn accent_preset() -> AccentPreset {
    *ACCENT_PRESET.lock().unwrap()
}

pub fn set_accent_preset(preset: AccentPreset) {
    *ACCENT_PRESET.lock().unwrap() = preset;
    set_accent(preset.color());
}

pub fn gui_scale() -> f32 {
    *GUI_SCALE.lock().unwrap()
}

pub fn set_gui_scale(scale: f32) {
    *GUI_SCALE.lock().unwrap() = scale.clamp(0.5, 2.0);
}

pub fn gui_opacity() -> f32 {
    *GUI_OPACITY.lock().unwrap()
}

pub fn set_gui_opacity(opacity: f32) {
    *GUI_OPACITY.lock().unwrap() = opacity.clamp(0.1, 1.0);
}

pub fn perf_limit_fps() -> i32 {
    PERF_LIMIT_FPS.load(Ordering::Relaxed)
}

pub fn set_perf_limit_fps(fps: i32) {
    PERF_LIMIT_FPS.store(fps.clamp(10, 240), Ordering::Relaxed);
}

pub fn list_profiles() -> Vec<String> {
    let list = PROFILES_LIST.lock().unwrap();
    if list.is_empty() {
        vec!["Default".to_string()]
    } else {
        list.clone()
    }
}

// --- Layout methods --------------------------------------------------------

/// The saved position of a category panel, if the user has moved it.
pub fn panel_pos(category: ModuleCategory) -> Option<[f32; 2]> {
    PANEL_POS.get(&category).map(|entry| *entry)
}

/// Records a category panel's position.
pub fn set_panel_pos(category: ModuleCategory, pos: [f32; 2]) {
    PANEL_POS.insert(category, pos);
}

/// Whether a module's settings panel is expanded.
pub fn is_expanded(id: ModuleId) -> bool {
    EXPANDED.get(&id).map(|entry| *entry).unwrap_or(false)
}

/// Records a module's expanded state.
pub fn set_expanded(id: ModuleId, expanded: bool) {
    EXPANDED.insert(id, expanded);
}

/// Clears the persisted GUI layout — panel positions and expanded modules.
/// Backs the GUI's "Reset UI" button.
pub fn reset_ui_state() {
    PANEL_POS.clear();
    EXPANDED.clear();
}

// --- Configuration Serialization -------------------------------------------

const CONFIG_FILE: &str = "dark_client_config.json";

pub fn base_dir() -> PathBuf {
    match std::env::var_os("DARK_CONFIG_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from("."),
    }
}

fn config_path() -> PathBuf {
    base_dir().join(CONFIG_FILE)
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    #[serde(default = "default_active_profile")]
    active_profile: String,
    #[serde(default)]
    accent_preset: AccentPreset,
    #[serde(default = "default_gui_scale")]
    gui_scale: f32,
    #[serde(default = "default_gui_opacity")]
    gui_opacity: f32,
    #[serde(default = "default_perf_limit_fps")]
    perf_limit_fps: i32,
    #[serde(default)]
    profiles: Vec<UserProfile>,
}

fn default_active_profile() -> String {
    "Default".to_string()
}

fn default_gui_scale() -> f32 {
    1.0
}

fn default_gui_opacity() -> f32 {
    1.0
}

fn default_perf_limit_fps() -> i32 {
    60
}

#[derive(Serialize, Deserialize, Clone)]
struct UserProfile {
    name: String,
    modules: Vec<ModuleConfig>,
    #[serde(default)]
    panels: Vec<PanelConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ModuleConfig {
    id: ModuleId,
    key_bind: i32,
    enabled: bool,
    #[serde(default)]
    expanded: bool,
    settings: Vec<SavedSetting>,
}

#[derive(Serialize, Deserialize, Clone)]
struct PanelConfig {
    category: ModuleCategory,
    x: f32,
    y: f32,
}

#[derive(Serialize, Deserialize, Clone)]
struct SavedSetting {
    name: String,
    value: SettingValue,
}

#[derive(Serialize, Deserialize, Clone)]
enum SettingValue {
    Toggle(bool),
    Slider(f32),
    Choice(usize),
    Color([f32; 4]),
}

impl SettingValue {
    fn capture(setting: &ModuleSetting) -> SettingValue {
        match setting {
            ModuleSetting::Toggle { value, .. } => SettingValue::Toggle(*value),
            ModuleSetting::Slider { value, .. } => SettingValue::Slider(*value),
            ModuleSetting::Choice { value, .. } => SettingValue::Choice(*value),
            ModuleSetting::Color { value, .. } => SettingValue::Color(*value),
        }
    }

    fn apply(&self, setting: &mut ModuleSetting) {
        match (self, setting) {
            (SettingValue::Toggle(v), ModuleSetting::Toggle { value, .. }) => *value = *v,
            (
                SettingValue::Slider(v),
                ModuleSetting::Slider {
                    value, min, max, ..
                },
            ) => *value = v.clamp(*min, *max),
            (SettingValue::Choice(v), ModuleSetting::Choice { value, options, .. })
                if *v < options.len() =>
            {
                *value = *v;
            }
            (SettingValue::Color(v), ModuleSetting::Color { value, .. }) => *value = *v,
            _ => {}
        }
    }
}

// --- Atomic File Operations & Persistence ---------------------------------

/// Helper to write file atomically to prevent truncation/corruption on sudden game exits.
fn atomic_write(path: &std::path::Path, content: &str) -> std::io::Result<()> {
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, content)?;
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e);
    }
    Ok(())
}

/// Reads the existing file, or creates an empty Config if it doesn't exist/is corrupt.
fn read_or_create_config() -> Config {
    let path = config_path();
    let json = match std::fs::read_to_string(&path) {
        Ok(json) => json,
        Err(_) => return Config::default(),
    };
    serde_json::from_str(&json).unwrap_or_default()
}

/// Writes the current state to the active profile on disk.
pub fn save() {
    let mut config = read_or_create_config();

    // Sync global configurations
    config.active_profile = active_profile();
    config.accent_preset = accent_preset();
    config.gui_scale = gui_scale();
    config.gui_opacity = gui_opacity();
    config.perf_limit_fps = perf_limit_fps();

    // Capture current live modules & panels layout
    let mut current_modules = Vec::new();
    for handle in client().modules.handles() {
        let Ok(module) = handle.lock() else {
            continue;
        };
        let data = module.get_module_data();
        current_modules.push(ModuleConfig {
            id: data.id,
            key_bind: data.key_bind as i32,
            enabled: data.enabled,
            expanded: is_expanded(data.id),
            settings: data
                .settings
                .iter()
                .map(|setting| SavedSetting {
                    name: setting.name().to_string(),
                    value: SettingValue::capture(setting),
                })
                .collect(),
        });
    }

    let current_panels: Vec<PanelConfig> = PANEL_POS
        .iter()
        .map(|entry| PanelConfig {
            category: *entry.key(),
            x: entry.value()[0],
            y: entry.value()[1],
        })
        .collect();

    // Insert or update active profile in the config structure
    let active_name = active_profile();
    if let Some(p) = config.profiles.iter_mut().find(|p| p.name == active_name) {
        p.modules = current_modules;
        p.panels = current_panels;
    } else {
        config.profiles.push(UserProfile {
            name: active_name,
            modules: current_modules,
            panels: current_panels,
        });
    }

    // Refresh profile names list
    let names: Vec<String> = config.profiles.iter().map(|p| p.name.clone()).collect();
    *PROFILES_LIST.lock().unwrap() = names;

    let path = config_path();
    match serde_json::to_string_pretty(&config) {
        Ok(json) => {
            if let Err(error) = atomic_write(&path, &json) {
                log::warn!("config: could not write {}: {error}", path.display());
            }
        }
        Err(error) => log::warn!("config: could not serialize: {error}"),
    }
}

/// Loads the saved config, if any, and applies it to the registered modules.
pub fn load() {
    let path = config_path();
    let json = match std::fs::read_to_string(&path) {
        Ok(json) => json,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // First run, populate default profile
            *ACTIVE_PROFILE.lock().unwrap() = "Default".to_string();
            *PROFILES_LIST.lock().unwrap() = vec!["Default".to_string()];
            return;
        }
        Err(error) => {
            log::warn!("config: could not read {}: {error}", path.display());
            return;
        }
    };
    let config: Config = match serde_json::from_str(&json) {
        Ok(config) => config,
        Err(error) => {
            log::warn!("config: could not parse {}: {error}", path.display());
            return;
        }
    };

    // Load user settings
    *ACTIVE_PROFILE.lock().unwrap() = if config.active_profile.is_empty() {
        "Default".to_string()
    } else {
        config.active_profile.clone()
    };

    set_accent_preset(config.accent_preset);
    *GUI_SCALE.lock().unwrap() = config.gui_scale;
    *GUI_OPACITY.lock().unwrap() = config.gui_opacity;
    PERF_LIMIT_FPS.store(config.perf_limit_fps, Ordering::Relaxed);

    // Refresh profiles list cache
    let mut names: Vec<String> = config.profiles.iter().map(|p| p.name.clone()).collect();
    if names.is_empty() {
        names.push("Default".to_string());
    }
    *PROFILES_LIST.lock().unwrap() = names;

    // Apply active profile or fallback
    let active_name = active_profile();
    let profile = config.profiles.iter().find(|p| p.name == active_name);

    if let Some(saved_profile) = profile {
        // Reset panel pos first so clean transitions work
        PANEL_POS.clear();
        for panel in &saved_profile.panels {
            set_panel_pos(panel.category, [panel.x, panel.y]);
        }

        for handle in client().modules.handles() {
            let Ok(mut module) = handle.lock() else {
                continue;
            };
            let id = module.get_module_data().id;
            let Some(saved) = saved_profile.modules.iter().find(|entry| entry.id == id) else {
                continue;
            };

            {
                let data = module.get_module_data_mut();
                data.key_bind = KeyboardKey::from(saved.key_bind);
                for setting in &mut data.settings {
                    if let Some(saved_setting) = saved
                        .settings
                        .iter()
                        .find(|entry| entry.name == setting.name())
                    {
                        saved_setting.value.apply(setting);
                    }
                }
                data.set_enabled(saved.enabled);
            }

            set_expanded(id, saved.expanded);
        }
    }
}

// --- Active Profile Manipulation API --------------------------------------

/// Switches the active profile and reloads config settings
pub fn switch_profile(name: &str) {
    if name.is_empty() {
        return;
    }
    save(); // Save current profile before switching!
    set_active_profile_name(name.to_string());
    load(); // Load target profile settings
    save(); // Overwrite config with updated active profile marker
}

/// Creates a new profile from the current settings
pub fn create_profile(name: &str) {
    if name.is_empty() {
        return;
    }
    set_active_profile_name(name.to_string());
    save(); // Saves current state under new name and refreshes list
}

/// Deletes a profile
pub fn delete_profile(name: &str) {
    if name == "Default" || name.is_empty() {
        return; // Safeguard Default profile
    }
    let mut config = read_or_create_config();
    config.profiles.retain(|p| p.name != name);

    let active = active_profile();
    if active == name {
        set_active_profile_name("Default".to_string());
    }

    // Refresh profiles list cache
    let mut names: Vec<String> = config.profiles.iter().map(|p| p.name.clone()).collect();
    if names.is_empty() {
        names.push("Default".to_string());
    }
    *PROFILES_LIST.lock().unwrap() = names;

    // Save changes atomically
    let path = config_path();
    if let Ok(json) = serde_json::to_string_pretty(&config) {
        let _ = atomic_write(&path, &json);
    }

    // If we deleted the active profile, load Default settings
    if active == name {
        load();
    }
}
