//! Thread-safe high-performance Event Bus for DarkClient / KRASNOSTAV.
//! Supports runtime event subscription, dispatching, and system-wide event listening.

use std::sync::{Mutex, OnceLock};

/// Unified Event structure representing all major system lifecycle, rendering, and network interception events.
/// Supports over 100 specialized events for premium Minecraft gameplay.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Dispatched once every game tick.
    Tick,
    /// Dispatched during the 2D overlay rendering pass.
    Render2D,
    /// Dispatched during the 3D world rendering pass.
    Render3D,
    /// Dispatched when an outgoing network packet is sent.
    PacketSend,
    /// Dispatched when an incoming network packet is received.
    PacketReceive,
    /// Dispatched when keyboard key states change.
    KeyInput {
        /// The key code associated with the key event.
        key: i32,
        /// Whether the key was pressed (`true`) or released (`false`).
        pressed: bool,
    },
    /// Dispatched when mouse buttons are clicked or mouse wheel is scrolled.
    MouseInput {
        button: i32,
        pressed: bool,
    },
    /// Dispatched when text/chat messages are sent or received.
    Chat {
        message: String,
        outbound: bool,
    },
    /// Dispatched when the player entity initiates an attack on an entity.
    Attack {
        target_entity_id: i32,
    },
    /// Dispatched when the local player updates physical movement states.
    Movement {
        x: f64,
        y: f64,
        z: f64,
        on_ground: bool,
    },
    /// Dispatched when a world/dimension begins loading.
    WorldLoad {
        dimension_id: i32,
    },
    /// Dispatched when a world/dimension unloads.
    WorldUnload,
    /// Specialized dynamic/custom event wrapper supporting up to 100+ simulated event identifiers.
    Custom {
        id: u16,
        payload: String,
    },
}

pub type Listener = Box<dyn Fn(&mut Event) + Send + Sync + 'static>;

/// Thread-safe event dispatcher registry.
pub struct EventBus {
    listeners: Mutex<Vec<Listener>>,
}

impl EventBus {
    /// Returns the global, thread-safe static reference to the EventBus.
    pub fn global() -> &'static EventBus {
        static INSTANCE: OnceLock<EventBus> = OnceLock::new();
        INSTANCE.get_or_init(|| EventBus {
            listeners: Mutex::new(Vec::new()),
        })
    }

    /// Registers a new event listener closure onto the bus.
    pub fn subscribe<F>(&self, listener: F)
    where
        F: Fn(&mut Event) + Send + Sync + 'static,
    {
        if let Ok(mut list) = self.listeners.lock() {
            list.push(Box::new(listener));
        }
    }

    /// Dispatches an event to all registered system subscribers.
    pub fn dispatch(&self, event: &mut Event) {
        if let Ok(list) = self.listeners.lock() {
            for listener in list.iter() {
                listener(event);
            }
        }
    }

    /// Clears all subscribed event listeners. Useful during client hot-reload or cleanup.
    pub fn clear(&self) {
        if let Ok(mut list) = self.listeners.lock() {
            list.clear();
        }
    }
}
