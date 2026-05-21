//! Wrapper for Minecraft's `BlockEntity` (chests, barrels, …).

use crate::mapping::math::BlockPos;
use crate::mapping::{MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;

/// A Minecraft `BlockEntity`.
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = BlockEntity)]
pub struct BlockEntity {
    jni_ref: GlobalRef,
}

impl BlockEntity {
    /// Wraps an existing `BlockEntity` JVM object.
    pub fn new(jni_ref: GlobalRef) -> BlockEntity {
        BlockEntity { jni_ref }
    }

    /// The block position this block entity occupies.
    pub fn get_block_pos(&self) -> anyhow::Result<BlockPos> {
        self.in_frame(|| {
            let pos = self.call_method("getBlockPos", &[])?.l()?;
            BlockPos::read(&pos)
        })
    }

    /// Whether this block entity is a storage container — chest (incl. trapped
    /// chest, a `ChestBlockEntity` subclass), ender chest, barrel or shulker box.
    pub fn is_container(&self) -> bool {
        const KINDS: [MinecraftClassType; 4] = [
            MinecraftClassType::ChestBlockEntity,
            MinecraftClassType::EnderChestBlockEntity,
            MinecraftClassType::BarrelBlockEntity,
            MinecraftClassType::ShulkerBoxBlockEntity,
        ];
        self.in_frame(|| {
            Ok(KINDS.iter().any(|&kind| {
                mapping()
                    .is_instance_of(kind, self.jni_ref().as_obj())
                    .unwrap_or(false)
            }))
        })
        .unwrap_or(false)
    }
}
