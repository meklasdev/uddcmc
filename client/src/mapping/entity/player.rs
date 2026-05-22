//! The player wrappers — `Player`, `LocalPlayer`, and the `Abilities` carried
//! by `LocalPlayer`.

use crate::mapping::client::player_info::PlayerInfo;
use crate::mapping::entity::{EntityRef, LivingEntityRef, PlayerRef};
use crate::mapping::inventory::{AbstractContainerMenu, EquipmentSlot, Inventory, ItemStack};
use crate::mapping::{FieldType, MappedObject, MinecraftClassType};
use crate::state::mapping;
use jni::objects::{GlobalRef, JValue};
use jni::sys::jboolean;

/// Any Minecraft `Player` entity (local or remote). Its behaviour comes from
/// [`EntityRef`], [`LivingEntityRef`] and [`PlayerRef`].
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Player)]
pub struct Player {
    jni_ref: GlobalRef,
}

impl Player {
    /// Wraps an existing `Player` JVM object.
    pub fn new(jni_ref: GlobalRef) -> Player {
        Player { jni_ref }
    }
}

impl EntityRef for Player {}
impl LivingEntityRef for Player {}
impl PlayerRef for Player {}

/// The client's own `LocalPlayer`. Behaves as a [`PlayerRef`] and additionally
/// carries the player's [`Abilities`].
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = LocalPlayer)]
pub struct LocalPlayer {
    jni_ref: GlobalRef,
    pub abilities: Abilities,
}

impl LocalPlayer {
    /// Wraps an existing `LocalPlayer` JVM object.
    pub fn new(player_ref: GlobalRef) -> anyhow::Result<Self> {
        Ok(Self {
            abilities: Abilities::new(player_ref.clone())?,
            jni_ref: player_ref,
        })
    }

    /// The player's `Inventory`. `getInventory` is declared on `Player`, so the
    /// call resolves against that class (see [`Abilities::new`]).
    pub fn get_inventory(&self) -> anyhow::Result<Inventory> {
        mapping().in_frame(|| {
            let inventory = mapping()
                .call_method(
                    MinecraftClassType::Player,
                    self.jni_ref().as_obj(),
                    "getInventory",
                    &[],
                )?
                .l()?;
            Ok(Inventory::new(mapping().new_global_ref(inventory)?))
        })
    }

    /// The container menu currently open for the player — the player's own
    /// inventory menu (`containerId` 0) unless an external container is open.
    pub fn container_menu(&self) -> anyhow::Result<AbstractContainerMenu> {
        mapping().in_frame(|| {
            let menu = mapping()
                .get_field(
                    MinecraftClassType::Player,
                    self.jni_ref().as_obj(),
                    "containerMenu",
                    FieldType::Object(MinecraftClassType::AbstractContainerMenu),
                )?
                .l()?;
            Ok(AbstractContainerMenu::new(mapping().new_global_ref(menu)?))
        })
    }

    /// The client-side `PlayerInfo` for this player. The field is on
    /// `AbstractClientPlayer`; reading it through `LocalPlayer` works in
    /// reflected mode because `GetFieldID` walks superclasses.
    pub fn player_info(&self) -> anyhow::Result<PlayerInfo> {
        mapping().in_frame(|| {
            let info = mapping()
                .get_field(
                    MinecraftClassType::LocalPlayer,
                    self.jni_ref().as_obj(),
                    "playerInfo",
                    FieldType::Object(MinecraftClassType::PlayerInfo),
                )?
                .l()?;
            if info.is_null() {
                anyhow::bail!("LocalPlayer.playerInfo is null");
            }
            Ok(PlayerInfo::new(mapping().new_global_ref(info)?))
        })
    }

    /// Re-delivers an inbound packet through the local player's packet
    /// listener — the same path `Connection.channelRead` would take. Used by
    /// a module to replay a packet it captured and queued earlier (e.g.
    /// Freecam's queued `ClientboundPlayerInfoUpdatePacket`s).
    pub fn forward_packet(&self, packet: &GlobalRef) -> anyhow::Result<()> {
        mapping().in_frame(|| {
            let mut env = mapping().get_env()?;
            let listener = env
                .get_field(
                    self.jni_ref().as_obj(),
                    "connection",
                    "Lnet/minecraft/client/multiplayer/ClientPacketListener;",
                )?
                .l()?;
            if listener.is_null() {
                // No connection any more — nothing to deliver into.
                return Ok(());
            }
            env.call_method(
                packet.as_obj(),
                "handle",
                "(Lnet/minecraft/network/protocol/game/ClientGamePacketListener;)V",
                &[JValue::Object(&listener)],
            )?;
            Ok(())
        })
    }

    /// The `EquipmentSlot` the item `stack` belongs to — an armor slot for
    /// armor, `MAINHAND` otherwise. `getEquipmentSlotForItem` is declared on
    /// `LivingEntity`.
    pub fn equipment_slot_for_item(&self, stack: &ItemStack) -> anyhow::Result<EquipmentSlot> {
        mapping().in_frame(|| {
            let slot = mapping()
                .call_method(
                    MinecraftClassType::LivingEntity,
                    self.jni_ref().as_obj(),
                    "getEquipmentSlotForItem",
                    &[JValue::Object(stack.jni_ref().as_obj())],
                )?
                .l()?;
            Ok(EquipmentSlot::new(mapping().new_global_ref(slot)?))
        })
    }
}

impl EntityRef for LocalPlayer {}
impl LivingEntityRef for LocalPlayer {}
impl PlayerRef for LocalPlayer {}

/// The player's `Abilities` — flight flags and creative-fly speed.
#[derive(Debug, Clone, MappedObject)]
#[mapped(class = Abilities)]
pub struct Abilities {
    jni_ref: GlobalRef,
}

impl Abilities {
    /// Reads the `Abilities` object off `player`.
    pub fn new(player: GlobalRef) -> anyhow::Result<Self> {
        mapping().in_frame(|| {
            let jni_ref = mapping()
                .call_method(MinecraftClassType::Player, &player, "getAbilities", &[])?
                .l()?;
            Ok(Self {
                jni_ref: mapping().new_global_ref(jni_ref)?,
            })
        })
    }

    /// Sets the flying / may-fly flags.
    pub fn fly(&self, value: bool) -> anyhow::Result<()> {
        let value: jboolean = if value { 1 } else { 0 };
        self.set_field("flying", FieldType::Boolean, JValue::Bool(value))?;
        self.set_field("mayfly", FieldType::Boolean, JValue::Bool(value))?;
        Ok(())
    }

    /// Whether the player is currently flying.
    pub fn is_flying(&self) -> anyhow::Result<bool> {
        Ok(self.get_field("flying", FieldType::Boolean)?.z()?)
    }

    /// Sets the creative-fly speed (vanilla default `0.05`).
    pub fn set_flying_speed(&self, speed: f32) -> anyhow::Result<()> {
        self.call_method("setFlyingSpeed", &[JValue::Float(speed)])?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_may_fly(&self) -> anyhow::Result<bool> {
        Ok(self.get_field("mayfly", FieldType::Boolean)?.z()?)
    }
}
