use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::entity::Entity;
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};

#[derive(Debug, Clone, MappedObject)]
#[mapped(class = MultiPlayerGameMode)]
pub struct MultiPlayerGameMode {
    jni_ref: GlobalRef,
}

impl MultiPlayerGameMode {
    pub fn new(jni_ref: GlobalRef) -> Self {
        Self { jni_ref }
    }

    /// Attacks `target` on behalf of the local player.
    pub fn attack(&self, player: &LocalPlayer, target: &Entity) -> anyhow::Result<()> {
        self.call_method(
            "attack",
            &[
                JValue::Object(player.jni_ref().as_obj()),
                JValue::Object(target.jni_ref().as_obj()),
            ],
        )?;
        Ok(())
    }

    /// Issues a container click with the named `ContainerInput` action — sends
    /// it to the server and applies it client-side.
    fn container_click(
        &self,
        container_id: i32,
        slot: i32,
        button: i32,
        action: &str,
        player: &LocalPlayer,
    ) -> anyhow::Result<()> {
        self.in_frame(|| {
            let input = mapping()
                .get_static_field(
                    MinecraftClassType::ContainerInput,
                    action,
                    FieldType::Object(MinecraftClassType::ContainerInput),
                )?
                .l()?;
            self.call_method(
                "handleContainerInput",
                &[
                    JValue::Int(container_id),
                    JValue::Int(slot),
                    JValue::Int(button),
                    JValue::Object(&input),
                    JValue::Object(player.jni_ref().as_obj()),
                ],
            )?;
            Ok(())
        })
    }

    /// SWAP click: swaps menu slot `slot` of the container `container_id` with
    /// an inventory hotbar slot (`hotbar_button` 0-8) or the off-hand (40).
    pub fn container_swap(
        &self,
        container_id: i32,
        slot: i32,
        hotbar_button: i32,
        player: &LocalPlayer,
    ) -> anyhow::Result<()> {
        self.container_click(container_id, slot, hotbar_button, "SWAP", player)
    }

    /// QUICK_MOVE click (shift-click): moves menu slot `slot`'s stack to its
    /// default destination — into the inventory, or onto an equipment slot.
    pub fn container_quick_move(
        &self,
        container_id: i32,
        slot: i32,
        player: &LocalPlayer,
    ) -> anyhow::Result<()> {
        self.container_click(container_id, slot, 0, "QUICK_MOVE", player)
    }
}
