//! ESP overlay: 3D wireframe boxes for players, mobs and containers.
//!
//! # Performance model
//!
//! The expensive part of an ESP is the JNI traffic — reading every entity's
//! position, type and stats. Doing that per frame is what tanks the FPS, so it
//! is **decoupled from the frame rate**:
//!
//! * [`gather`] (all the JNI work) runs at most ~20 Hz, throttled by wall time.
//! * Every frame only reads the camera (~6 JNI calls) and does pure-CPU
//!   projection + egui drawing.
//! * Positions are interpolated between the last two gathers, so boxes move
//!   smoothly even though the data behind them updates at 20 Hz.
//! * Every JNI scope is wrapped in a local-reference frame: without this the
//!   JVM local-ref table grows unbounded, which is itself a slow FPS killer.
//!
//! The chest scan is heavier (it walks loaded chunks) so it runs even rarer,
//! every [`CHEST_SCAN_INTERVAL`].

use crate::mapping::{FieldType, Mapping, MinecraftClassType as Cls};
use crate::module::ModuleSetting;
use crate::state::{client, mapping, minecraft};
use egui::{
    pos2, vec2, Align2, Color32, Context, FontId, Id, LayerId, Order, Painter, Pos2, Rect,
    Rounding, Stroke,
};
use jni::objects::{GlobalRef, JObject, JValue};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Minimum wall-time between two entity gathers (≈15 Hz). Interpolation keeps
/// the boxes smooth between gathers, so a low rate costs nothing visually.
const GATHER_INTERVAL: Duration = Duration::from_millis(66);
/// Wall-time between two chest scans — chests do not move, so this is rare.
const CHEST_SCAN_INTERVAL: Duration = Duration::from_secs(2);
/// Half-extent, in chunks, of the area scanned for containers.
const CHEST_CHUNK_RADIUS: i32 = 8;
/// Box edge thickness, in points.
const LINE_WIDTH: f32 = 1.6;

// --- math ------------------------------------------------------------------

/// A plain 3D vector — kept separate from egui/JNI types so the projection
/// math stays allocation- and dependency-free.
#[derive(Debug, Clone, Copy)]
struct V3 {
    x: f64,
    y: f64,
    z: f64,
}

impl V3 {
    fn sub(self, o: V3) -> V3 {
        V3 {
            x: self.x - o.x,
            y: self.y - o.y,
            z: self.z - o.z,
        }
    }

    fn dot(self, o: V3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    fn cross(self, o: V3) -> V3 {
        V3 {
            x: self.y * o.z - self.z * o.y,
            y: self.z * o.x - self.x * o.z,
            z: self.x * o.y - self.y * o.x,
        }
    }

    fn lerp(self, o: V3, t: f64) -> V3 {
        V3 {
            x: self.x + (o.x - self.x) * t,
            y: self.y + (o.y - self.y) * t,
            z: self.z + (o.z - self.z) * t,
        }
    }

    fn length(self) -> f64 {
        self.dot(self).sqrt()
    }
}

/// A camera, reduced to exactly what world→screen projection needs.
struct View {
    cam: V3,
    fwd: V3,
    right: V3,
    up: V3,
    tan_x: f64,
    tan_y: f64,
    w: f32,
    h: f32,
}

impl View {
    /// Projects a world point to logical screen coordinates, or `None` if it
    /// lies behind the camera (where a perspective divide is meaningless).
    fn project(&self, p: V3) -> Option<Pos2> {
        let r = p.sub(self.cam);
        let depth = r.dot(self.fwd);
        if depth < 0.05 {
            return None;
        }
        let ndc_x = (r.dot(self.right) / depth) / self.tan_x;
        let ndc_y = (r.dot(self.up) / depth) / self.tan_y;
        let sx = (ndc_x * 0.5 + 0.5) * self.w as f64;
        let sy = (0.5 - ndc_y * 0.5) * self.h as f64;
        Some(pos2(sx as f32, sy as f32))
    }
}

/// Builds a [`View`] from Minecraft's camera state.
fn build_view(cam: V3, yaw_deg: f32, pitch_deg: f32, fov_deg: f64, w: f32, h: f32) -> View {
    let yaw = (yaw_deg as f64).to_radians();
    let pitch = (pitch_deg as f64).to_radians();
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();

    // Minecraft yaw: 0° faces +Z, increasing clockwise. Pitch: positive looks
    // down. The forward vector follows directly from those conventions.
    let fwd = V3 {
        x: -sy * cp,
        y: -sp,
        z: cy * cp,
    };
    let right = V3 {
        x: -cy,
        y: 0.0,
        z: -sy,
    };
    let up = right.cross(fwd);

    let tan_y = (fov_deg.to_radians() * 0.5).tan();
    let tan_x = tan_y * (w / h) as f64;

    View {
        cam,
        fwd,
        right,
        up,
        tan_x,
        tan_y,
        w,
        h,
    }
}

// --- gathered snapshot -----------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetKind {
    Player,
    Mob,
}

