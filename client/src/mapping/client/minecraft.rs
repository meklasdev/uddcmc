use crate::mapping::client::gamemode::MultiPlayerGameMode;
use crate::mapping::client::window::Window;
use crate::mapping::client::world::World;
use crate::mapping::entity::player::{Abilities, LocalPlayer};
use crate::mapping::entity::Entity;
use crate::mapping::{FieldType, GameContext, Mapping, MinecraftClassType};
use jni::objects::GlobalRef;
use log::error;
use std::ops::Deref;
use std::sync::{Arc, OnceLock, RwLock};

#[derive(Debug)]
pub struct Minecraft {
    pub jni_ref: GlobalRef,
    mapping: Mapping,
    player: RwLock<LocalPlayer>,
    #[allow(dead_code)]
    pub world: World,
    pub window: Window,
    pub game_mode: MultiPlayerGameMode,
}

impl GameContext for Minecraft {}

impl Minecraft {
    pub fn instance() -> &'static Minecraft {
        static INSTANCE: OnceLock<Arc<Minecraft>> = OnceLock::new();

        INSTANCE.get_or_init(|| unsafe {
            Arc::new(Minecraft::new().unwrap_or_else(|e| {
                error!("Failed to initialize Minecraft: {:?}", e);
                panic!("Failed to initialize Minecraft");
            }))
        })
    }

    unsafe fn new() -> anyhow::Result<Minecraft> {
        let mapping = Mapping::new()?;
        let minecraft = mapping
            .call_static_method(MinecraftClassType::Minecraft, "getInstance", &[])?
            .l()?;

        if minecraft.is_null() {
            error!("Minecraft is null")
        }

        let minecraft = mapping.new_global_ref(minecraft)?;

        let player = LocalPlayer::new(&minecraft, &mapping)?;
        let world = World::new(&minecraft, &mapping)?;
        let window = Window::new(&minecraft, &mapping)?;
        let game_mode = MultiPlayerGameMode::new(
            mapping.new_global_ref(
                mapping
                    .get_field(
                        MinecraftClassType::Minecraft,
                        minecraft.as_obj(),
                        "gameMode",
                        FieldType::Object(MinecraftClassType::MultiPlayerGameMode, &mapping),
                    )?
                    .l()?,
            )?,
        );

        Ok(Minecraft {
            jni_ref: minecraft,
            mapping,
            player: RwLock::new(player),
            world,
            window,
            game_mode,
        })
    }

    pub fn get_mapping(&self) -> &Mapping {
        &self.mapping
    }

    pub fn get_player(&self) -> anyhow::Result<LocalPlayer> {
        let player_obj = self
            .mapping
            .get_field(
                MinecraftClassType::Minecraft,
                self.jni_ref.as_obj(),
                "player",
                FieldType::Object(MinecraftClassType::LocalPlayer, &self.mapping),
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
            if self
                .mapping
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

        let jni_ref = self.mapping.new_global_ref(player_obj)?;
        *write_guard = LocalPlayer {
            jni_ref: jni_ref.clone(),
            abilities: Abilities::new(jni_ref.clone(), &self.mapping)?,
            entity: Entity::new(jni_ref),
        };

        Ok(write_guard.clone())
    }
    pub fn current_screen_is_null(&self) -> bool {
        if let Ok(screen_obj) = self.mapping.get_field(
            MinecraftClassType::Minecraft,
            self.jni_ref.as_obj(),
            "screen",
            FieldType::Object(MinecraftClassType::Screen, &self.mapping),
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
