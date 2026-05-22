use crate::mapping::entity::{EntityRef, LivingEntityRef};
use crate::module::{KeyboardKey, Module, ModuleCategory, ModuleData, ModuleId, ModuleSetting};
use crate::net::packet::{Packet, PacketAction};
use crate::state::minecraft;
use std::sync::Mutex;

/// Active Freecam state — present only while the module is running.
#[derive(Debug)]
struct FreecamState {
    /// The body's real position. Outbound movement packets are pinned here so
    /// the server keeps seeing the player stand still.
    anchor: (f64, f64, f64),
    /// The free view's current position — the player is teleported here every
    /// tick, so the camera (which follows the player) flies with it.
    position: (f64, f64, f64),
}

/// Detaches the view: the player flies freely through the world — no clip —
/// while the server keeps seeing the body frozen where Freecam was switched on.
///
/// The camera is left attached to the player; it is the *player* that is flown.
/// Its movement packets are rewritten to the anchor in [`handle_packet`], so
/// the body never actually moves as far as the server is concerned.
#[derive(Debug)]
pub struct FreecamModule {
    pub module: ModuleData,
    state: Mutex<Option<FreecamState>>,
}

impl FreecamModule {
    pub fn new() -> Self {
        Self {
            module: ModuleData {
                id: ModuleId::Freecam,
                description: "Flies the view freely while the body stays put".to_string(),
                category: ModuleCategory::Player,
                key_bind: KeyboardKey::KeyNone,
                enabled: false,
                settings: vec![ModuleSetting::Slider {
                    name: "Speed".to_string(),
                    value: 1.0,
                    min: 0.2,
                    max: 3.0,
                }],
            },
            state: Mutex::new(None),
        }
    }

    fn speed(&self) -> f64 {
        self.module
            .get_setting("Speed")
            .and_then(|setting| setting.get_slider_value())
            .unwrap_or(1.0) as f64
    }
}

impl Module for FreecamModule {
    fn on_start(&self) -> anyhow::Result<()> {
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
        let feet = player.get_position()?;
        // Fly with no block collision; the server-side body is frozen via the
        // packet rewrite in `handle_packet`.
        player.set_no_physics(true)?;
        *self.state.lock().unwrap() = Some(FreecamState {
            anchor: (feet.x(), feet.y(), feet.z()),
            position: (feet.x(), feet.y(), feet.z()),
        });
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        let Some(state) = self.state.lock().unwrap().take() else {
            return Ok(());
        };
        if let Some(player) = minecraft().player()? {
            player.set_no_physics(false)?;
            // Snap the body back to where the server has kept it.
            player.set_pos(state.anchor.0, state.anchor.1, state.anchor.2)?;
            player.set_delta_movement(0.0, 0.0, 0.0)?;
        }
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        let Some(player) = minecraft().player()? else {
            return Ok(());
        };
        let mut guard = self.state.lock().unwrap();
        let Some(state) = guard.as_mut() else {
            return Ok(());
        };

        // `Player.tick` resets `noPhysics` to `isSpectator()` every tick — so
        // it must be re-asserted, or the view-blocking overlay (`Player.noPhysics`
        // gate in `ScreenEffectRenderer`) would show as soon as the player is
        // inside a block.
        player.set_no_physics(true)?;

        // Move the free view by the movement keys, in the look direction.
        let (strafe, forward) = player.move_input()?;
        let yaw = (player.get_yaw()? as f64).to_radians();
        let (sin, cos) = yaw.sin_cos();
        let mut wx = strafe as f64 * cos - forward as f64 * sin;
        let mut wz = forward as f64 * cos + strafe as f64 * sin;
        let horizontal = (wx * wx + wz * wz).sqrt();
        if horizontal > 1.0e-4 {
            wx /= horizontal;
            wz /= horizontal;
        } else {
            wx = 0.0;
            wz = 0.0;
        }
        let wy = match (player.is_jumping()?, player.is_shift_key_down()?) {
            (true, false) => 1.0,
            (false, true) => -1.0,
            _ => 0.0,
        };

        let speed = self.speed();
        state.position.0 += wx * speed;
        state.position.1 += wy * speed;
        state.position.2 += wz * speed;

        // Teleport the player — and so the camera — to the free position.
        // `setPos` leaves the previous-tick position alone, so the renderer
        // still interpolates smoothly between ticks.
        player.set_pos(state.position.0, state.position.1, state.position.2)?;
        player.set_delta_movement(0.0, 0.0, 0.0)?;
        Ok(())
    }

    fn handle_packet(&self, packet: &mut Packet) -> PacketAction {
        // Pin every outbound position to the anchor — the server must keep
        // seeing the body where Freecam was switched on, not where it flies.
        let Packet::ServerboundMovePlayer(move_packet) = packet else {
            return PacketAction::Forward;
        };
        if move_packet.has_position {
            if let Ok(guard) = self.state.lock() {
                if let Some(state) = guard.as_ref() {
                    move_packet.x = state.anchor.0;
                    move_packet.y = state.anchor.1;
                    move_packet.z = state.anchor.2;
                    move_packet.on_ground = true;
                }
            }
        }
        PacketAction::Forward
    }

    fn get_module_data(&self) -> &ModuleData {
        &self.module
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.module
    }
}