/// One living target, as captured by the last [`gather`].
#[derive(Debug, Clone)]
struct EntityTarget {
    id: i32,
    kind: TargetKind,
    /// Position at the previous gather — the start of the interpolation.
    prev: V3,
    /// Position at the latest gather — the end of the interpolation.
    pos: V3,
    width: f64,
    height: f64,
    name: String,
    health: f32,
    max_health: f32,
}

/// One container block, by its lower-corner block coordinates.
#[derive(Debug, Clone, Copy)]
struct ChestTarget {
    pos: V3,
}

/// Cross-frame ESP state: the cached camera handles and the latest snapshot.
struct EspState {
    camera: Option<GlobalRef>,
    entities: Vec<EntityTarget>,
    chests: Vec<ChestTarget>,
    prev_gather: Option<Instant>,
    last_gather: Option<Instant>,
    last_chest_scan: Option<Instant>,
    /// Field of view currently used for projection — eased toward `target_fov`
    /// each frame so flying / sprinting transitions do not snap the boxes.
    fov: f64,
    /// Field of view the game is heading to, refreshed each gather.
    target_fov: f64,
    /// Set once camera resolution has failed, so the failure is logged once.
    camera_logged: bool,
}

impl EspState {
    fn new() -> Self {
        EspState {
            camera: None,
            entities: Vec::new(),
            chests: Vec::new(),
            prev_gather: None,
            last_gather: None,
            last_chest_scan: None,
            fov: 70.0,
            target_fov: 70.0,
            camera_logged: false,
        }
    }
}

fn state() -> &'static Mutex<EspState> {
    static STATE: OnceLock<Mutex<EspState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(EspState::new()))
}

// --- module configuration --------------------------------------------------

/// Per-category drawing options, snapshotted from the modules once per frame.
struct EntityCfg {
    enabled: bool,
    color: Color32,
    show_name: bool,
    show_distance: bool,
    show_health: bool,
    /// Maximum distance, in blocks, an entity is drawn (and processed) at.
    range: f32,
}

struct ChestCfg {
    enabled: bool,
    color: Color32,
    show_distance: bool,
}

struct EspConfig {
    player: EntityCfg,
    mob: EntityCfg,
    chest: ChestCfg,
}

impl EspConfig {
    fn any_enabled(&self) -> bool {
        self.player.enabled || self.mob.enabled || self.chest.enabled
    }
}

fn color_setting(data: &crate::module::ModuleData, name: &str, fallback: Color32) -> Color32 {
    match data.get_setting(name) {
        Some(ModuleSetting::Color { value, .. }) => Color32::from_rgba_unmultiplied(
            (value[0] * 255.0) as u8,
            (value[1] * 255.0) as u8,
            (value[2] * 255.0) as u8,
            (value[3] * 255.0).max(40.0) as u8,
        ),
        _ => fallback,
    }
}

fn toggle_setting(data: &crate::module::ModuleData, name: &str, fallback: bool) -> bool {
    data.get_setting(name)
        .and_then(ModuleSetting::get_toggle_value)
        .unwrap_or(fallback)
}

fn slider_setting(data: &crate::module::ModuleData, name: &str, fallback: f32) -> f32 {
    data.get_setting(name)
        .and_then(ModuleSetting::get_slider_value)
        .unwrap_or(fallback)
}

