use egui::{Align2, Color32, Pos2, Rect, Rounding, Stroke, Vec2};
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NotificationType {
    Info,
    Warning,
    Alert,
}

#[derive(Clone)]
pub struct Notification {
    pub notif_type: NotificationType,
    pub title: String,
    pub message: String,
    pub spawn_time: Instant,
    pub duration: f32,
}

impl Notification {
    pub fn send(notif_type: NotificationType, title: &str, message: &str) {
        Self::send_with_time(notif_type, title, message, 3.0);
    }

    pub fn send_with_time(notif_type: NotificationType, title: &str, message: &str, duration: f32) {
        let mut queue = NOTIFICATIONS.lock().unwrap();
        queue.push(Notification {
            notif_type,
            title: title.to_string(),
            message: message.to_string(),
            spawn_time: Instant::now(),
            duration,
        });
    }
}

lazy_static! {
    pub static ref NOTIFICATIONS: Mutex<Vec<Notification>> = Mutex::new(Vec::new());
}

pub fn draw_notifications(ctx: &egui::Context) {
    let mut queue = NOTIFICATIONS.lock().unwrap();
    let now = Instant::now();

    // Remove expired notifications
    queue.retain(|n| now.duration_since(n.spawn_time).as_secs_f32() < n.duration);

    let screen_width = ctx.screen_rect().width();
    let pad = 15.0;
    let width = 250.0;
    let height = 60.0;
    let mut current_y = pad; // Draw notifications from top right downwards

    for n in queue.iter() {
        let elapsed = now.duration_since(n.spawn_time).as_secs_f32();

        // Calculate animation offsets (slide in from right, then slide out to right)
        let in_anim_dur = 0.3;
        let out_anim_dur = 0.3;

        let slide_offset = if elapsed < in_anim_dur {
            // Slide in from right (start off-screen right, move left to 0 offset)
            let progress = elapsed / in_anim_dur;
            // cubic ease out
            let ease = 1.0 - (1.0 - progress).powi(3);
            300.0 * (1.0 - ease)
        } else if elapsed > n.duration - out_anim_dur {
            // Slide out to right
            let progress = (elapsed - (n.duration - out_anim_dur)) / out_anim_dur;
            // cubic ease in
            let ease = progress.powi(3);
            300.0 * ease
        } else {
            0.0 // Fully visible
        };

        // Define actual rect anchored to top-right
        // x-coord = screen_width - pad - width + slide_offset
        let rect_x = screen_width - pad - width + slide_offset;

        let rect = Rect::from_min_size(Pos2::new(rect_x, current_y), Vec2::new(width, height));

        // Draw background
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("notifications"),
        ))
        .rect(
            rect,
            Rounding::same(4.0),
            Color32::from_black_alpha(220),
            Stroke::new(1.0, Color32::from_rgb(45, 45, 45)),
        );

        // Color coding by type
        let (icon_color, icon_char) = match n.notif_type {
            NotificationType::Info => (Color32::from_rgb(26, 171, 138), "i"), // Teal
            NotificationType::Warning => (Color32::from_rgb(255, 165, 0), "!"), // Orange
            NotificationType::Alert => (Color32::from_rgb(220, 50, 50), "X"), // Red
        };

        // Draw left accent bar
        let accent_rect = Rect::from_min_size(rect.min, Vec2::new(4.0, height));
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("notifications"),
        ))
        .rect_filled(
            accent_rect,
            Rounding {
                nw: 4.0,
                sw: 4.0,
                ne: 0.0,
                se: 0.0,
            },
            icon_color,
        );

        // Draw Icon bg circle
        let center_icon = rect.min + Vec2::new(25.0, height / 2.0);
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("notifications"),
        ))
        .circle_filled(center_icon, 12.0, Color32::from_white_alpha(20));

        // Draw Icon text
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("notifications"),
        ))
        .text(
            center_icon,
            Align2::CENTER_CENTER,
            icon_char,
            egui::FontId::proportional(16.0),
            icon_color,
        );

        // Draw Texts
        let text_start = rect.min + Vec2::new(50.0, 10.0);
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("notifications"),
        ))
        .text(
            text_start,
            Align2::LEFT_TOP,
            &n.title,
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );

        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("notifications"),
        ))
        .text(
            text_start + Vec2::new(0.0, 18.0),
            Align2::LEFT_TOP,
            &n.message,
            egui::FontId::proportional(12.0),
            Color32::from_gray(180),
        );

        current_y += height + pad;
    }
}
