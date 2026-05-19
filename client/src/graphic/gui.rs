//! Top-level overlay renderer.
//!
//! This module owns only the per-frame *dispatch order* — HUD underneath,
//! ClickGUI in the middle, notifications always on top. Visual styling lives
//! in [`theme`](crate::graphic::theme), animation in
//! [`anim`](crate::graphic::anim); each widget owns its own module.

use crate::graphic::anim::{self, Easing};
use crate::graphic::input::GUI_OPEN;
use crate::graphic::{hud, menu, notification};
use egui::{Context, Id};
use std::sync::atomic::Ordering;

/// Renders the whole overlay for a single frame.
pub fn render_all(ctx: &Context) {
    hud::draw(ctx);

    // A single tween turns the open/close toggle into the 0..1 factor the
    // menu uses to fade and slide itself in.
    let open = GUI_OPEN.load(Ordering::Relaxed);
    let progress = anim::toggle(ctx, Id::new("clickgui_open"), open, 0.18, Easing::InOut);
    if progress > 0.0 {
        menu::draw(ctx, progress);
    }

    notification::draw(ctx);

    // Reclaim animation state for anything that stopped being drawn.
    anim::gc(ctx);
}
