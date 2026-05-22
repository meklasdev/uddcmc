use crate::mapping::block::Direction;
use crate::mapping::client::world::World;
use crate::mapping::entity::EntityRef;
use crate::mapping::math::BlockPos;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::state::minecraft;
use std::sync::Mutex;

/// Face reported while breaking — any face works, the block breaks regardless.
const MINING_FACE: Direction = Direction::Up;

/// Breaks the blocks around the player, one at a time — instantly in creative,
/// progressively (proper mining) in survival.
#[derive(Debug)]
pub struct NukerModule {
    pub module: ModuleData,
    /// The block currently being broken. Kept so the area is rescanned only
    /// once it is gone — and so survival mining is not restarted on a different
    /// block every tick, which would never finish any of them.
    target: Mutex<Option<BlockPos>>,
}

impl NukerModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Nuker,
                description: "Breaks the blocks around the player".to_string(),
                category: ModuleCategory::World,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Range".to_string(),
                    value: 4.0,
                    min: 1.0,
                    max: 5.0,
                }],
            },
            target: Mutex::new(None),
        }
    }

    fn range(&self) -> i32 {
        self.module
            .get_setting("Range")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(4.0) as i32
    }
}

impl Module for NukerModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        *self.target.lock().unwrap() = None;
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let minecraft = minecraft();
        // Stand down while a menu is open.
        if minecraft.current_screen().is_open() {
            return Ok(());
        }
        let (Some(player), Some(world), Some(game_mode)) = (
            minecraft.player()?,
            minecraft.world()?,
            minecraft.game_mode()?,
        ) else {
            *self.target.lock().unwrap() = None;
            return Ok(());
        };

        let feet = player.get_position()?;
        let center = BlockPos::new(
            feet.x().floor() as i32,
            feet.y().floor() as i32,
            feet.z().floor() as i32,
        );
        let range = self.range();

        // Keep mining the cached target while it is still a block; otherwise
        // scan the area for the nearest one.
        let mut target = self.target.lock().unwrap();
        let chosen = match *target {
            Some(pos) if !world.is_block_air(&pos)? => Some(pos),
            _ => nearest_block(&world, center, range)?,
        };
        *target = chosen;
        drop(target);

        if let Some(pos) = chosen {
            game_mode.start_destroy_block(&pos, MINING_FACE)?;
            game_mode.continue_destroy_block(&pos, MINING_FACE)?;
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

/// The nearest non-air block within `range` of `center`, if any.
fn nearest_block(world: &World, center: BlockPos, range: i32) -> anyhow::Result<Option<BlockPos>> {
    let mut best: Option<(BlockPos, i32)> = None;
    let range_sq = range * range;
    for dx in -range..=range {
        for dy in -range..=range {
            for dz in -range..=range {
                let distance_sq = dx * dx + dy * dy + dz * dz;
                if distance_sq == 0 || distance_sq > range_sq {
                    continue;
                }
                let pos = center.offset(dx, dy, dz);
                if world.is_block_air(&pos)? {
                    continue;
                }
                if best.is_none_or(|(_, best_distance)| distance_sq < best_distance) {
                    best = Some((pos, distance_sq));
                }
            }
        }
    }
    Ok(best.map(|(pos, _)| pos))
}