/// Reads the three ESP modules' state with a single lock pass.
fn read_config() -> EspConfig {
    let disabled_entity = || EntityCfg {
        enabled: false,
        color: Color32::WHITE,
        show_name: false,
        show_distance: false,
        show_health: false,
        range: 0.0,
    };
    let mut cfg = EspConfig {
        player: disabled_entity(),
        mob: disabled_entity(),
        chest: ChestCfg {
            enabled: false,
            color: Color32::WHITE,
            show_distance: false,
        },
    };

    let registry = match client().modules.read() {
        Ok(guard) => guard,
        Err(_) => return cfg,
    };

    if let Some(arc) = registry.get("Player ESP") {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            cfg.player = EntityCfg {
                enabled: data.enabled,
                color: color_setting(data, "Color", Color32::from_rgb(255, 70, 70)),
                show_name: toggle_setting(data, "Name", true),
                show_distance: toggle_setting(data, "Distance", true),
                show_health: toggle_setting(data, "Health", true),
                range: slider_setting(data, "Range", 64.0),
            };
        }
    }
    if let Some(arc) = registry.get("Mob ESP") {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            cfg.mob = EntityCfg {
                enabled: data.enabled,
                color: color_setting(data, "Color", Color32::from_rgb(255, 215, 50)),
                show_name: toggle_setting(data, "Name", true),
                show_distance: toggle_setting(data, "Distance", true),
                show_health: toggle_setting(data, "Health", true),
                range: slider_setting(data, "Range", 64.0),
            };
        }
    }
    if let Some(arc) = registry.get("Chest ESP") {
        if let Ok(module) = arc.lock() {
            let data = module.get_module_data();
            cfg.chest = ChestCfg {
                enabled: data.enabled,
                color: color_setting(data, "Color", Color32::from_rgb(255, 140, 30)),
                show_distance: toggle_setting(data, "Distance", true),
            };
        }
    }

    cfg
}

// --- public entry point ----------------------------------------------------

/// Draws the whole ESP overlay for the current frame.
///
/// Called once per frame from the overlay renderer; cheap when every ESP
/// module is disabled.
pub fn draw(ctx: &Context) {
    let cfg = read_config();
    let mut state = state().lock().unwrap();

    if !cfg.any_enabled() {
        state.entities.clear();
        state.chests.clear();
        return;
    }

    let now = Instant::now();
    if state
        .last_gather
        .map_or(true, |t| now - t >= GATHER_INTERVAL)
    {
        gather(&mut state, &cfg, now);
    }

    let view = match read_view(&mut state, ctx) {
        Some(view) => view,
        None => return,
    };
    let t = interp_factor(&state, now);

    let painter = ctx.layer_painter(LayerId::new(Order::Background, Id::new("esp_overlay")));

    for entity in &state.entities {
        draw_entity(&painter, &view, entity, t, &cfg);
    }
    for chest in &state.chests {
        draw_chest(&painter, &view, chest, &cfg);
    }
}

/// Fraction of the way from the previous gather to the latest one.
fn interp_factor(state: &EspState, now: Instant) -> f64 {
    match (state.prev_gather, state.last_gather) {
        (Some(prev), Some(last)) => {
            let span = (last - prev).as_secs_f64();
            if span <= 1e-4 {
                1.0
            } else {
                ((now - last).as_secs_f64() / span).clamp(0.0, 1.0)
            }
        }
        _ => 1.0,
    }
}

// --- camera ----------------------------------------------------------------

/// Resolves the current camera into a [`View`], caching the JNI handles.
fn read_view(state: &mut EspState, ctx: &Context) -> Option<View> {
    let mapping = mapping();

    if state.camera.is_none() {
        match init_camera(mapping) {
            Ok(camera) => state.camera = Some(camera),
            Err(e) => {
                log::debug!("ESP: camera unavailable: {e}");
                return None;
            }
        }
    }

    let cam = state.camera.clone()?;
    let rect = ctx.screen_rect();
    if rect.width() < 1.0 || rect.height() < 1.0 {
        return None;
    }

    let mut env = mapping.get_env().ok()?;
    // The camera state is read from `Camera`'s fields, not getter methods:
    // method names churn between versions, the plain fields are far stabler.
    let read = env.with_local_frame(32, |_| -> anyhow::Result<(V3, f32, f32)> {
        let pos = mapping
            .get_field(
                Cls::Camera,
                cam.as_obj(),
                "position",
                FieldType::Object(Cls::Vec3),
            )?
            .l()?;
        let cam_pos = V3 {
            x: mapping
                .get_field(Cls::Vec3, &pos, "x", FieldType::Double)?
                .d()?,
            y: mapping
                .get_field(Cls::Vec3, &pos, "y", FieldType::Double)?
                .d()?,
            z: mapping
                .get_field(Cls::Vec3, &pos, "z", FieldType::Double)?
                .d()?,
        };
        let yaw = mapping
            .get_field(Cls::Camera, cam.as_obj(), "yRot", FieldType::Float)?
            .f()?;
        let pitch = mapping
            .get_field(Cls::Camera, cam.as_obj(), "xRot", FieldType::Float)?
            .f()?;
        Ok((cam_pos, yaw, pitch))
    });

    let (cam_pos, yaw, pitch) = match read {
        Ok(values) => values,
        Err(error) => {
            if !state.camera_logged {
                state.camera_logged = true;
                log::warn!("ESP: camera read failed: {error}");
            }
            return None;
        }
    };

    // Ease the FOV toward its target — flying / sprinting transitions ramp
    // smoothly instead of snapping the boxes (Minecraft eases it too, so an
    // instant jump here would show up as a stutter).
    let dt = ctx.input(|input| input.stable_dt).clamp(0.0, 0.1) as f64;
    let blend = 1.0 - 0.5_f64.powf(dt / 0.05);
    state.fov += (state.target_fov - state.fov) * blend;

    Some(build_view(
        cam_pos,
        yaw,
        pitch,
        state.fov,
        rect.width(),
        rect.height(),
    ))
}

