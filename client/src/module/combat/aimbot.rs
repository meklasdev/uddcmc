use crate::mapping::JavaObject;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleSetting};
use crate::state::minecraft;

#[derive(Debug)]
pub struct AimbotModule {
    pub module: ModuleData,
}

impl AimbotModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                name: "Aimbot".to_string(),
                description: "Automatically aims at entities".to_string(),
                category: ModuleCategory::Combat,
                key_bind: KeyboardKey::KeyC,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Range".to_string(),
                    value: 4.0,
                    min: 1.0,
                    max: 6.0,
                }],
            },
        }
    }

    pub fn get_range(&self) -> f32 {
        self.module
            .get_setting("Range")
            .and_then(|s| s.get_slider_value())
            .unwrap_or(4.0)
    }
}

impl Module for AimbotModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let minecraft = minecraft();
        let (Some(player), Some(world)) = (minecraft.player()?, minecraft.world()?) else {
            return Ok(()); // not in a world — nothing to do
        };

        let range = self.get_range() as f64;
        let player_pos = player.entity.get_position()?;
        let mut closest_dist = range;
        let mut target_entity = None;

        for entity in world.get_entities()? {
            if entity.is_same(&player.entity) {
                continue;
            }

            let entity_pos = entity.get_position()?;
            let dist = ((player_pos.x() - entity_pos.x()).powi(2)
                + (player_pos.y() - entity_pos.y()).powi(2)
                + (player_pos.z() - entity_pos.z()).powi(2))
            .sqrt();

            if dist <= closest_dist {
                closest_dist = dist;
                target_entity = Some(entity);
            }
        }

        if let Some(target) = target_entity {
            let target_pos = target.get_position()?;
            let dx = target_pos.x() - player_pos.x();
            let dy = target_pos.y() - player_pos.y(); // This is simplistic, usually need eye height
            let dz = target_pos.z() - player_pos.z();

            let dist = (dx * dx + dz * dz).sqrt();
            let yaw = (dz.atan2(dx) * 180.0 / std::f64::consts::PI) as f32 - 90.0;
            let pitch = (-(dy.atan2(dist)) * 180.0 / std::f64::consts::PI) as f32;

            player.entity.set_yaw(yaw)?;
            player.entity.set_pitch(pitch)?;
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
