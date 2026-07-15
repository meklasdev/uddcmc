//! Always-on HUD Framework.
//! Overhauled to implement a dynamic anchoring, positioning, and layout system
//! with a fully interactive Drag & Drop HUD Editor, snap grid, and auto-anchoring.

use crate::graphic::anim::{self, Easing};
use crate::graphic::theme;
use crate::graphic::input::GUI_OPEN;
use egui::{
    Align2, Color32, Context, FontId, Id, LayerId, Order, Pos2, Rect, Rounding, Sense, Stroke, Vec2,
};
use std::sync::atomic::Ordering;

/// Screen-edge padding shared by every HUD element.
const MARGIN: f32 = 12.0;
const SNAP_GRID_SIZE: f32 = 15.0;

/// Core anchoring options supporting all four screen corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HudAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl HudAnchor {
    /// Resolves the absolute top-left position of a widget given the screen boundaries and widget dimensions.
    pub fn resolve_pos(self, screen_size: Vec2, widget_size: Vec2, margin: f32) -> Pos2 {
        match self {
            HudAnchor::TopLeft => Pos2::new(margin, margin),
            HudAnchor::TopRight => Pos2::new(screen_size.x - margin - widget_size.x, margin),
            HudAnchor::BottomLeft => Pos2::new(margin, screen_size.y - margin - widget_size.y),
            HudAnchor::BottomRight => Pos2::new(screen_size.x - margin - widget_size.x, screen_size.y - margin - widget_size.y),
        }
    }

    /// Finds the closest anchor for a given screen position.
    pub fn closest_anchor(pos: Pos2, screen_size: Vec2) -> Self {
        let half_x = screen_size.x / 2.0;
        let half_y = screen_size.y / 2.0;

        if pos.x < half_x {
            if pos.y < half_y {
                HudAnchor::TopLeft
            } else {
                HudAnchor::BottomLeft
            }
        } else {
            if pos.y < half_y {
                HudAnchor::TopRight
            } else {
                HudAnchor::BottomRight
            }
        }
    }
}

/// Helper to snap a coordinate value to the nearest grid step.
pub fn snap_to_grid(val: f32, grid_size: f32) -> f32 {
    (val / grid_size).round() * grid_size
}

/// Draws the full HUD onto the background/interactive layer.
pub fn draw(ctx: &Context) {
    let gui_open = GUI_OPEN.load(Ordering::Relaxed);

    if gui_open {
        // Draw the visual snap grid backdrop when editing HUD elements
        draw_editor_grid(ctx);
    }

    // Watermark Module
    draw_watermark_element(ctx, gui_open);

    // ArrayList Module
    draw_arraylist_element(ctx, gui_open);
}

/// Renders a premium dotted snap grid when the client's GUI editor mode is active.
fn draw_editor_grid(ctx: &Context) {
    let painter = ctx.layer_painter(LayerId::new(Order::Background, Id::new("hud_editor_grid")));
    let screen = ctx.screen_rect();
    let grid_color = Color32::from_rgba_unmultiplied(20, 20, 30, 45);

    // Horizontal grid dots
    let mut x = screen.min.x + SNAP_GRID_SIZE;
    while x < screen.max.x {
        let mut y = screen.min.y + SNAP_GRID_SIZE;
        while y < screen.max.y {
            painter.circle_filled(Pos2::new(x, y), 1.0, grid_color);
            y += SNAP_GRID_SIZE;
        }
        x += SNAP_GRID_SIZE;
    }
}