/// Fetches the (session-stable) `Camera` handle via the game renderer.
fn init_camera(mapping: &Mapping) -> anyhow::Result<GlobalRef> {
    let mc = minecraft();
    let mut env = mapping.get_env()?;
    env.with_local_frame(16, |_| -> anyhow::Result<GlobalRef> {
        let renderer = mapping
            .get_field(
                Cls::Minecraft,
                mc.jni_ref.as_obj(),
                "gameRenderer",
                FieldType::Object(Cls::GameRenderer),
            )?
            .l()?;
        let camera = mapping
            .call_method(Cls::GameRenderer, &renderer, "getMainCamera", &[])?
            .l()?;
        mapping.new_global_ref(camera)
    })
}

/// Reads the vertical field of view, in degrees, Minecraft is rendering with:
/// the options value scaled by the flying / sprinting modifiers Minecraft
/// itself applies. Without them the box drifts off entities while either is
/// active (`GameRenderer.getFov` would give this directly, but its signature
/// is not stable across versions).
fn read_fov(mapping: &Mapping) -> f64 {
    let base = match read_option_fov(mapping) {
        Ok(fov) if fov.is_finite() && (1.0..=179.0).contains(&fov) => fov,
        _ => 70.0,
    };
    (base * fov_modifier(mapping)).clamp(1.0, 179.0)
}

/// The FOV multiplier Minecraft applies on top of the options value: ×1.1
/// while flying and ≈×1.15 while sprinting — the constants from
/// `Player.getFieldOfViewModifier`.
fn fov_modifier(mapping: &Mapping) -> f64 {
    let player = match minecraft().player() {
        Ok(Some(player)) => player,
        _ => return 1.0,
    };

    let mut modifier = 1.0;

    let flying = mapping
        .get_field(
            Cls::Abilities,
            player.abilities.jni_ref.as_obj(),
            "flying",
            FieldType::Boolean,
        )
        .ok()
        .and_then(|value| value.z().ok())
        .unwrap_or(false);
    if flying {
        modifier *= 1.1;
    }

    let sprinting = mapping
        .call_method(
            Cls::Entity,
            player.entity.jni_ref.as_obj(),
            "isSprinting",
            &[],
        )
        .ok()
        .and_then(|value| value.z().ok())
        .unwrap_or(false);
    if sprinting {
        modifier *= 1.15;
    }

    modifier
}

/// Reads the raw FOV slider value from the game options.
fn read_option_fov(mapping: &Mapping) -> anyhow::Result<f64> {
    let mc = minecraft();
    let mut env = mapping.get_env()?;
    env.with_local_frame(16, |_| -> anyhow::Result<f64> {
        let options = mapping
            .get_field(
                Cls::Minecraft,
                mc.jni_ref.as_obj(),
                "options",
                FieldType::Object(Cls::Options),
            )?
            .l()?;
        let option = mapping
            .get_field(
                Cls::Options,
                &options,
                "fov",
                FieldType::Object(Cls::OptionInstance),
            )?
            .l()?;
        let value = mapping
            .call_method(Cls::OptionInstance, &option, "get", &[])?
            .l()?;
        let fov = mapping
            .call_method(Cls::Integer, &value, "intValue", &[])?
            .i()?;
        Ok(fov as f64)
    })
}

// --- gather ----------------------------------------------------------------

