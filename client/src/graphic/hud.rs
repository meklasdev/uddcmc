//! Always-on HUD Framework.
//! Overhauled to implement a dynamic anchoring, positioning, and layout system.
//!
//! Watermarks, ArrayLists, and custom widgets are drawn straight onto a background
//! [`egui::Painter`] with absolute screen coordinates resolved dynamically via
//! anchor positions.

use crate::graphic::anim::{self, Easing};
use crate::graphic::theme;
use egui::{
    Align2, Color32, Context, FontId, Id, LayerId, Order, Painter, Pos2, Rect, Rounding, Stroke, Vec2,
};

/// Screen-edge padding shared by every HUD element.
const MARGIN: f32 = 10.0;

/// Core anchoring options supporting all four screen corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

/// Draws the full HUD onto the background layer.
pub fn draw(ctx: &Context) {
    let painter = ctx.layer_painter(LayerId::new(Order::Background, Id::new("hud_layer")));

    // Watermark anchored to the Top-Left
    draw_watermark(ctx, &painter, HudAnchor::TopLeft);

    // ArrayList anchored to the Top-Right (cascades downward)
    draw_arraylist(ctx, &painter, HudAnchor::TopRight);
}

/// Dynamic Watermark: Renders branding with an accent indicator inside a glowing chip.
fn draw_watermark(ctx: &Context, painter: &Painter, anchor: HudAnchor) {
    let font = FontId::proportional(17.0);
    let pad = Vec2::new(11.0, 6.0);

    // Measure both halves so the chip hugs the text exactly.
    let (dark_w, text_h) = ctx.fonts(|f| {
        let g = f.layout_no_wrap("KRASNOSTAV".to_owned(), font.clone(), theme::TEXT);
        (g.size().x, g.size().y)
    });
    let client_w = ctx.fonts(|f| {
        f.layout_no_wrap(" dev-local".to_owned(), font.clone(), theme::TEAL)
            .size()
            .x
    });

    let size = Vec2::new(dark_w + client_w + pad.x * 2.0, text_h + pad.y * 2.0);
    let screen_size = ctx.screen_rect().size();

    // Resolve pos dynamically using the layout anchor
    let pos = anchor.resolve_pos(screen_size, size, MARGIN);
    let rect = Rect::from_min_size(pos, size);

    painter.rect_filled(
        rect,
        Rounding::same(theme::RADIUS),
        Color32::from_black_alpha(165),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(theme::RADIUS),
        Stroke::new(1.0_f32, theme::BORDER),
    );

    // Accent edge on the left side of the chip.
    let edge = Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
    painter.rect_filled(
        edge,
        Rounding {
            nw: theme::RADIUS,
            sw: theme::RADIUS,
            ne: 0.0,
            se: 0.0,
        },
        theme::accent(),
    );

    let anchor_text = egui::pos2(rect.min.x + pad.x, rect.center().y);
    let after = painter.text(
        anchor_text,
        Align2::LEFT_CENTER,
        "KRASNOSTAV",
        font.clone(),
        theme::TEXT,
    );
    painter.text(
        egui::pos2(after.max.x, anchor_text.y),
        Align2::LEFT_CENTER,
        " dev-local",
        font,
        theme::TEAL,
    );
}

/// Dynamic ArrayList: Renders the active modules in order of text width.
/// Supports cascading downward or upward depending on top/bottom anchors.
fn draw_arraylist(ctx: &Context, painter: &Painter, anchor: HudAnchor) {
    const ROW_H: f32 = 19.0;
    const PAD_X: f32 = 8.0;
    let font = FontId::proportional(14.0);

    // One lock per module: snapshot just the name and enabled flag.
    let snapshot: Vec<(String, bool)> = crate::state::client()
        .modules
        .handles()
        .iter()
        .filter_map(|m| {
            let module = m.lock().ok()?;
            let data = module.get_module_data();
            Some((data.name().to_string(), data.enabled))
        })
        .collect();

    // Resolve presence factors.
    let mut rows: Vec<(String, f32, f32)> = Vec::new(); // (name, factor, text width)
    for (name, enabled) in snapshot {
        let factor = anim::toggle(
            ctx,
            Id::new("arraylist").with(&name),
            enabled,
            0.22,
            Easing::Out,
        );
        if factor <= 0.001 {
            continue;
        }
        let width = ctx.fonts(|f| {
            f.layout_no_wrap(name.clone(), font.clone(), theme::TEXT)
                .size()
                .x
        });
        rows.push((name, factor, width));
    }
    if rows.is_empty() {
        return;
    }

    // Longest entry on top — the classic staircase silhouette.
    rows.sort_by(|a, b| b.2.total_cmp(&a.2));

    let screen_size = ctx.screen_rect().size();

    // Top-anchored lists cascade down; Bottom-anchored lists cascade up.
    let is_top = anchor == HudAnchor::TopLeft || anchor == HudAnchor::TopRight;
    let is_right = anchor == HudAnchor::TopRight || anchor == HudAnchor::BottomRight;

    let mut y = if is_top {
        MARGIN
    } else {
        screen_size.y - MARGIN - ROW_H
    };

    for (name, factor, text_w) in &rows {
        let eased = *factor;
        let row_w = text_w + PAD_X * 2.0;

        // Resolve X position dynamically based on left/right anchoring
        let x = if is_right {
            screen_size.x - MARGIN - row_w * eased
        } else {
            MARGIN
        };

        let rect = Rect::from_min_size(egui::pos2(x, y), Vec2::new(row_w, ROW_H));

        painter.rect_filled(
            rect,
            Rounding::ZERO,
            theme::with_alpha(Color32::from_black_alpha(175), eased),
        );

        // Accent tab welded to the screen edge.
        let tab_x = if is_right {
            rect.right_top().x - 2.0
        } else {
            rect.left_top().x
        };
        let tab = Rect::from_min_size(
            egui::pos2(tab_x, y),
            Vec2::new(2.0, ROW_H),
        );
        painter.rect_filled(tab, Rounding::ZERO, theme::with_alpha(theme::accent(), eased));

        painter.text(
            egui::pos2(rect.min.x + PAD_X, rect.center().y),
            Align2::LEFT_CENTER,
            name,
            font.clone(),
            theme::with_alpha(theme::TEXT, eased),
        );

        // Cascade direction
        if is_top {
            y += ROW_H + 2.0;
        } else {
            y -= ROW_H + 2.0;
        }
    }
}