/// Interactive Watermark Widget: Handles drag-and-drop dragging, anchor auto-snapping, and visual bounding box overlays.
fn draw_watermark_element(ctx: &Context, edit_mode: bool) {
    let font = FontId::proportional(17.0);
    let pad = Vec2::new(11.0, 6.0);

    // Measure bounding box size
    let (dark_w, text_h) = ctx.fonts(|f| {
        let g = f.layout_no_wrap("KRASNOSTAV".to_owned(), font.clone(), theme::TEXT);
        (g.size().x, g.size().y)
    });
    let client_w = ctx.fonts(|f| {
        f.layout_no_wrap(" dev-local".to_owned(), font.clone(), theme::TEAL)
            .size()
            .x
    });

    let widget_size = Vec2::new(dark_w + client_w + pad.x * 2.0, text_h + pad.y * 2.0);
    let screen_size = ctx.screen_rect().size();

    // Fetch or initialize position & anchor
    let anchor_id = Id::new("hud_watermark_anchor");
    let pos_id = Id::new("hud_watermark_pos");

    let mut anchor = ctx.data(|d| d.get_temp::<HudAnchor>(anchor_id)).unwrap_or(HudAnchor::TopLeft);
    let custom_pos = ctx.data(|d| d.get_temp::<Pos2>(pos_id));

    let display_pos = if let Some(p) = custom_pos {
        p
    } else {
        let default_p = anchor.resolve_pos(screen_size, widget_size, MARGIN);
        ctx.data_mut(|d| d.insert_temp(pos_id, default_p));
        default_p
    };

    if edit_mode {
        // Drag Handler Area for Watermark
        let area_id = Id::new("hud_watermark_area");
        egui::Area::new(area_id)
            .current_pos(display_pos)
            .order(Order::Tooltip)
            .show(ctx, |ui| {
                let rect = Rect::from_min_size(display_pos, widget_size);
                let response = ui.allocate_rect(rect, Sense::drag());

                if response.dragged() {
                    let delta = ui.ctx().input(|i| i.pointer.delta());
                    let mut next_pos = display_pos + delta;

                    // Clamping
                    next_pos.x = next_pos.x.clamp(0.0, screen_size.x - widget_size.x);
                    next_pos.y = next_pos.y.clamp(0.0, screen_size.y - widget_size.y);

                    // Dynamic Grid Snapping
                    next_pos.x = snap_to_grid(next_pos.x, SNAP_GRID_SIZE);
                    next_pos.y = snap_to_grid(next_pos.y, SNAP_GRID_SIZE);

                    // Recalculate closest corner anchor
                    let new_anchor = HudAnchor::closest_anchor(next_pos, screen_size);
                    if new_anchor != anchor {
                        anchor = new_anchor;
                        ui.ctx().data_mut(|d| d.insert_temp(anchor_id, anchor));
                    }

                    ui.ctx().data_mut(|d| d.insert_temp(pos_id, next_pos));
                }

                // Render dynamic drag boundary border & anchor indicator
                let painter = ui.painter();
                painter.rect_filled(rect, Rounding::same(theme::RADIUS), Color32::from_rgba_unmultiplied(30, 41, 59, 140));
                painter.rect_stroke(rect, Rounding::same(theme::RADIUS), Stroke::new(1.0, theme::TEAL));

                // Bounding label
                painter.text(
                    rect.center() + Vec2::new(0.0, -2.0),
                    Align2::CENTER_CENTER,
                    "✥ Watermark",
                    FontId::proportional(10.0),
                    theme::TEXT,
                );
            });
    } else {
        // Normal HUD Render mode
        let painter = ctx.layer_painter(LayerId::new(Order::Background, Id::new("hud_watermark_mesh")));
        let rect = Rect::from_min_size(display_pos, widget_size);

        painter.rect_filled(rect, Rounding::same(theme::RADIUS), Color32::from_black_alpha(165));
        painter.rect_stroke(rect, Rounding::same(theme::RADIUS), Stroke::new(1.0, theme::BORDER));

        let edge = Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
        painter.rect_filled(edge, Rounding { nw: theme::RADIUS, sw: theme::RADIUS, ne: 0.0, se: 0.0 }, theme::accent());

        let text_start = egui::pos2(rect.min.x + pad.x, rect.center().y);
        let after = painter.text(text_start, Align2::LEFT_CENTER, "KRASNOSTAV", font.clone(), theme::TEXT);
        painter.text(egui::pos2(after.max.x, text_start.y), Align2::LEFT_CENTER, " dev-local", font, theme::TEAL);
    }
}

