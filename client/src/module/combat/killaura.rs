use crate::mapping::MinecraftClassType;
use crate::module::combat::aura::BaseAura;
use crate::module::{KeyboardKey, Module, ModuleData};

#[derive(Debug)]
pub struct KillAuraModule {
    pub aura: BaseAura,
}

impl KillAuraModule {
    pub fn new() -> Self {
        Self {
            aura: BaseAura::new(
                "KillAura".to_string(),
                "Automatically attacks players".to_string(),
                KeyboardKey::KeyR,
                MinecraftClassType::Player,
            ),
        }
    }
}

impl Module for KillAuraModule {
    fn on_start(&self) -> anyhow::Result<()> {
        self.aura.on_start()
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        self.aura.on_stop()
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        self.aura.on_tick()
    }

    fn get_module_data(&self) -> &ModuleData {
        self.aura.get_module_data()
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        self.aura.get_module_data_mut()
    }
}
