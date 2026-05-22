use crate::mapping::entity::EntityRef;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;

/// Approximate vanilla sprint ground speed, in blocks/tick — the `1.0`
/// reference the multiplier scales.
const SPRINT_SPEED: f64 = 0.13;
/// Horizontal speed, in blocks/tick, below which the player counts as still.
const MOVING_THRESHOLD: f64 = 0.05;

/// Moves the player faster than vanilla by rescaling their ground velocity.
///
/// Each tick the horizontal velocity is set to `SPRINT_SPEED × multiplier`,
/// keeping its direction — so movement still follows the player's input, just
/// faster. Only ground movement is touched: boosting mid-air is floaty and
/// conspicuous.
#[derive(Debug)]
pub struct SpeedModule {
    pub module: ModuleData,
}

impl SpeedModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Speed,
                description: "Moves the player faster than vanilla".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Multiplier".to_string(),
                    value: 1.5,
                    min: 1.0,
                    max: 3.0,
                }],
            },
        }
    }

    fn multiplier(&self) -> f64 {
        self.module
            .get_setting("Multiplier")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(1.5) as f64
    }
}

impl Module for SpeedModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
        // Boost ground movement only.
        if !player.on_ground()? {
            return Ok(());
        }

        let motion = player.get_delta_movement()?;
        let horizontal = (motion.x() * motion.x() + motion.z() * motion.z()).sqrt();
        if horizontal <= MOVING_THRESHOLD {
            return Ok(()); // standing still — nothing to scale
        }

        // Rescale the horizontal velocity to the target speed, direction kept.
        let target = SPRINT_SPEED * self.multiplier();
        let scale = target / horizontal;
        player.set_delta_movement(motion.x() * scale, motion.y(), motion.z() * scale)
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
