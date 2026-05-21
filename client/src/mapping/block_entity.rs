//! Wrapper for Minecraft's `BlockEntity` (chests, barrels, …).

use crate::mapping::math::BlockPos;
use crate::mapping::{JavaObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;
use std::ops::Deref;

/// A Minecraft `BlockEntity`.
#[derive(Debug, Clone)]
pub struct BlockEntity {
    pub jni_ref: GlobalRef,
}

impl BlockEntity {
    /// Wraps an existing `BlockEntity` JVM object.
    pub fn new(jni_ref: GlobalRef) -> BlockEntity {
        BlockEntity { jni_ref }
    }

    /// The block position this block entity occupies.
    pub fn get_block_pos(&self) -> anyhow::Result<BlockPos> {
        mapping().in_frame(|| {
            let pos = mapping()
                .call_method(
                    MinecraftClassType::BlockEntity,
                    self.jni_ref.as_obj(),
                    "getBlockPos",
                    &[],
                )?
                .l()?;
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
        mapping()
            .in_frame(|| {
                Ok(KINDS.iter().any(|&kind| {
                    mapping()
                        .is_instance_of(kind, self.jni_ref.as_obj())
                        .unwrap_or(false)
                }))
            })
            .unwrap_or(false)
    }
}

impl JavaObject for BlockEntity {
    fn jni_ref(&self) -> &GlobalRef {
        &self.jni_ref
    }

    fn class_type() -> MinecraftClassType {
        MinecraftClassType::BlockEntity
    }
}

impl Deref for BlockEntity {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