/// Refreshes the snapshot: entities every call, chests on their own schedule.
fn gather(state: &mut EspState, cfg: &EspConfig, now: Instant) {
    state.prev_gather = state.last_gather;
    state.last_gather = Some(now);
    state.target_fov = read_fov(mapping());

    if cfg.player.enabled || cfg.mob.enabled {
        let mut range = 0.0_f32;
        if cfg.player.enabled {
            range = range.max(cfg.player.range);
        }
        if cfg.mob.enabled {
            range = range.max(cfg.mob.range);
        }
        let range_sq = (range as f64) * (range as f64);

        let previous = std::mem::take(&mut state.entities);
        match gather_entities(&previous, range_sq, cfg.player.enabled, cfg.mob.enabled) {
            Ok(list) => state.entities = list,
            Err(e) => log::debug!("ESP: entity gather failed: {e}"),
        }
    } else {
        state.entities.clear();
    }

    if cfg.chest.enabled {
        let due = state
            .last_chest_scan
            .map_or(true, |t| now - t >= CHEST_SCAN_INTERVAL);
        if due {
            state.last_chest_scan = Some(now);
            match gather_chests() {
                Ok(list) => state.chests = list,
                Err(e) => log::debug!("ESP: chest scan failed: {e}"),
            }
        }
    } else {
        state.chests.clear();
        state.last_chest_scan = None;
    }
}

/// Walks `Level.entitiesForRendering()` once, classifying players and mobs.
fn gather_entities(
    previous: &[EntityTarget],
    range_sq: f64,
    want_player: bool,
    want_mob: bool,
) -> anyhow::Result<Vec<EntityTarget>> {
    let mc = minecraft();
    let mapping = mapping();

    // Carry positions forward so the new snapshot can interpolate from them.
    let prev_pos: HashMap<i32, V3> = previous.iter().map(|e| (e.id, e.pos)).collect();

    let mut env = mapping.get_env()?;
    let mut out: Vec<EntityTarget> = Vec::new();

    env.with_local_frame(32, |env| -> anyhow::Result<()> {
        let (local_id, player_pos) = {
            let Some(player) = mc.player()? else {
                return Ok(());
            };
            let id = mapping
                .call_method(Cls::Entity, player.entity.jni_ref.as_obj(), "getId", &[])?
                .i()?;
            let pos = player.entity.get_position()?;
            (
                id,
                V3 {
                    x: pos.0,
                    y: pos.1,
                    z: pos.2,
                },
            )
        };

        let level = mapping
            .get_field(
                Cls::Minecraft,
                mc.jni_ref.as_obj(),
                "level",
                FieldType::Object(Cls::Level),
            )?
            .l()?;
        if level.is_null() {
            return Ok(());
        }

        let iterable = mapping
            .call_method(Cls::Level, &level, "entitiesForRendering", &[])?
            .l()?;
        let iterator = mapping
            .call_method(Cls::Iterable, &iterable, "iterator", &[])?
            .l()?;

        loop {
            if !mapping
                .call_method(Cls::Iterator, &iterator, "hasNext", &[])?
                .z()?
            {
                break;
            }
            // One frame per entity bounds the local-ref table no matter how
            // many entities the world contains.
            let target = env.with_local_frame(64, |_| -> anyhow::Result<Option<EntityTarget>> {
                let entity = mapping
                    .call_method(Cls::Iterator, &iterator, "next", &[])?
                    .l()?;
                Ok(process_entity(
                    mapping,
                    &entity,
                    local_id,
                    player_pos,
                    range_sq,
                    want_player,
                    want_mob,
                    &prev_pos,
                ))
            })?;
            if let Some(target) = target {
                out.push(target);
            }
        }
        Ok(())
    })?;

    Ok(out)
}

