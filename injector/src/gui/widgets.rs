//! Reusable egui widgets for the redesigned premium injector GUI.

use egui::{
    pos2, Align2, Button, Color32, FontId, Frame, Margin, Pos2, Rect, Response, RichText,
    Sense, Spinner, Stroke, Ui, Vec2, Rounding,
};

use super::theme::Palette;
use crate::app::{InjectionStatus, InjectorApp};
use crate::platform::ProcessInfo;

/// Computes the exact progress fraction for each real-time operation step.
fn progress_fraction(status: &InjectionStatus) -> f32 {
    match status {
        InjectionStatus::Idle => 0.0,
        InjectionStatus::Scanning => 0.15,
        InjectionStatus::Initializing => 0.35,
        InjectionStatus::DetectingJvm => 0.55,
        InjectionStatus::LoadingAgent => 0.75,
        InjectionStatus::ConnectingClient => 0.90,
        InjectionStatus::Finished(_) => 1.0,
        InjectionStatus::Failed(_) => 1.0,
    }
}

/// Beautiful vector text brand block rendered in the top header panel.
pub fn header(ui: &mut Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(4.0);
        // Custom PREMIUM Branding Logo
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(ui.available_width() / 2.0 - 64.0);
                ui.label(
                    RichText::new("KRASNO")
                        .size(24.0)
                        .strong()
                        .color(Palette::TEXT)
                        .extra_letter_spacing(1.5),
                );
                ui.label(
                    RichText::new("STAV")
                        .size(24.0)
                        .strong()
                        .color(Palette::ACCENT)
                        .extra_letter_spacing(1.5),
                );
            });
        });
        ui.add_space(2.0);
        ui.label(
            RichText::new("https://www.krasnostav.pro/")
                .size(10.0)
                .strong()
                .color(Palette::TEXT_DIM)
                .extra_letter_spacing(2.0),
        );
        ui.add_space(4.0);
    });
}

/// A small secondary button (used for "Scan").
pub fn tool_button(ui: &mut Ui, label: &str, enabled: bool) -> Response {
    let button = Button::new(RichText::new(label).color(Palette::TEXT).strong())
        .fill(Palette::CARD)
        .rounding(8.0)
        .min_size(Vec2::new(0.0, 34.0));
    ui.add_enabled(enabled, button)
}

/// A selectable process card. Returns the click response.
pub fn process_card(ui: &mut Ui, proc: &ProcessInfo, selected: bool) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 62.0), Sense::click());

    let bg = if selected {
        Palette::ACCENT_SOFT
    } else if response.hovered() {
        Palette::CARD_HOVER
    } else {
        Palette::CARD
    };

    let painter = ui.painter();
    painter.rect_filled(rect, 10.0, bg);

    // Glowing border on hover/selected
    let stroke_color = if selected {
        Palette::ACCENT
    } else if response.hovered() {
        Palette::BORDER
    } else {
        Color32::TRANSPARENT
    };
    painter.rect_stroke(rect, 10.0, Stroke::new(1.2, stroke_color));

    let dot_color = if selected {
        Palette::ACCENT
    } else {
        Palette::TEXT_DIM
    };
    painter.circle_filled(rect.left_center() + Vec2::new(18.0, 0.0), 4.5, dot_color);

    let text_x = rect.left() + 34.0;
    painter.text(
        pos2(text_x, rect.center().y - 10.0),
        Align2::LEFT_CENTER,
        format!("PID {}", proc.pid),
        FontId::proportional(14.5),
        Palette::TEXT,
    );
    painter.text(
        pos2(text_x, rect.center().y + 10.0),
        Align2::LEFT_CENTER,
        &proc.info,
        FontId::proportional(11.5),
        Palette::TEXT_DIM,
    );

    response
}

/// Placeholder shown when no Minecraft processes were found.
pub fn empty_state(ui: &mut Ui) {
    ui.add_space(54.0);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new("No active Minecraft processes detected")
                .size(14.0)
                .strong()
                .color(Palette::TEXT),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new("Please start the game, then press Scan.")
                .size(11.5)
                .color(Palette::TEXT_DIM),
        );
    });
}

/// The status banner shown in the footer with progress bar.
pub fn status_banner(ui: &mut Ui, status: &InjectionStatus) {
    let (color, icon) = match status {
        InjectionStatus::Idle => (Palette::TEXT_DIM, "●"),
        InjectionStatus::Scanning => (Palette::ACCENT, "◌"),
        InjectionStatus::Initializing
        | InjectionStatus::DetectingJvm
        | InjectionStatus::LoadingAgent
        | InjectionStatus::ConnectingClient => (Palette::WARN, "◌"),
        InjectionStatus::Finished(_) => (Palette::OK, "✔"),
        InjectionStatus::Failed(_) => (Palette::ERR, "✖"),
    };

    let busy = matches!(
        status,
        InjectionStatus::Scanning
            | InjectionStatus::Initializing
            | InjectionStatus::DetectingJvm
            | InjectionStatus::LoadingAgent
            | InjectionStatus::ConnectingClient
    );

    Frame::none()
        .fill(Palette::BG)
        .rounding(Rounding::same(8.0))
        .stroke(Stroke::new(1.0, Palette::BORDER))
        .inner_margin(Margin::symmetric(14.0, 12.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if busy {
                        ui.add(Spinner::new().size(14.0).color(color));
                    } else {
                        ui.label(RichText::new(icon).size(14.0).strong().color(color));
                    }
                    ui.label(RichText::new(status.message()).strong().size(12.0).color(Palette::TEXT));
                });

                ui.add_space(8.0);

                // Draw dynamic smooth progress bar!
                draw_progress_bar(ui, status);
            });
        });
}

/// Draws an elegant animated progress bar.
pub fn draw_progress_bar(ui: &mut Ui, status: &InjectionStatus) {
    let fraction = progress_fraction(status);

    let size = Vec2::new(ui.available_width(), 5.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());

    let painter = ui.painter();

    // Track background
    painter.rect_filled(rect, Rounding::same(2.5), Color32::from_rgb(18, 19, 24));

    if fraction > 0.001 {
        let fill_w = rect.width() * fraction;
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height()));

        let fill_color = match status {
            InjectionStatus::Failed(_) => Palette::ERR,
            InjectionStatus::Finished(_) => Palette::OK,
            _ => Palette::ACCENT,
        };
        painter.rect_filled(fill_rect, Rounding::same(2.5), fill_color);

        // Progress tip indicator glow
        if fraction < 0.999 {
            let glow_center = Pos2::new(rect.min.x + fill_w, rect.center().y);
            painter.circle_filled(glow_center, 3.5, fill_color);
        }
    }
}

/// The full-width primary "Inject" button.
pub fn inject_button(ui: &mut Ui, app: &InjectorApp) -> Response {
    let enabled = app.selected_pid().is_some() && !app.is_busy();
    let label = if app.is_busy() {
        "Processing Injection..."
    } else {
        "Inject"
    };
    let button = Button::new(
        RichText::new(label)
            .size(14.5)
            .strong()
            .color(Color32::WHITE),
    )
    .fill(Palette::ACCENT)
    .rounding(8.0)
    .min_size(Vec2::new(ui.available_width(), 44.0));
    ui.add_enabled(enabled, button)
}
