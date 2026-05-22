use crate::mapping::inventory::Inventory;
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId};
use crate::state::minecraft;

/// Container id of the player's own inventory menu.
const INVENTORY_MENU_ID: i32 = 0;
/// `Inventory` base index of the armor block — slots `36..=39` (feet … head).
const ARMOR_BASE: i32 = 36;
/// `Inventory` indices scanned for armor pieces — hotbar + main storage.
const STORAGE_SLOTS: i32 = 36;

/// Equips armor from the inventory automatically: whenever an armor slot is
/// empty and a matching piece is in the inventory, it is shift-clicked on.
#[derive(Debug)]
pub struct AutoArmorModule {
    pub module: ModuleData,
}

impl AutoArmorModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::AutoArmor,
                description: "Equips armor from the inventory automatically".to_string(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![],
            },
        }
    }
}

impl Module for AutoArmorModule {
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

        // Equipping shift-clicks through the player's own inventory menu —
        // stand down while an external container is open.
        if player.container_menu()?.container_id()? != INVENTORY_MENU_ID {
            return Ok(());
        }

        let inventory = player.get_inventory()?;
        for slot in 0..STORAGE_SLOTS {
            let stack = inventory.get_item(slot)?;
            if stack.is_empty()? {
                continue;
            }

            // Only armor pieces, and only when their armor slot is free —
            // a shift-click into an occupied slot would just shuffle the item.
            let equipment_slot = player.equipment_slot_for_item(&stack)?;
            if !equipment_slot.is_armor()? {
                continue;
            }
            let armor_slot = equipment_slot.index(ARMOR_BASE)?;
            if !inventory.get_item(armor_slot)?.is_empty()? {
                continue;
            }

            // Shift-click it on — Minecraft routes armor to its armor slot.
            let Some(game_mode) = minecraft().game_mode()? else {
                return Ok(());
            };
            return game_mode.container_quick_move(
                INVENTORY_MENU_ID,
                Inventory::menu_slot(slot),
                &player,
            );
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
