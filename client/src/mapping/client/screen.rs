//! The Minecraft screen currently open.
//!
//! Combat modules query [`Minecraft::current_screen`](super::minecraft::Minecraft::current_screen)
//! and stand down while a screen is open — the player cannot fight with an
//! inventory, chest, crafting table or chat in front of them.

/// Which Minecraft screen is currently open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// No screen — the player is in the world.
    None,
    /// The chat input screen.
    Chat,
    /// The player inventory (survival or creative).
    Inventory,
    /// A container UI — chest, barrel, furnace, …
    Container,
    /// A crafting-table UI.
    Crafting,
    /// A non-gameplay menu — pause, options, …
    Menu,
    /// A screen is open but its kind was not recognized — treated as open.
    Unknown,
}

impl Screen {
    /// Whether a screen is open, i.e. modules should stand down.
    pub fn is_open(self) -> bool {
        self != Screen::None
    }
}
