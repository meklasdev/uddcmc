use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;

/// Ground speed, in blocks/tick, the player is restored to while using an item
/// — roughly vanilla walking speed.
const NORMAL_SPEED: f64 = 0.1;
/// Horizontal speed below which direction is too ill-defined to rescale.
const EPSILON: f64 = 0.001;

/// Cancels the movement slowdown Minecraft applies while an item is in use
/// (eating, drawing a bow, blocking).
///
/// The slowdown is client-side input physics with no hook to remove, so it is
/// undone after the fact: while an item is in use, the player is on the ground
/// and a movement key is held, the horizontal velocity is rescaled back up to
/// normal walking speed. Gating on real input (`xxa`/`zza`) is what keeps the
/// player from gliding once the keys are released.
#[derive(Debug)]
pub struct NoSlowdownModule {
    pub module: ModuleData,
}

impl NoSlowdownModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::NoSlowdown,
                description: "Removes the slowdown from using items".to_string(),
                category: ModuleCategory::Movement,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
        }
    }
}

impl Module for NoSlowdownModule {
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
        // Only act while an item is in use, on the ground, with a key held —
        // the last condition stops the player gliding after a key release.
        if !player.is_using_item()? || !player.on_ground()? || !player.has_move_input()? {
            return Ok(());
        }

        let motion = player.get_delta_movement()?;
        let horizontal = (motion.x() * motion.x() + motion.z() * motion.z()).sqrt();
        if horizontal <= EPSILON || horizontal >= NORMAL_SPEED {
            return Ok(()); // standing still, or already up to speed
        }

        // Rescale the slowed velocity back to normal speed, direction kept.
        let scale = NORMAL_SPEED / horizontal;
        player.set_delta_movement(motion.x() * scale, motion.y(), motion.z() * scale)
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
