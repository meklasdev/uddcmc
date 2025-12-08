use crate::mapping::client::minecraft::Minecraft;
use crate::mapping::FieldType;
use crate::mapping::GameContext;
use crate::mapping::MinecraftClassType;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleSetting};
use jni::objects::JValue;

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
                category: ModuleCategory::COMBAT,
                key_bind: KeyboardKey::KeyC,
                enabled: false,
                player: Minecraft::instance().player.clone(),
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
        let minecraft = Minecraft::instance();
        let player = &minecraft.player;
        let world = &minecraft.world;

        let entities = world.get_entities()?;
        let range = self.get_range() as f64;
        let mapping = minecraft.mapping();

        let player_pos = player.entity.get_position()?;
        let mut closest_dist = range;
        let mut target_entity = None;
        let env = mapping.get_env()?;

        for entity in entities {
            if env.is_same_object(entity.jni_ref.as_obj(), player.entity.jni_ref.as_obj())? {
                continue;
            }

            let entity_pos = entity.get_position()?;
            let dist = ((player_pos.0 - entity_pos.0).powi(2)
                + (player_pos.1 - entity_pos.1).powi(2)
                + (player_pos.2 - entity_pos.2).powi(2))
                .sqrt();

            if dist <= closest_dist {
                closest_dist = dist;
                target_entity = Some(entity);
            }
        }

        if let Some(target) = target_entity {
            let target_pos = target.get_position()?;
            let dx = target_pos.0 - player_pos.0;
            let dy = target_pos.1 - player_pos.1; // This is simplistic, usually need eye height
            let dz = target_pos.2 - player_pos.2;

            let dist = (dx * dx + dz * dz).sqrt();
            let yaw = (dz.atan2(dx) * 180.0 / std::f64::consts::PI) as f32 - 90.0;
            let pitch = (-(dy.atan2(dist)) * 180.0 / std::f64::consts::PI) as f32;

            let mapping = player.mapping();

            // Set yaw
            mapping.set_field(
                MinecraftClassType::Entity,
                player.entity.jni_ref.as_obj(),
                "yRot",
                FieldType::Float,
                JValue::Float(yaw),
            )?;

            // Set pitch
            mapping.set_field(
                MinecraftClassType::Entity,
                player.entity.jni_ref.as_obj(),
                "xRot",
                FieldType::Float,
                JValue::Float(pitch),
            )?;
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
