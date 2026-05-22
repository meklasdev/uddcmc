//! Wrappers for Minecraft's inventory layer — `Inventory`, `ItemStack`, `Item`
//! and the open container menu.

use crate::mapping::java::iterable::Iterable;
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

    /// Maps an `Inventory` index to its slot number in the `InventoryMenu`:
    /// the hotbar (`Inventory` 0-8) sits at menu slots 36-44; the main storage
    /// (`Inventory` 9-35) keeps the same numbers.
    pub fn menu_slot(inventory_slot: i32) -> i32 {
        if inventory_slot < 9 {
            inventory_slot + 36
        } else {
            inventory_slot
        }
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

    /// Every slot's item stack, in slot order. A standard menu ends with the
    /// player's own 36 inventory slots (27 storage + 9 hotbar).
    pub fn get_items(&self) -> anyhow::Result<Vec<ItemStack>> {
        self.in_frame(|| {
            let list = self.call_method("getItems", &[])?.l()?;
            let iterable = Iterable::new(mapping().new_global_ref(list)?);
            let iterator = iterable.iterator()?;
            let mut items = Vec::new();
            while iterator.has_next()? {
                items.push(ItemStack::new(iterator.next()?));
            }
            Ok(items)
        })
    }
}

/// A Minecraft `EquipmentSlot` — which equipment slot an item belongs to.
#[derive(Debug, MappedObject)]
#[mapped(class = EquipmentSlot)]
pub struct EquipmentSlot {
    jni_ref: GlobalRef,
}

impl EquipmentSlot {
    /// Wraps an existing `EquipmentSlot` JVM object.
    pub fn new(jni_ref: GlobalRef) -> EquipmentSlot {
        EquipmentSlot { jni_ref }
    }

    /// Whether this is one of the four armor slots (head, chest, legs, feet).
    pub fn is_armor(&self) -> anyhow::Result<bool> {
        Ok(self.call_method("isArmor", &[])?.z()?)
    }

    /// This slot's `Inventory` index, given the armor block's base index — for
    /// the player's inventory that base is `36`, giving slots `36..=39`.
    pub fn index(&self, base: i32) -> anyhow::Result<i32> {
        Ok(self.call_method("getIndex", &[JValue::Int(base)])?.i()?)
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