/// Interactive ArrayList Widget: Supports drag-and-drop relocation, screen boundary scaling, and cascading list items.
fn draw_arraylist_element(ctx: &Context, edit_mode: bool) {
    const ROW_H: f32 = 19.0;
    const PAD_X: f32 = 8.0;
    let font = FontId::proportional(14.0);

    // List of active module strings
    let snapshot: Vec<(String, bool)> = crate::state::client()
        .modules
        .handles()
        .iter()
        .filter_map(|m| {
            let module = m.lock().ok()?;
            let data = module.get_snapshot();
            Some(data)
        })
        .collect();

    let mut rows: Vec<(String, f32, f32)> = Vec::new();
    for (name, enabled) in snapshot {
        let factor = anim::toggle(ctx, Id::new("arraylist_anim").with(&name), enabled, 0.22, Easing::Out);
        if factor <= 0.001 && !edit_mode {
            continue;
        }
        let disp_factor = if edit_mode { 1.0 } else { factor };
        let width = ctx.fonts(|f| f.layout_no_wrap(name.clone(), font.clone(), theme::TEXT).size().x);
        rows.push((name, disp_factor, width));
    }

    if rows.is_empty() && !edit_mode {
        return;
    }

    // Sort by descending width for staircase alignment
    rows.sort_by(|a, b| b.2.total_cmp(&a.2));

    let max_row_w = rows.first().map(|r| r.2 + PAD_X * 2.0).unwrap_or(120.0);
    let total_h = (rows.len() as f32 * (ROW_H + 2.0)).max(ROW_H);
    let widget_size = Vec2::new(max_row_w, total_h);
    let screen_size = ctx.screen_rect().size();

    // Fetch position & anchor
    let anchor_id = Id::new("hud_arraylist_anchor");
    let pos_id = Id::new("hud_arraylist_pos");

    let mut anchor = ctx.data(|d| d.get_temp::<HudAnchor>(anchor_id)).unwrap_or(HudAnchor::TopRight);
    let custom_pos = ctx.data(|d| d.get_temp::<Pos2>(pos_id));

    let display_pos = if let Some(p) = custom_pos {
        p
    } else {
        let default_p = anchor.resolve_pos(screen_size, widget_size, MARGIN);
        ctx.data_mut(|d| d.insert_temp(pos_id, default_p));
        default_p
    };

    if edit_mode {
        // Drag Area for ArrayList
        let area_id = Id::new("hud_arraylist_area");
        egui::Area::new(area_id)
            .current_pos(display_pos)
            .order(Order::Tooltip)
            .show(ctx, |ui| {
                let rect = Rect::from_min_size(display_pos, widget_size);
                let response = ui.allocate_rect(rect, Sense::drag());

                if response.dragged() {
                    let delta = ui.ctx().input(|i| i.pointer.delta());
                    let mut next_pos = display_pos + delta;

                    next_pos.x = next_pos.x.clamp(0.0, screen_size.x - widget_size.x);
                    next_pos.y = next_pos.y.clamp(0.0, screen_size.y - widget_size.y);

                    next_pos.x = snap_to_grid(next_pos.x, SNAP_GRID_SIZE);
                    next_pos.y = snap_to_grid(next_pos.y, SNAP_GRID_SIZE);

                    let new_anchor = HudAnchor::closest_anchor(next_pos, screen_size);
                    if new_anchor != anchor {
                        anchor = new_anchor;
                        ui.ctx().data_mut(|d| d.insert_temp(anchor_id, anchor));
                    }

                    ui.ctx().data_mut(|d| d.insert_temp(pos_id, next_pos));
                }

                // Drag indicator border
                let painter = ui.painter();
                painter.rect_filled(rect, Rounding::ZERO, Color32::from_rgba_unmultiplied(30, 41, 59, 140));
                painter.rect_stroke(rect, Rounding::ZERO, Stroke::new(1.0, theme::TEAL));

                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "✥ ArrayList",
                    FontId::proportional(11.0),
                    theme::TEXT,
                );
            });
    } else {
        // Real active rendering
        let painter = ctx.layer_painter(LayerId::new(Order::Background, Id::new("hud_arraylist_mesh")));
        let is_top = anchor == HudAnchor::TopLeft || anchor == HudAnchor::TopRight;
        let is_right = anchor == HudAnchor::TopRight || anchor == HudAnchor::BottomRight;

        let mut current_y = display_pos.y;

        for (name, factor, text_w) in &rows {
            let eased = *factor;
            let row_w = text_w + PAD_X * 2.0;

            let current_x = if is_right {
                display_pos.x + widget_size.x - row_w * eased
            } else {
                display_pos.x
            };

            let rect = Rect::from_min_size(egui::pos2(current_x, current_y), Vec2::new(row_w, ROW_H));

            painter.rect_filled(rect, Rounding::ZERO, theme::with_alpha(Color32::from_black_alpha(175), eased));

            let tab_x = if is_right { rect.right() - 2.0 } else { rect.left() };
            let tab = Rect::from_min_size(egui::pos2(tab_x, current_y), Vec2::new(2.0, ROW_H));
            painter.rect_filled(tab, Rounding::ZERO, theme::with_alpha(theme::accent(), eased));

            painter.text(
                egui::pos2(rect.min.x + PAD_X, rect.center().y),
                Align2::LEFT_CENTER,
                name,
                font.clone(),
                theme::with_alpha(theme::TEXT, eased),
            );

            if is_top {
                current_y += ROW_H + 2.0;
            } else {
                current_y -= ROW_H + 2.0;
            }
        }
    }
}

/// Helper module method extension to snapshot the module state quickly without complex locks.
trait ModuleHudExt {
    fn get_snapshot(&self) -> (String, bool);
}

impl ModuleHudExt for Box<dyn crate::module::Module + Send + Sync> {
    fn get_snapshot(&self) -> (String, bool) {
        let d = self.get_module_data();
        (d.name().to_string(), d.enabled)
    }
}
