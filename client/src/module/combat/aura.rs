use crate::mapping::entity::mob::Mob;
use crate::mapping::entity::player::Player;
use crate::mapping::entity::Entity;
use crate::mapping::JavaObject;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleSetting};
use crate::state::minecraft;

/// Which entities an aura attacks.
#[derive(Debug, Clone, Copy)]
pub enum AuraTarget {
    Players,
    Mobs,
}

#[derive(Debug)]
pub struct BaseAura {
    pub module: ModuleData,
    pub target: AuraTarget,
}

impl BaseAura {
    pub fn new(
        name: String,
        description: String,
        key_bind: KeyboardKey,
        target: AuraTarget,
    ) -> Self {
        Self {
            module: ModuleData {
                name,
                description,
                category: ModuleCategory::Combat,
                key_bind,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Range".to_string(),
                    value: 4.0,
                    min: 1.0,
                    max: 6.0,
                }],
            },
            target,
        }
    }

    pub fn get_range(&self) -> f32 {
        self.module
            .get_setting("Range")
            .and_then(|s| s.get_slider_value())
            .unwrap_or(4.0)
    }

    /// Whether `entity` is the kind of entity this aura attacks.
    fn is_target(&self, entity: &Entity) -> bool {
        match self.target {
            AuraTarget::Players => entity.instance_of::<Player>(),
            AuraTarget::Mobs => entity.instance_of::<Mob>(),
        }
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
        let (Some(player), Some(world), Some(game_mode)) = (
            minecraft.player()?,
            minecraft.world()?,
            minecraft.game_mode()?,
        ) else {
            return Ok(()); // not in a world — nothing to do
        };

        let range = self.get_range() as f64;
        let player_pos = player.entity.get_position()?;

        for entity in world.get_entities()? {
            if entity.is_same(&player.entity) {
                continue;
            }
            if !self.is_target(&entity) {
                continue;
            }

            let entity_pos = entity.get_position()?;
            let dist = ((player_pos.x() - entity_pos.x()).powi(2)
                + (player_pos.y() - entity_pos.y()).powi(2)
                + (player_pos.z() - entity_pos.z()).powi(2))
            .sqrt();

            if dist <= range {
                game_mode.attack(&player, &entity)?;
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
