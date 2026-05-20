use crate::mapping::client::gamemode::MultiPlayerGameMode;
use crate::mapping::client::window::Window;
use crate::mapping::client::world::World;
use crate::mapping::entity::player::LocalPlayer;
use crate::mapping::{FieldType, MinecraftClassType};
use crate::state::mapping;
use jni::objects::GlobalRef;
use std::ops::Deref;
use std::sync::RwLock;

/// The running Minecraft client.
///
/// Only the things that exist from the main menu onward are held eagerly: the
/// `Minecraft.getInstance()` handle and the game [`Window`]. The world-scoped
/// objects — player, level, game mode — are null in the menu and on world
/// exit, so they are fetched lazily and reported as `Ok(None)` when absent.
/// This is what lets the client be injected from the main menu.
#[derive(Debug)]
pub struct Minecraft {
    pub jni_ref: GlobalRef,
    pub window: Window,
    /// Cached local player, refreshed when the underlying JVM object changes.
    player: RwLock<Option<LocalPlayer>>,
}

impl Minecraft {
    /// Builds the game wrapper from the live `Minecraft.getInstance()`.
    /// Succeeds whether or not a world is loaded.
    pub fn new() -> anyhow::Result<Minecraft> {
        let minecraft = mapping()
            .call_static_method(MinecraftClassType::Minecraft, "getInstance", &[])?
            .l()?;
        if minecraft.is_null() {
            return Err(anyhow::anyhow!("Minecraft.getInstance() returned null"));
        }
        let jni_ref = mapping().new_global_ref(minecraft)?;
        let window = Window::new(&jni_ref)?;

        Ok(Minecraft {
            jni_ref,
            window,
            player: RwLock::new(None),
        })
    }

    /// The local player, or `Ok(None)` when not in a world.
    ///
    /// The result is cached and refreshed when the underlying JVM object
    /// changes (a new world join produces a fresh instance).
    pub fn player(&self) -> anyhow::Result<Option<LocalPlayer>> {
        let Some(player_ref) = self.world_object("player", MinecraftClassType::LocalPlayer)? else {
            return Ok(None);
        };

        {
            let cache = self
                .player
                .read()
                .map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
            if let Some(cached) = cache.as_ref() {
                if mapping()
                    .get_env()?
                    .is_same_object(&cached.jni_ref, &player_ref)?
                {
                    return Ok(Some(cached.clone()));
                }
            }
        }

        let player = LocalPlayer::new(player_ref)?;
        *self
            .player
            .write()
            .map_err(|_| anyhow::anyhow!("Lock poisoned"))? = Some(player.clone());
        Ok(Some(player))
    }

    /// The current world / level, or `Ok(None)` when not in a world.
    pub fn world(&self) -> anyhow::Result<Option<World>> {
        Ok(self
            .world_object("level", MinecraftClassType::Level)?
            .map(World::new))
    }

    /// The interaction controller, or `Ok(None)` when not in a world.
    pub fn game_mode(&self) -> anyhow::Result<Option<MultiPlayerGameMode>> {
        Ok(self
            .world_object("gameMode", MinecraftClassType::MultiPlayerGameMode)?
            .map(MultiPlayerGameMode::new))
    }

    /// Whether a world is currently loaded.
    pub fn in_world(&self) -> bool {
        matches!(self.player(), Ok(Some(_)))
    }

    /// Whether no screen (menu / inventory / …) is currently open.
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

    /// Reads a world-scoped object field of `Minecraft`, returning `Ok(None)`
    /// when it is null — i.e. when there is no world loaded.
    fn world_object(
        &self,
        field: &str,
        class: MinecraftClassType,
    ) -> anyhow::Result<Option<GlobalRef>> {
        let obj = mapping()
            .get_field(
                MinecraftClassType::Minecraft,
                self.jni_ref.as_obj(),
                field,
                FieldType::Object(class),
            )?
            .l()?;
        if obj.is_null() {
            return Ok(None);
        }
        Ok(Some(mapping().new_global_ref(obj)?))
    }
}

impl Deref for Minecraft {
    type Target = GlobalRef;

    fn deref(&self) -> &Self::Target {
        &self.jni_ref
    }
}