/// Turns one entity object into an [`EntityTarget`], or `None` if it is not a
/// wanted target. Errors are swallowed per-field so one bad entity cannot
/// abort the whole gather.
#[allow(clippy::too_many_arguments)]
fn process_entity(
    mapping: &Mapping,
    entity: &JObject,
    local_id: i32,
    player_pos: V3,
    range_sq: f64,
    want_player: bool,
    want_mob: bool,
    prev_pos: &HashMap<i32, V3>,
) -> Option<EntityTarget> {
    // Cheap distance gate first — a far entity then costs just this one JNI
    // call. Skipped entirely if `distanceToSqr` is not exposed by this build.
    if let Some(dist_sq) = mapping
        .call_method(
            Cls::Entity,
            entity,
            "distanceToSqr",
            &[
                JValue::Double(player_pos.x),
                JValue::Double(player_pos.y),
                JValue::Double(player_pos.z),
            ],
        )
        .ok()
        .and_then(|value| value.d().ok())
    {
        if dist_sq > range_sq {
            return None;
        }
    }

    let kind = if want_player && mapping.is_instance_of(Cls::Player, entity).unwrap_or(false) {
        TargetKind::Player
    } else if want_mob && mapping.is_instance_of(Cls::Mob, entity).unwrap_or(false) {
        TargetKind::Mob
    } else {
        return None;
    };

    let id = mapping
        .call_method(Cls::Entity, entity, "getId", &[])
        .ok()?
        .i()
        .ok()?;
    if id == local_id {
        return None;
    }

    let pos = read_vec3(mapping, entity, "position")?;
    let width = mapping
        .call_method(Cls::Entity, entity, "getBbWidth", &[])
        .ok()?
        .f()
        .ok()? as f64;
    let height = mapping
        .call_method(Cls::Entity, entity, "getBbHeight", &[])
        .ok()?
        .f()
        .ok()? as f64;

    let name = read_name(mapping, entity).unwrap_or_default();
    let (health, max_health) = read_health(mapping, entity).unwrap_or((0.0, 0.0));

    Some(EntityTarget {
        id,
        kind,
        prev: prev_pos.get(&id).copied().unwrap_or(pos),
        pos,
        width,
        height,
        name,
        health,
        max_health,
    })
}

/// Calls a no-arg `Vec3`-returning method and reads its `x`/`y`/`z`.
fn read_vec3(mapping: &Mapping, obj: &JObject, method: &str) -> Option<V3> {
    let vec3 = mapping
        .call_method(Cls::Entity, obj, method, &[])
        .ok()?
        .l()
        .ok()?;
    Some(V3 {
        x: mapping
            .get_field(Cls::Vec3, &vec3, "x", FieldType::Double)
            .ok()?
            .d()
            .ok()?,
        y: mapping
            .get_field(Cls::Vec3, &vec3, "y", FieldType::Double)
            .ok()?
            .d()
            .ok()?,
        z: mapping
            .get_field(Cls::Vec3, &vec3, "z", FieldType::Double)
            .ok()?
            .d()
            .ok()?,
    })
}

/// Reads an entity's display name via `getName().getString()`.
fn read_name(mapping: &Mapping, entity: &JObject) -> anyhow::Result<String> {
    let component = mapping
        .call_method(Cls::Entity, entity, "getName", &[])?
        .l()?;
    if component.is_null() {
        return Ok(String::new());
    }
    let string = mapping
        .call_method(Cls::Component, &component, "getString", &[])?
        .l()?;
    let mut name = mapping.get_string(string)?;
    if name.chars().count() > 24 {
        name = name.chars().take(24).collect();
    }
    Ok(name)
}

/// Reads `(health, maxHealth)` for a living entity.
fn read_health(mapping: &Mapping, entity: &JObject) -> anyhow::Result<(f32, f32)> {
    let health = mapping
        .call_method(Cls::LivingEntity, entity, "getHealth", &[])?
        .f()?;
    let max_health = mapping
        .call_method(Cls::LivingEntity, entity, "getMaxHealth", &[])?
        .f()?;
    Ok((health, max_health))
}

/// Scans loaded chunks around the player for container block entities.
fn gather_chests() -> anyhow::Result<Vec<ChestTarget>> {
    let mc = minecraft();
    let mapping = mapping();

    let mut env = mapping.get_env()?;
    let mut out: Vec<ChestTarget> = Vec::new();

    env.with_local_frame(32, |env| -> anyhow::Result<()> {
        let level = mapping
            .get_field(
                Cls::Minecraft,
                mc.jni_ref.as_obj(),
                "level",
                FieldType::Object(Cls::Level),
            )?
            .l()?;
        if level.is_null() {
            return Ok(());
        }

        let Some(player) = mc.player()? else {
            return Ok(());
        };
        let player_pos = player.entity.get_position()?;
        let pcx = (player_pos.0 / 16.0).floor() as i32;
        let pcz = (player_pos.2 / 16.0).floor() as i32;

        for cx in (pcx - CHEST_CHUNK_RADIUS)..=(pcx + CHEST_CHUNK_RADIUS) {
            for cz in (pcz - CHEST_CHUNK_RADIUS)..=(pcz + CHEST_CHUNK_RADIUS) {
                // One frame per chunk keeps the block-entity locals bounded.
                env.with_local_frame(128, |_| -> anyhow::Result<()> {
                    scan_chunk(mapping, &level, cx, cz, &mut out)
                })?;
            }
        }
        Ok(())
    })?;

    Ok(out)
}

