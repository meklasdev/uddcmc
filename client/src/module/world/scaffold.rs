use crate::mapping::block::Direction;
use crate::mapping::entity::EntityRef;
use crate::mapping::math::{BlockPos, Vec3};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;

/// Places a block beneath the player whenever the support under their feet is
/// missing — an auto-bridge. The held item is what gets placed, so the player
/// should be holding blocks.
#[derive(Debug)]
pub struct ScaffoldModule {
    pub module: ModuleData,
}

impl ScaffoldModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Scaffold,
                description: "Places blocks beneath the player".to_string(),
                category: ModuleCategory::World,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
        }
    }
}

impl Module for ScaffoldModule {
    fn on_start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
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
            return Ok(());
        };

        // The block the player needs beneath their feet.
        let feet = player.get_position()?;
        let target = BlockPos::new(
            feet.x().floor() as i32,
            feet.y().round() as i32 - 1,
            feet.z().floor() as i32,
        );

        // Already supported — nothing to place.
        if !world.is_block_air(&target)? {
            return Ok(());
        }

        // Place against the first solid neighbour found.
        for direction in Direction::ALL {
            let (dx, dy, dz) = direction.offset();
            let anchor = target.offset(dx, dy, dz);
            if world.is_block_air(&anchor)? {
                continue; // need a solid block to place against
            }
            // The anchor face pointing back at the target is where the new
            // block attaches.
            let face = direction.opposite();
            return game_mode.place_block_on(&player, &anchor, face, face_center(anchor, face));
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

/// The world point at the centre of the `face` face of block `anchor`.
fn face_center(anchor: BlockPos, face: Direction) -> Vec3 {
    let (fx, fy, fz) = face.offset();
    Vec3::new(
        anchor.x() as f64 + 0.5 + fx as f64 * 0.5,
        anchor.y() as f64 + 0.5 + fy as f64 * 0.5,
        anchor.z() as f64 + 0.5 + fz as f64 * 0.5,
    )
}
