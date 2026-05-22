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

    /// Performs a SWAP container click: swaps menu slot `slot` of the container
    /// `container_id` with an inventory hotbar slot (`hotbar_button` 0-8) or the
    /// off-hand (`hotbar_button` 40). Sends the click to the server.
    pub fn container_swap(
        &self,
        container_id: i32,
        slot: i32,
        hotbar_button: i32,
        player: &LocalPlayer,
    ) -> anyhow::Result<()> {
        self.in_frame(|| {
            let swap = mapping()
                .get_static_field(
                    MinecraftClassType::ContainerInput,
                    "SWAP",
                    FieldType::Object(MinecraftClassType::ContainerInput),
                )?
                .l()?;
            self.call_method(
                "handleContainerInput",
                &[
                    JValue::Int(container_id),
                    JValue::Int(slot),
                    JValue::Int(hotbar_button),
                    JValue::Object(&swap),
                    JValue::Object(player.jni_ref().as_obj()),
                ],
            )?;
            Ok(())
        })
    }
}
