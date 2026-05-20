use crate::mapping::client::gamemode::MultiPlayerGameMode;
use crate::mapping::client::window::Window;
use crate::mapping::client::world::World;
use crate::mapping::entity::player::{Abilities, LocalPlayer};
use crate::mapping::entity::Entity;
use crate::mapping::{FieldType, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;
use std::ops::Deref;
use std::sync::RwLock;

/// The running Minecraft client — the `net.minecraft.client.Minecraft`
/// instance plus the game objects reached through it.
#[derive(Debug)]
pub struct Minecraft {
    pub jni_ref: GlobalRef,
    player: RwLock<LocalPlayer>,
    #[allow(dead_code)]
    pub world: World,
    pub window: Window,
    pub game_mode: MultiPlayerGameMode,
}

impl Minecraft {
    /// Builds the game wrapper from the live `Minecraft.getInstance()`.
    pub fn new() -> anyhow::Result<Minecraft> {
        let minecraft = mapping()
            .call_static_method(MinecraftClassType::Minecraft, "getInstance", &[])?
            .l()?;
        if minecraft.is_null() {
            return Err(anyhow::anyhow!("Minecraft.getInstance() returned null"));
        }
        let minecraft = mapping().new_global_ref(minecraft)?;

        let player = LocalPlayer::new(&minecraft)?;
        let world = World::new(&minecraft)?;
        let window = Window::new(&minecraft)?;
        let game_mode = MultiPlayerGameMode::new(
            mapping().new_global_ref(
                mapping()
                    .get_field(
                        MinecraftClassType::Minecraft,
                        minecraft.as_obj(),
                        "gameMode",
                        FieldType::Object(MinecraftClassType::MultiPlayerGameMode),
                    )?
                    .l()?,
            )?,
        );

        Ok(Minecraft {
            jni_ref: minecraft,
            player: RwLock::new(player),
            world,
            window,
            game_mode,
        })
    }

    /// Returns the local player, refreshing the cache when the underlying
    /// JVM object has changed (a new world join produces a new instance).
    pub fn get_player(&self) -> anyhow::Result<LocalPlayer> {
        let player_obj = mapping()
            .get_field(
                MinecraftClassType::Minecraft,
                self.jni_ref.as_obj(),
                "player",
                FieldType::Object(MinecraftClassType::LocalPlayer),
            )?
            .l()?;

        if player_obj.is_null() {
            return Err(anyhow::anyhow!("Player is null"));
        }

        {
            let read_guard = self
                .player
                .read()
                .map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
            if mapping()
                .get_env()?
                .is_same_object(&read_guard.jni_ref, &player_obj)?
            {
                return Ok(read_guard.clone());
            }
        }

        let mut write_guard = self
            .player
            .write()
            .map_err(|_| anyhow::anyhow!("Lock poisoned"))?;

        let jni_ref = mapping().new_global_ref(player_obj)?;
        *write_guard = LocalPlayer {
            jni_ref: jni_ref.clone(),
            abilities: Abilities::new(jni_ref.clone())?,
            entity: Entity::new(jni_ref),
        };

        Ok(write_guard.clone())
    }

    pub fn current_screen_is_null(&self) -> bool {
        if let Ok(screen_obj) = mapping().get_field(
            MinecraftClassType::Minecraft,
            self.jni_ref.as_obj(),
            "screen",
            FieldType::Object(MinecraftClassType::Screen),
        ) {
            if let Ok(l) = screen_obj.l() {
                return l.is_null();
            }
        }
        true
    }
}

impl Deref for Minecraft {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