/// Adds every container block entity of one chunk to `out`.
fn scan_chunk(
    mapping: &Mapping,
    level: &JObject,
    cx: i32,
    cz: i32,
    out: &mut Vec<ChestTarget>,
) -> anyhow::Result<()> {
    let chunk = mapping
        .call_method(
            Cls::LevelReader,
            level,
            "getChunk",
            &[JValue::Int(cx), JValue::Int(cz)],
        )?
        .l()?;
    if chunk.is_null() {
        return Ok(());
    }

    let map = mapping
        .call_method(Cls::LevelChunk, &chunk, "getBlockEntities", &[])?
        .l()?;
    if map.is_null() {
        return Ok(());
    }
    let values = mapping.call_method(Cls::Map, &map, "values", &[])?.l()?;
    let iterator = mapping
        .call_method(Cls::Iterable, &values, "iterator", &[])?
        .l()?;

    loop {
        if !mapping
            .call_method(Cls::Iterator, &iterator, "hasNext", &[])?
            .z()?
        {
            break;
        }
        let block_entity = mapping
            .call_method(Cls::Iterator, &iterator, "next", &[])?
            .l()?;
        if is_container(mapping, &block_entity) {
            if let Some(pos) = block_entity_pos(mapping, &block_entity) {
                out.push(ChestTarget { pos });
            }
        }
    }
    Ok(())
}

/// True for chest / trapped chest / ender chest / barrel / shulker box.
fn is_container(mapping: &Mapping, block_entity: &JObject) -> bool {
    // `ChestBlockEntity` already covers trapped chests (a subclass).
    const KINDS: [Cls; 4] = [
        Cls::ChestBlockEntity,
        Cls::EnderChestBlockEntity,
        Cls::BarrelBlockEntity,
        Cls::ShulkerBoxBlockEntity,
    ];
    KINDS
        .iter()
        .any(|&kind| mapping.is_instance_of(kind, block_entity).unwrap_or(false))
}

/// Reads a block entity's `BlockPos` as a [`V3`].
fn block_entity_pos(mapping: &Mapping, block_entity: &JObject) -> Option<V3> {
    let block_pos = mapping
        .call_method(Cls::BlockEntity, block_entity, "getBlockPos", &[])
        .ok()?
        .l()
        .ok()?;
    let axis = |name: &str| -> Option<f64> {
        Some(
            mapping
                .call_method(Cls::Vec3i, &block_pos, name, &[])
                .ok()?
                .i()
                .ok()? as f64,
        )
    };
    Some(V3 {
        x: axis("getX")?,
        y: axis("getY")?,
        z: axis("getZ")?,
    })
}

// --- drawing ---------------------------------------------------------------

/// The 12 edges of a box, as index pairs into an 8-corner array.
const EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0), // bottom
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4), // top
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7), // verticals
];

/// The 8 corners of an axis-aligned box `[min, max]`.
fn box_corners(min: V3, max: V3) -> [V3; 8] {
    [
        V3 {
            x: min.x,
            y: min.y,
            z: min.z,
        },
        V3 {
            x: max.x,
            y: min.y,
            z: min.z,
        },
        V3 {
            x: max.x,
            y: min.y,
            z: max.z,
        },
        V3 {
            x: min.x,
            y: min.y,
            z: max.z,
        },
        V3 {
            x: min.x,
            y: max.y,
            z: min.z,
        },
        V3 {
            x: max.x,
            y: max.y,
            z: min.z,
        },
        V3 {
            x: max.x,
            y: max.y,
            z: max.z,
        },
        V3 {
            x: min.x,
            y: max.y,
            z: max.z,
        },
    ]
}

/// Draws a wireframe box and returns its 2D screen bounds (for label
/// placement), or `None` if no corner is in front of the camera.
fn draw_wire_box(
    painter: &Painter,
    view: &View,
    corners: &[V3; 8],
    color: Color32,
) -> Option<Rect> {
    let projected: [Option<Pos2>; 8] = std::array::from_fn(|i| view.project(corners[i]));
    let stroke = Stroke::new(LINE_WIDTH, color);

    for &(a, b) in &EDGES {
        if let (Some(pa), Some(pb)) = (projected[a], projected[b]) {
            painter.line_segment([pa, pb], stroke);
        }
    }

    let mut bounds: Option<Rect> = None;
    for point in projected.into_iter().flatten() {
        bounds = Some(match bounds {
            Some(rect) => rect.union(Rect::from_min_max(point, point)),
            None => Rect::from_min_max(point, point),
        });
    }
    bounds
}

