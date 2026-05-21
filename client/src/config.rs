//! Persistent module configuration.
//!
//! The keybind, setting values and enabled state of every module — plus the
//! GUI layout (panel positions, which modules are expanded) — are written to
//! `dark_client_config.json` so they survive a re-injection. [`load`] runs once
//! after the modules are registered; [`save`] is called whenever the user
//! changes something in the GUI.

use crate::module::{KeyboardKey, ModuleCategory, ModuleId, ModuleSetting};
use crate::state::client;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::LazyLock;

/// Category-panel positions, keyed by category — updated as the user drags a
/// panel, and persisted so the layout survives a re-injection.
static PANEL_POS: LazyLock<DashMap<ModuleCategory, [f32; 2]>> = LazyLock::new(DashMap::new);

/// Which modules have their settings panel expanded in the GUI, keyed by id.
static EXPANDED: LazyLock<DashMap<ModuleId, bool>> = LazyLock::new(DashMap::new);

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

/// Config file name.
const CONFIG_FILE: &str = "dark_client_config.json";

/// The directory DarkClient keeps its files in — the config and the log. The
/// injector passes its own working directory through the `DARK_CONFIG_DIR` env
/// var (set by the agent loader); keeping files there, rather than inside
/// `.minecraft`, leaves no trace in the game directory. Falls back to the
/// process working directory if the variable is absent.
pub fn base_dir() -> PathBuf {
    match std::env::var_os("DARK_CONFIG_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from("."),
    }
}

/// Absolute path of the config file.
fn config_path() -> PathBuf {
    base_dir().join(CONFIG_FILE)
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    modules: Vec<ModuleConfig>,
    /// `#[serde(default)]` keeps configs written before panels were saved
    /// loadable.
    #[serde(default)]
    panels: Vec<PanelConfig>,
}

#[derive(Serialize, Deserialize)]
struct ModuleConfig {
    id: ModuleId,
    key_bind: i32,
    enabled: bool,
    /// Whether the module's settings panel is expanded. `#[serde(default)]`
    /// keeps older configs loadable.
    #[serde(default)]
    expanded: bool,
    settings: Vec<SavedSetting>,
}

#[derive(Serialize, Deserialize)]
struct PanelConfig {
    category: ModuleCategory,
    x: f32,
    y: f32,
}

#[derive(Serialize, Deserialize)]
struct SavedSetting {
    name: String,
    value: SettingValue,
}

/// The persisted value of a setting — only the value, never the bounds
/// (`min` / `max` / `options` always come from the code defaults).
#[derive(Serialize, Deserialize)]
enum SettingValue {
    Toggle(bool),
    Slider(f32),
    Choice(usize),
    Color([f32; 4]),
}

impl SettingValue {
    /// Snapshots a live setting's value.
    fn capture(setting: &ModuleSetting) -> SettingValue {
        match setting {
            ModuleSetting::Toggle { value, .. } => SettingValue::Toggle(*value),
            ModuleSetting::Slider { value, .. } => SettingValue::Slider(*value),
            ModuleSetting::Choice { value, .. } => SettingValue::Choice(*value),
            ModuleSetting::Color { value, .. } => SettingValue::Color(*value),
        }
    }

    /// Applies the saved value onto a live setting. A type mismatch — the code
    /// changed a setting's kind since the file was written — is ignored.
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

/// Writes the current state of every registered module to disk.
pub fn save() {
    let mut config = Config::default();
    for handle in client().modules.handles() {
        let Ok(module) = handle.lock() else {
            continue;
        };
        let data = module.get_module_data();
        config.modules.push(ModuleConfig {
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

    config.panels = PANEL_POS
        .iter()
        .map(|entry| PanelConfig {
            category: *entry.key(),
            x: entry.value()[0],
            y: entry.value()[1],
        })
        .collect();

    let path = config_path();
    match serde_json::to_string_pretty(&config) {
        Ok(json) => {
            if let Err(error) = std::fs::write(&path, json) {
                log::warn!("config: could not write {}: {error}", path.display());
            }
        }
        Err(error) => log::warn!("config: could not serialize: {error}"),
    }
}

/// Loads the saved config, if any, and applies it to the registered modules.
/// Must be called after `register_modules()`. A missing file is the normal
/// first-run case and is silently ignored.
pub fn load() {
    let path = config_path();
    let json = match std::fs::read_to_string(&path) {
        Ok(json) => json,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
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

    for handle in client().modules.handles() {
        let Ok(mut module) = handle.lock() else {
            continue;
        };
        let id = module.get_module_data().id;
        let Some(saved) = config.modules.iter().find(|entry| entry.id == id) else {
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

        // Re-enter a module that was saved enabled.
        if saved.enabled {
            if let Err(error) = module.on_start() {
                let name = module.get_module_data().name();
                log::warn!("config: '{name}' failed to start: {error}");
            }
        }
    }

    for panel in &config.panels {
        set_panel_pos(panel.category, [panel.x, panel.y]);
    }
}
