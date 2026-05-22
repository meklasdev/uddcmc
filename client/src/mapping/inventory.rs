//! Wrappers for Minecraft's inventory layer — `Inventory`, `ItemStack`, `Item`
//! and the open container menu.

use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};

/// A player's `Inventory`.
#[derive(Debug, MappedObject)]
#[mapped(class = Inventory)]
pub struct Inventory {
    jni_ref: GlobalRef,
}

impl Inventory {
    /// `Inventory` index of the off-hand slot (`Inventory.SLOT_OFFHAND`).
    pub const SLOT_OFFHAND: i32 = 40;

    /// Wraps an existing `Inventory` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Inventory {
        Inventory { jni_ref }
    }

    /// The item stack in `slot`.
    pub fn get_item(&self, slot: i32) -> anyhow::Result<ItemStack> {
        self.in_frame(|| {
            let stack = self.call_method("getItem", &[JValue::Int(slot)])?.l()?;
            Ok(ItemStack::new(mapping().new_global_ref(stack)?))
        })
    }
}

/// A Minecraft `ItemStack` — an item type plus a count.
#[derive(Debug, MappedObject)]
#[mapped(class = ItemStack)]
pub struct ItemStack {
    jni_ref: GlobalRef,
}

impl ItemStack {
    /// Wraps an existing `ItemStack` JVM object.
    pub fn new(jni_ref: GlobalRef) -> ItemStack {
        ItemStack { jni_ref }
    }

    /// Whether the stack is empty — no item, or a zero count.
    pub fn is_empty(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isEmpty", &[])?.z()?)
    }

    /// The `Item` this stack holds.
    pub fn get_item(&self) -> anyhow::Result<Item> {
        self.in_frame(|| {
            let item = self.call_method("getItem", &[])?.l()?;
            Ok(Item::new(mapping().new_global_ref(item)?))
        })
    }
}

/// A Minecraft `Item` — the *type* of an item, a registered singleton.
/// Identified by JVM identity ([`MappedObject::is_same`]) against the constants
/// in `Items`.
#[derive(Debug, MappedObject)]
#[mapped(class = Item)]
pub struct Item {
    jni_ref: GlobalRef,
}

impl Item {
    /// Wraps an existing `Item` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Item {
        Item { jni_ref }
    }
}

/// The `AbstractContainerMenu` currently open for a player.
#[derive(Debug, MappedObject)]
#[mapped(class = AbstractContainerMenu)]
pub struct AbstractContainerMenu {
    jni_ref: GlobalRef,
}

impl AbstractContainerMenu {
    /// Wraps an existing `AbstractContainerMenu` JVM object.
    pub fn new(jni_ref: GlobalRef) -> AbstractContainerMenu {
        AbstractContainerMenu { jni_ref }
    }

    /// This menu's network id — `0` is always the player's own inventory menu.
    pub fn container_id(&self) -> anyhow::Result<i32> {
        Ok(self.get_field("containerId", FieldType::Int)?.i()?)
    }
}

/// The `Items.TOTEM_OF_UNDYING` item singleton.
pub fn totem_of_undying() -> anyhow::Result<Item> {
    mapping().in_frame(|| {
        let item = mapping()
            .get_static_field(
                MinecraftClassType::Items,
                "TOTEM_OF_UNDYING",
                FieldType::Object(MinecraftClassType::Item),
            )?
            .l()?;
        Ok(Item::new(mapping().new_global_ref(item)?))
    })
}