fn draw_entity(painter: &Painter, view: &View, entity: &EntityTarget, t: f64, cfg: &EspConfig) {
    let icfg = match entity.kind {
        TargetKind::Player => &cfg.player,
        TargetKind::Mob => &cfg.mob,
    };

    let feet = entity.prev.lerp(entity.pos, t);
    let half = entity.width * 0.5;
    let corners = box_corners(
        V3 {
            x: feet.x - half,
            y: feet.y,
            z: feet.z - half,
        },
        V3 {
            x: feet.x + half,
            y: feet.y + entity.height,
            z: feet.z + half,
        },
    );

    let rect = match draw_wire_box(painter, view, &corners, icfg.color) {
        Some(rect) => rect,
        None => return,
    };

    if icfg.show_health && entity.max_health > 0.0 {
        draw_health_bar(painter, rect, entity.health / entity.max_health);
    }
    if icfg.show_name && !entity.name.is_empty() {
        draw_label(
            painter,
            pos2(rect.center().x, rect.top() - 7.0),
            Align2::CENTER_BOTTOM,
            &entity.name,
            icfg.color,
        );
    }
    if icfg.show_distance {
        let distance = feet.sub(view.cam).length();
        draw_label(
            painter,
            pos2(rect.center().x, rect.bottom() + 3.0),
            Align2::CENTER_TOP,
            &format!("{distance:.0}m"),
            Color32::from_rgb(214, 216, 224),
        );
    }
}

fn draw_chest(painter: &Painter, view: &View, chest: &ChestTarget, cfg: &EspConfig) {
    let corners = box_corners(
        chest.pos,
        V3 {
            x: chest.pos.x + 1.0,
            y: chest.pos.y + 1.0,
            z: chest.pos.z + 1.0,
        },
    );
    let rect = match draw_wire_box(painter, view, &corners, cfg.chest.color) {
        Some(rect) => rect,
        None => return,
    };

    if cfg.chest.show_distance {
        let center = V3 {
            x: chest.pos.x + 0.5,
            y: chest.pos.y + 0.5,
            z: chest.pos.z + 0.5,
        };
        let distance = center.sub(view.cam).length();
        draw_label(
            painter,
            pos2(rect.center().x, rect.bottom() + 3.0),
            Align2::CENTER_TOP,
            &format!("{distance:.0}m"),
            cfg.chest.color,
        );
    }
}

/// A thin health bar straddling the top edge of `rect`, red→yellow→green.
fn draw_health_bar(painter: &Painter, rect: Rect, fraction: f32) {
    let fraction = fraction.clamp(0.0, 1.0);
    let top = rect.top() - 4.0;
    let bottom = rect.top() - 1.5;
    let background = Rect::from_min_max(pos2(rect.left(), top), pos2(rect.right(), bottom));
    painter.rect_filled(background, Rounding::ZERO, Color32::from_black_alpha(190));

    let fill_width = rect.width() * fraction;
    let fill = Rect::from_min_max(
        pos2(rect.left(), top),
        pos2(rect.left() + fill_width, bottom),
    );
    painter.rect_filled(fill, Rounding::ZERO, health_color(fraction));
}

/// Interpolates red → yellow → green across `0..=1`.
fn health_color(fraction: f32) -> Color32 {
    let (r, g) = if fraction < 0.5 {
        (255.0, 255.0 * (fraction * 2.0))
    } else {
        (255.0 * (1.0 - (fraction - 0.5) * 2.0), 255.0)
    };
    Color32::from_rgb(r as u8, g as u8, 60)
}

/// Draws text with a 1px drop shadow so it stays readable over any backdrop.
fn draw_label(painter: &Painter, pos: Pos2, anchor: Align2, text: &str, color: Color32) {
    let font = FontId::proportional(11.0);
    painter.text(
        pos + vec2(0.8, 0.8),
        anchor,
        text,
        font.clone(),
        Color32::from_black_alpha(210),
    );
    painter.text(pos, anchor, text, font, color);
}
