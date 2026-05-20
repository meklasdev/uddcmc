use crate::mapping::MinecraftClassType;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleSetting};
use crate::state::{mapping, minecraft};

#[derive(Debug)]
pub struct BaseAura {
    pub module: ModuleData,
    pub target_type: MinecraftClassType,
}

impl BaseAura {
    pub fn new(
        name: String,
        description: String,
        key_bind: KeyboardKey,
        target_type: MinecraftClassType,
    ) -> Self {
        Self {
            module: ModuleData {
                name,
                description,
                category: ModuleCategory::COMBAT,
                key_bind,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Range".to_string(),
                    value: 4.0,
                    min: 1.0,
                    max: 6.0,
                }],
            },
            target_type,
        }
    }

    pub fn get_range(&self) -> f32 {
        self.module
            .get_setting("Range")
            .and_then(|s| s.get_slider_value())
            .unwrap_or(4.0)
    }
}

impl Module for BaseAura {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let minecraft = minecraft();
        let player = &minecraft.get_player()?;
        let world = &minecraft.world;
        let game_mode = &minecraft.game_mode;
        let mapping = mapping();

        let entities = world.get_entities()?;
        let range = self.get_range() as f64;
        let env = mapping.get_env()?;

        let player_pos = player.entity.get_position()?;

        for entity in entities {
            if env.is_same_object(entity.jni_ref.as_obj(), player.entity.jni_ref.as_obj())? {
                continue;
            }

            if !mapping.is_instance_of(self.target_type, entity.jni_ref.as_obj())? {
                continue;
            }

            let entity_pos = entity.get_position()?;
            let dist = ((player_pos.0 - entity_pos.0).powi(2)
                + (player_pos.1 - entity_pos.1).powi(2)
                + (player_pos.2 - entity_pos.2).powi(2))
            .sqrt();

            if dist <= range {
                game_mode.attack(player, &entity)?;
            }
        }

        Ok(())
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
