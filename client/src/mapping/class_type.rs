use std::fmt;

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MinecraftClassType {
    Minecraft,
    LocalPlayer,
    Level,
    Player,
    Abilities,
    Entity,
    Vec3,
    Window,
    MultiPlayerGameMode,
    Iterable,
    Iterator,
    Mob,
}

impl MinecraftClassType {
    pub fn get_name(&self) -> &str {
        match self {
            MinecraftClassType::Minecraft => "net/minecraft/client/Minecraft",
            MinecraftClassType::LocalPlayer => "net/minecraft/client/player/LocalPlayer",
            MinecraftClassType::Level => "net/minecraft/client/multiplayer/ClientLevel",
            MinecraftClassType::Player => "net/minecraft/world/entity/player/Player",
            MinecraftClassType::Abilities => "net/minecraft/world/entity/player/Abilities",
            MinecraftClassType::Entity => "net/minecraft/world/entity/Entity",
            MinecraftClassType::Vec3 => "net/minecraft/world/phys/Vec3",
            MinecraftClassType::Window => "com/mojang/blaze3d/platform/Window",
            MinecraftClassType::MultiPlayerGameMode => {
                "net/minecraft/client/multiplayer/MultiPlayerGameMode"
            }
            MinecraftClassType::Iterable => "java/lang/Iterable",
            MinecraftClassType::Iterator => "java/util/Iterator",
            MinecraftClassType::Mob => "net/minecraft/world/entity/Mob",
        }
    }
}

// Implement Display for better error messages
impl fmt::Display for MinecraftClassType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}
