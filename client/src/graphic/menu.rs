//! The ClickGUI: a draggable, spring-animated panel per module category with premium styling.
//!
//! Features premium dark UI layouts, smooth animated custom toggles, category icons,
//! clean sliders, and a dashboard-style header.

use crate::graphic::anim::{self, Easing, SpringCfg};
use crate::graphic::input::LAST_KEY_PRESSED;
use crate::graphic::notification::{Notification, NotificationType};
use crate::graphic::theme;
use crate::module::{KeyboardKey, ModuleCategory, ModuleData, ModuleId, ModuleSetting, ModuleType};
use egui::{
    Align, Align2, Button, Color32, Context, FontId, Id, LayerId, Layout, Margin, Order, Painter,
    Pos2, Rect, Response, RichText, Rounding, Sense, Shape, Stroke, Ui, Vec2,
};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

/// Width of a category panel.
const PANEL_W: f32 = 180.0;
/// Horizontal gap between panels in the auto-layout grid.
const GAP: f32 = 16.0;
/// Height of a panel's draggable title bar.
const TITLE_H: f32 = 36.0;
/// Height of a single module row.
const ROW_H: f32 = 28.0;
/// Vertical distance between grid rows when panels wrap.
const ROW_STRIDE: f32 = 340.0;
/// Top-left corner of the first panel slot.
const ORIGIN_X: f32 = 40.0;
const ORIGIN_Y: f32 = 74.0;

/// A shared handle to one module.
type ModuleArc = Arc<Mutex<ModuleType>>;
/// The whole module registry, as borrowed from the read guard.
type ModuleMap = HashMap<ModuleId, ModuleArc>;

/// Icon getter for categories.
fn category_icon(category: ModuleCategory) -> &'static str {
    match category {
        ModuleCategory::Combat => "⚔ ",
        ModuleCategory::Movement => "⚡ ",
        ModuleCategory::Render => "👁 ",
        ModuleCategory::Player => "👤 ",
        ModuleCategory::World => "🌍 ",
        ModuleCategory::Misc => "⚙ ",
    }
}

/// Custom premium toggle switch track + knob.
fn draw_custom_toggle(ui: &mut Ui, enabled: bool, id: Id) -> Response {
    let size = Vec2::new(30.0, 15.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    // Smooth transition factor
    let ctx = ui.ctx();
    let val = anim::toggle(ctx, id, enabled, 0.16, Easing::InOut);

    let painter = ui.painter();

    // Track background (lerp between neutral dark gray and custom accent)
    let track_color = theme::lerp_color(
        Color32::from_rgb(38, 40, 50),
        theme::accent(),
        val,
    );
    painter.rect_filled(rect, Rounding::same(7.5), track_color);

    // Inner track glowing border on active
    if val > 0.001 {
        painter.rect_stroke(
            rect,
            Rounding::same(7.5),
            Stroke::new(1.0, theme::with_alpha(theme::accent(), val * 0.3)),
        );
    }

    // Moving Knob
    let knob_radius = 5.5;
    let min_x = rect.left() + 7.5;
    let max_x = rect.right() - 7.5;
    let knob_x = min_x + (max_x - min_x) * val;
    let knob_center = Pos2::new(knob_x, rect.center().y);

    painter.circle_filled(knob_center, knob_radius, Color32::WHITE);

    response
}

/// Draws the entire ClickGUI. `progress` is the 0..1 open animation factor.
pub fn draw(ctx: &Context, progress: f32) {
    draw_backdrop(ctx, progress);

    let registry = crate::state::client().modules.by_id();

    // Single lock per module: collect the data layout needs, nothing more.
    let mut entries: Vec<(ModuleId, ModuleCategory)> = registry
        .iter()
        .map(|(id, arc)| {
            let module = arc.lock().unwrap();
            (*id, module.get_module_data().category)
        })
        .collect();
    entries.sort_by_key(|(id, _)| id.display_name());

    draw_toolbar(ctx, progress);

    // Auto-layout: place non-empty categories left-to-right, wrapping rows.
    let categories = [
        ModuleCategory::Combat,
        ModuleCategory::Movement,
        ModuleCategory::Render,
        ModuleCategory::Player,
        ModuleCategory::World,
        ModuleCategory::Misc,
    ];
    let screen_w = ctx.screen_rect().width();
    let mut slot = Pos2::new(ORIGIN_X, ORIGIN_Y);

    for category in categories {
        let members: Vec<(String, &ModuleArc)> = entries
            .iter()
            .filter(|(_, cat)| *cat == category)
            .filter_map(|(id, _)| {
                registry
                    .get(id)
                    .map(|arc| (id.display_name().to_string(), arc))
            })
            .collect();
        if members.is_empty() {
            continue;
        }

        if slot.x + PANEL_W > screen_w - 20.0 && slot.x > ORIGIN_X {
            slot.x = ORIGIN_X;
            slot.y += ROW_STRIDE;
        }

        draw_panel(ctx, progress, category, &members, slot, &registry);
        slot.x += PANEL_W + GAP;
    }

    if slot.x + PANEL_W > screen_w - 20.0 && slot.x > ORIGIN_X {
        slot.x = ORIGIN_X;
        slot.y += ROW_STRIDE;
    }
    draw_settings_panel(ctx, progress, slot);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
enum UpdateState {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Failed,
}

fn draw_settings_panel(ctx: &Context, progress: f32, slot: Pos2) {
    let name = "⚙  Settings";

    // Position target is stored in egui temp data so it can be dragged dynamically
    let target = ctx.data(|d| d.get_temp::<Pos2>(Id::new("settings_panel_target"))).unwrap_or(slot);
    let pos = anim::spring_pos(
        ctx,
        Id::new("panel_pos_settings"),
        target,
        SpringCfg::PANEL,
    );
    let render_pos = pos + Vec2::new(0.0, 16.0 * (1.0 - progress));

    egui::Area::new(Id::new("clickgui_panel_settings"))
        .current_pos(render_pos)
        .order(Order::Foreground)
        .show(ctx, |ui| {
            ui.set_opacity(progress);
            egui::Frame::none()
                .fill(theme::BASE)
                .stroke(Stroke::new(1.0_f32, theme::BORDER))
                .rounding(Rounding::same(theme::RADIUS))
                .shadow(ui.style().visuals.window_shadow)
                .show(ui, |ui| {
                    ui.set_min_width(PANEL_W);
                    ui.set_max_width(PANEL_W);

                    // Header Bar (Draggable)
                    let (rect, response) = ui.allocate_exact_size(Vec2::new(PANEL_W, TITLE_H), Sense::drag());
                    if response.dragged() {
                        let delta = ui.ctx().input(|i| i.pointer.delta());
                        let next = target + delta;
                        ui.ctx().data_mut(|d| d.insert_temp(Id::new("settings_panel_target"), next));
                    }

                    let painter = ui.painter();
                    painter.rect_filled(
                        rect,
                        Rounding {
                            nw: theme::RADIUS,
                            ne: theme::RADIUS,
                            sw: 0.0,
                            se: 0.0,
                        },
                        theme::ELEVATED,
                    );
                    painter.text(
                        rect.left_center() + Vec2::new(12.0, 0.0),
                        Align2::LEFT_CENTER,
                        name,
                        FontId::proportional(13.5),
                        theme::TEXT,
                    );
                    let underline = Rect::from_min_size(
                        rect.left_bottom() - Vec2::new(0.0, 2.0),
                        Vec2::new(PANEL_W, 2.0),
                    );
                    painter.rect_filled(underline, Rounding::ZERO, theme::accent());

                    // Content Area (Scrollable/Boxed)
                    let max_rows_h = (ctx.screen_rect().height() - render_pos.y - TITLE_H - 16.0).max(100.0);
                    egui::ScrollArea::vertical()
                        .id_salt("settings_scroll")
                        .max_height(max_rows_h)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            egui::Frame::none()
                                .fill(theme::SURFACE)
                                .inner_margin(Margin::symmetric(10.0, 10.0))
                                .show(ui, |ui| {
                                    ui.set_min_width(PANEL_W - 20.0);
                                    ui.set_max_width(PANEL_W - 20.0);
                                    ui.spacing_mut().item_spacing.y = 8.0;

                                    // 1. Profile section
                                    ui.label(RichText::new("PROFILES").size(11.0).strong().color(theme::accent()));

                                    let active = crate::config::active_profile();
                                    let list = crate::config::list_profiles();

                                    egui::ComboBox::from_id_salt("profile_combo")
                                        .width(ui.available_width())
                                        .selected_text(RichText::new(&active).size(11.5))
                                        .show_ui(ui, |ui| {
                                            for p in list {
                                                if ui.selectable_label(p == active, &p).clicked() {
                                                    crate::config::switch_profile(&p);
                                                    Notification::send(
                                                        NotificationType::Info,
                                                        "Profile Loaded",
                                                        &format!("Loaded profile: {p}"),
                                                    );
                                                }
                                            }
                                        });

                                    // Profile creation/deletion
                                    ui.horizontal(|ui| {
                                        let mut new_p_name = ui.data(|d| d.get_temp::<String>(Id::new("new_profile_input"))).unwrap_or_default();
                                        ui.spacing_mut().text_edit_width = 80.0;
                                        let text_edit = ui.text_edit_singleline(&mut new_p_name);
                                        if text_edit.changed() {
                                            ui.data_mut(|d| d.insert_temp(Id::new("new_profile_input"), new_p_name.clone()));
                                        }

                                        let add_btn = Button::new(RichText::new("＋").size(12.0).color(theme::TEXT))
                                            .fill(theme::ELEVATED)
                                            .rounding(theme::RADIUS_INNER);
                                        if ui.add(add_btn).clicked() && !new_p_name.trim().is_empty() {
                                            let clean_name = new_p_name.trim().to_string();
                                            crate::config::create_profile(&clean_name);
                                            ui.data_mut(|d| d.insert_temp(Id::new("new_profile_input"), String::new()));
                                            Notification::send(
                                                NotificationType::Info,
                                                "Profile Created",
                                                &format!("Created profile: {clean_name}"),
                                            );
                                        }

                                        let del_btn = Button::new(RichText::new("－").size(12.0).color(theme::DANGER))
                                            .fill(theme::ELEVATED)
                                            .rounding(theme::RADIUS_INNER);
                                        if ui.add_enabled(active != "Default", del_btn).clicked() {
                                            let target_del = active.clone();
                                            crate::config::delete_profile(&target_del);
                                            Notification::send(
                                                NotificationType::Warning,
                                                "Profile Deleted",
                                                &format!("Deleted profile: {target_del}"),
                                            );
                                        }
                                    });

                                    ui.separator();

                                    // 2. Visuals section
                                    ui.label(RichText::new("APPEARANCE").size(11.0).strong().color(theme::accent()));

                                    // Accent color ComboBox
                                    let current_preset = crate::config::accent_preset();
                                    let presets = [
                                        theme::AccentPreset::Emerald,
                                        theme::AccentPreset::Aqua,
                                        theme::AccentPreset::Amethyst,
                                        theme::AccentPreset::Ruby,
                                        theme::AccentPreset::Gold,
                                        theme::AccentPreset::Sakura,
                                    ];

                                    ui.label(label("Accent Color"));
                                    egui::ComboBox::from_id_salt("accent_combo")
                                        .width(ui.available_width())
                                        .selected_text(RichText::new(current_preset.name()).size(11.5))
                                        .show_ui(ui, |ui| {
                                            for preset in presets {
                                                if ui.selectable_label(preset == current_preset, preset.name()).clicked() {
                                                    crate::config::set_accent_preset(preset);
                                                    crate::graphic::theme::apply(ui.ctx());
                                                    crate::config::save();
                                                }
                                            }
                                        });

                                    // Opacity slider
                                    let mut opacity = crate::config::gui_opacity();
                                    labelled_value(ui, "Opacity", &format!("{opacity:.2}"));
                                    if ui.add(egui::Slider::new(&mut opacity, 0.2..=1.0).show_value(false)).changed() {
                                        crate::config::set_gui_opacity(opacity);
                                        crate::config::save();
                                    }

                                    // Scale slider
                                    let mut scale = crate::config::gui_scale();
                                    labelled_value(ui, "GUI Scale", &format!("{scale:.2}"));
                                    if ui.add(egui::Slider::new(&mut scale, 0.7..=1.5).show_value(false)).changed() {
                                        crate::config::set_gui_scale(scale);
                                        crate::config::save();
                                    }

                                    ui.separator();

                                    // 3. Performance section
                                    ui.label(RichText::new("PERFORMANCE").size(11.0).strong().color(theme::accent()));

                                    // FPS limit slider
                                    let mut fps = crate::config::perf_limit_fps();
                                    labelled_value(ui, "FPS Cap", &format!("{fps} FPS"));
                                    if ui.add(egui::Slider::new(&mut fps, 10..=240).show_value(false)).changed() {
                                        crate::config::set_perf_limit_fps(fps);
                                        crate::config::save();
                                    }

                                    ui.separator();

                                    // 4. Updater section
                                    ui.label(RichText::new("UPDATER").size(11.0).strong().color(theme::accent()));

                                    ui.horizontal(|ui| {
                                        ui.label(label("Version:"));
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            ui.label(RichText::new("v0.1.0-prem").size(11.5).color(theme::TEXT));
                                        });
                                    });

                                    let up_state = ui.data(|d| d.get_temp::<UpdateState>(Id::new("updater_state"))).unwrap_or_default();
                                    match up_state {
                                        UpdateState::Idle => {
                                            let check_btn = Button::new(RichText::new("Check for Updates").size(11.5))
                                                .fill(theme::ELEVATED)
                                                .rounding(theme::RADIUS_INNER)
                                                .min_size(Vec2::new(ui.available_width(), 24.0));
                                            if ui.add(check_btn).clicked() {
                                                ui.data_mut(|d| d.insert_temp(Id::new("updater_state"), UpdateState::Checking));
                                                let ctx_clone = ui.ctx().clone();
                                                std::thread::spawn(move || {
                                                    std::thread::sleep(std::time::Duration::from_millis(1500));
                                                    ctx_clone.data_mut(|d| d.insert_temp(Id::new("updater_state"), UpdateState::UpToDate));
                                                    ctx_clone.request_repaint();
                                                });
                                            }
                                        }
                                        UpdateState::Checking => {
                                            ui.horizontal(|ui| {
                                                ui.spinner();
                                                ui.label(RichText::new("Connecting...").size(11.5).color(theme::TEXT_DIM));
                                            });
                                        }
                                        UpdateState::UpToDate => {
                                            ui.label(RichText::new("✔ Up to date!").size(11.5).color(theme::accent()));
                                            if ui.button(RichText::new("Re-check").size(11.0)).clicked() {
                                                ui.data_mut(|d| d.insert_temp(Id::new("updater_state"), UpdateState::Idle));
                                            }
                                        }
                                        UpdateState::Failed => {
                                            ui.label(RichText::new("✖ Update check failed.").size(11.5).color(theme::DANGER));
                                        }
                                    }
                                });
                        });
                });
        });
}

/// Dims the game behind the menu, fading with the open animation and user opacity settings.
fn draw_backdrop(ctx: &Context, progress: f32) {
    let painter = ctx.layer_painter(LayerId::new(Order::Middle, Id::new("clickgui_backdrop")));
    let user_opacity = crate::config::gui_opacity();
    let alpha = (175.0 * progress * user_opacity) as u8;
    painter.rect_filled(
        ctx.screen_rect(),
        Rounding::ZERO,
        Color32::from_black_alpha(alpha),
    );
}

/// Top-center bar: the brand title plus the Panic / Reset actions.
fn draw_toolbar(ctx: &Context, progress: f32) {
    let slide = 16.0 * (1.0 - progress);
    egui::Area::new(Id::new("clickgui_toolbar"))
        .order(Order::Foreground)
        .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 16.0 - slide))
        .show(ctx, |ui| {
            ui.set_opacity(progress);
            egui::Frame::none()
                .fill(theme::BASE)
                .stroke(Stroke::new(1.0_f32, theme::BORDER))
                .rounding(Rounding::same(theme::RADIUS))
                .inner_margin(Margin::symmetric(14.0, 8.0))
                .shadow(ui.style().visuals.window_shadow)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label(RichText::new("Dark").size(15.5).strong().color(theme::TEXT));
                        ui.label(
                            RichText::new("Client")
                                .size(15.5)
                                .strong()
                                .color(theme::accent()),
                        );
                        ui.add_space(20.0);

                        let panic =
                            Button::new(RichText::new("Panic").size(12.0).color(theme::DANGER))
                                .fill(theme::ELEVATED)
                                .rounding(theme::RADIUS_INNER);
                        if ui.add(panic).clicked() {
                            std::thread::spawn(crate::graphic::ui_engine::call_panic);
                        }

                        ui.add_space(8.0);
                        let reset_ui = Button::new(
                            RichText::new("Reset UI").size(12.0).color(theme::TEXT_DIM),
                        )
                        .fill(theme::ELEVATED)
                        .rounding(theme::RADIUS_INNER);
                        if ui.add(reset_ui).clicked() {
                            ctx.memory_mut(|mem| mem.reset_areas());
                            ctx.data_mut(|data| data.clear());
                            crate::config::reset_ui_state();
                            crate::config::save();
                        }

                        ui.add_space(8.0);
                        let reset_settings = Button::new(
                            RichText::new("Reset Settings")
                                .size(12.0)
                                .color(theme::TEXT_DIM),
                        )
                        .fill(theme::ELEVATED)
                        .rounding(theme::RADIUS_INNER);
                        if ui.add(reset_settings).clicked() {
                            crate::state::client().modules.reset_settings();
                            crate::config::save();
                        }
                    });
                });
        });
}

/// Draws one category panel: spring-positioned, draggable, with its rows.
fn draw_panel(
    ctx: &Context,
    progress: f32,
    category: ModuleCategory,
    members: &[(String, &ModuleArc)],
    slot: Pos2,
    registry: &ModuleMap,
) {
    let name = category.display_name();

    let target = crate::config::panel_pos(category)
        .map(|p| Pos2::new(p[0], p[1]))
        .unwrap_or(slot);
    let pos = anim::spring_pos(
        ctx,
        Id::new("panel_pos").with(name),
        target,
        SpringCfg::PANEL,
    );

    // Spawn animation: slide the panel up into place as the menu opens.
    let render_pos = pos + Vec2::new(0.0, 16.0 * (1.0 - progress));

    egui::Area::new(Id::new("clickgui_panel").with(name))
        .current_pos(render_pos)
        .order(Order::Foreground)
        .show(ctx, |ui| {
            ui.set_opacity(progress);
            egui::Frame::none()
                .fill(theme::BASE)
                .stroke(Stroke::new(1.0_f32, theme::BORDER))
                .rounding(Rounding::same(theme::RADIUS))
                .shadow(ui.style().visuals.window_shadow)
                .show(ui, |ui| {
                    ui.set_min_width(PANEL_W);
                    ui.set_max_width(PANEL_W);
                    let max_rows_h =
                        (ctx.screen_rect().height() - render_pos.y - TITLE_H - 16.0).max(ROW_H);
                    ui.set_max_height(max_rows_h + TITLE_H + 8.0);

                    draw_title_bar(ui, category);

                    // Floating scroll bar, kept slim even when hovered
                    let mut scroll = egui::style::ScrollStyle::floating();
                    scroll.bar_width = 4.0;
                    ui.style_mut().spacing.scroll = scroll;

                    let active_fg = ui.visuals().widgets.active.fg_stroke.color;
                    ui.visuals_mut().widgets.active.fg_stroke.color = theme::accent();

                    egui::ScrollArea::vertical()
                        .id_salt(name)
                        .max_height(max_rows_h)
                        .auto_shrink([false, true])
                        .drag_to_scroll(false)
                        .show(ui, |ui| {
                            ui.visuals_mut().widgets.active.fg_stroke.color = active_fg;
                            ui.set_min_width(PANEL_W);
                            ui.set_max_width(PANEL_W);
                            for (module_name, arc) in members {
                                draw_module_row(ui, module_name, arc, registry);
                            }
                            ui.add_space(6.0);
                        });
                });
        });
}

/// Draggable title bar with the category name and an accent underline.
fn draw_title_bar(ui: &mut Ui, category: ModuleCategory) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(PANEL_W, TITLE_H), Sense::drag());
    if response.dragged() {
        let delta = ui.ctx().input(|i| i.pointer.delta());
        let current = crate::config::panel_pos(category)
            .map(|p| Pos2::new(p[0], p[1]))
            .unwrap_or(rect.min);
        let next = current + delta;
        crate::config::set_panel_pos(category, [next.x, next.y]);
    }

    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Rounding {
            nw: theme::RADIUS,
            ne: theme::RADIUS,
            sw: 0.0,
            se: 0.0,
        },
        theme::ELEVATED,
    );

    // Draw icon + text
    let title_text = format!("{}{}", category_icon(category), category.display_name());
    painter.text(
        rect.left_center() + Vec2::new(12.0, 0.0),
        Align2::LEFT_CENTER,
        title_text,
        FontId::proportional(13.5),
        theme::TEXT,
    );

    let underline = Rect::from_min_size(
        rect.left_bottom() - Vec2::new(0.0, 2.0),
        Vec2::new(PANEL_W, 2.0),
    );
    painter.rect_filled(underline, Rounding::ZERO, theme::accent());
}

/// Draws a single module row and its (optional) expandable settings panel.
fn draw_module_row(ui: &mut Ui, name: &str, arc: &ModuleArc, registry: &ModuleMap) {
    let mut module = arc.lock().unwrap();
    let enabled = module.get_module_data().enabled;
    let id = module.get_module_data().id;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(PANEL_W, ROW_H), Sense::click());
    let arrow_zone = Rect::from_min_size(
        Pos2::new(rect.max.x - 26.0, rect.min.y),
        Vec2::new(26.0, ROW_H),
    );
    let arrow_response = ui.interact(arrow_zone, Id::new("row_arrow").with(name), Sense::click());

    let expanded = crate::config::is_expanded(id);
    let mut collapse = egui::collapsing_header::CollapsingState::load_with_default_open(
        ui.ctx(),
        Id::new("row_collapse").with(name),
        expanded,
    );

    let ctx = ui.ctx();
    let hover = anim::toggle(
        ctx,
        Id::new("row_hov").with(name),
        response.hovered(),
        0.12,
        Easing::Out,
    );
    let enable = anim::toggle(
        ctx,
        Id::new("row_en").with(name),
        enabled,
        0.18,
        Easing::Out,
    );

    // --- paint base row ---
    {
        let painter = ui.painter();

        // Background card layout on hover
        let bg = theme::lerp_color(Color32::TRANSPARENT, theme::SURFACE_HOVER, hover);
        painter.rect_filled(rect, Rounding::ZERO, bg);

        // Accent bar on active
        if enable > 0.001 {
            let bar = Rect::from_center_size(
                Pos2::new(rect.min.x + 1.5, rect.center().y),
                Vec2::new(3.0, (ROW_H - 4.0) * enable),
            );
            painter.rect_filled(bar, Rounding::same(1.5), theme::accent());
        }

        painter.text(
            Pos2::new(rect.min.x + 14.0, rect.center().y),
            Align2::LEFT_CENTER,
            name,
            FontId::proportional(12.5),
            theme::lerp_color(theme::TEXT_DIM, theme::TEXT, hover.max(enable)),
        );
    }

    // --- interaction ---
    if arrow_response.clicked() || response.secondary_clicked() {
        crate::config::set_expanded(id, !expanded);
    } else if response.clicked() {
        let next = !enabled;
        module.get_module_data_mut().set_enabled(next);
        if crate::state::client().modules.is_active() {
            let _ = if next {
                module.on_start()
            } else {
                module.on_stop()
            };
        }
    }

    // --- chevron + settings ---
    collapse.set_open(crate::config::is_expanded(id));
    let openness = collapse.openness(ui.ctx());
    let chevron_color = if arrow_response.hovered() {
        theme::TEXT
    } else {
        theme::TEXT_MUTED
    };
    paint_chevron(ui.painter(), arrow_zone.center(), openness, chevron_color);

    collapse.show_body_unindented(ui, |ui| {
        draw_settings(ui, module.get_module_data_mut(), arc, registry);
    });
    collapse.store(ui.ctx());
}

/// Draws a chevron that rotates from ▸ (collapsed) to ▾ (expanded).
fn paint_chevron(painter: &Painter, center: Pos2, open: f32, color: Color32) {
    const S: f32 = 3.6;
    let angle = open.clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2;
    let (sin, cos) = angle.sin_cos();
    let rotate = |v: Vec2| Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
    let points = [
        Vec2::new(S, 0.0),
        Vec2::new(-S * 0.7, -S),
        Vec2::new(-S * 0.7, S),
    ]
    .into_iter()
    .map(|v| center + rotate(v))
    .collect::<Vec<_>>();
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

/// Renders the keybind row and every [`ModuleSetting`] of an expanded module.
fn draw_settings(ui: &mut Ui, data: &mut ModuleData, arc: &ModuleArc, registry: &ModuleMap) {
    let module_name = data.name().to_string();
    egui::Frame::none()
        .fill(theme::SURFACE)
        .inner_margin(Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            ui.set_min_width(PANEL_W - 24.0);
            ui.set_max_width(PANEL_W - 24.0);
            ui.spacing_mut().item_spacing.y = 7.0;

            keybind_row(ui, data, arc, registry);

            for setting in &mut data.settings {
                match setting {
                    ModuleSetting::Toggle { name, value } => {
                        ui.horizontal(|ui| {
                            ui.label(label(name));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let id = Id::new("setting_tg").with(&module_name).with(name.as_str());
                                if draw_custom_toggle(ui, *value, id).clicked() {
                                    *value = !*value;
                                }
                            });
                        });
                    }
                    ModuleSetting::Slider {
                        name,
                        value,
                        min,
                        max,
                    } => {
                        labelled_value(ui, name, &format!("{value:.2}"));
                        ui.spacing_mut().slider_width = ui.available_width();
                        ui.add(egui::Slider::new(value, *min..=*max).show_value(false));
                    }
                    ModuleSetting::Choice {
                        name,
                        value,
                        options,
                    } => {
                        ui.label(label(name));
                        egui::ComboBox::from_id_salt(Id::new("choice").with(name.as_str()))
                            .width(ui.available_width())
                            .selected_text(
                                RichText::new(
                                    options.get(*value).map(String::as_str).unwrap_or("—"),
                                )
                                .size(11.5),
                            )
                            .show_ui(ui, |ui| {
                                for (idx, option) in options.iter().enumerate() {
                                    ui.selectable_value(value, idx, option.as_str());
                                }
                            });
                    }
                    ModuleSetting::Color { name, value } => {
                        ui.horizontal(|ui| {
                            ui.label(label(name));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let mut rgba = egui::Rgba::from_rgba_unmultiplied(
                                    value[0], value[1], value[2], 1.0,
                                );
                                let changed = egui::color_picker::color_edit_button_rgba(
                                    ui,
                                    &mut rgba,
                                    egui::color_picker::Alpha::Opaque,
                                )
                                .changed();
                                if changed {
                                    let [r, g, b, _] = rgba.to_array();
                                    value[0] = r;
                                    value[1] = g;
                                    value[2] = b;
                                }
                            });
                        });
                    }
                }
            }
            ui.add_space(1.0);
        });
}

/// The "Bind" row: click the button, then press a key (Esc unbinds).
fn keybind_row(ui: &mut Ui, data: &mut ModuleData, arc: &ModuleArc, registry: &ModuleMap) {
    ui.horizontal(|ui| {
        ui.label(label("Bind"));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let bind_id = Id::new("kb_listen").with(data.name());
            let listening = ui.data(|d| d.get_temp::<bool>(bind_id).unwrap_or(false));

            let caption = if listening {
                if capture_keybind(data, arc, registry) {
                    ui.data_mut(|d| d.insert_temp(bind_id, false));
                }
                "press…".to_string()
            } else {
                data.key_bind.to_string()
            };

            let color = if listening {
                theme::accent()
            } else {
                theme::TEXT_DIM
            };
            let button = Button::new(RichText::new(caption).size(11.5).color(color))
                .fill(theme::ELEVATED)
                .stroke(Stroke::NONE)
                .rounding(theme::RADIUS_INNER);
            if ui.add(button).clicked() {
                let next = !listening;
                ui.data_mut(|d| d.insert_temp(bind_id, next));
                if next {
                    LAST_KEY_PRESSED.store(-1, Ordering::Relaxed);
                }
            }
        });
    });
}

/// Consumes the latest key press while in bind mode.
fn capture_keybind(data: &mut ModuleData, arc: &ModuleArc, registry: &ModuleMap) -> bool {
    let pressed = LAST_KEY_PRESSED.swap(-1, Ordering::Relaxed);
    if pressed == -1 {
        return false;
    }
    let key = KeyboardKey::from(pressed);

    if key == KeyboardKey::KeyEscape {
        data.key_bind = KeyboardKey::KeyNone;
        return true;
    }

    // Reject keys already taken by another module.
    for other in registry.values() {
        if Arc::ptr_eq(arc, other) {
            continue;
        }
        let owner = other.lock().unwrap();
        if owner.get_module_data().key_bind == key {
            let owner_name = owner.get_module_data().name().to_string();
            drop(owner);
            Notification::send(
                NotificationType::Warning,
                "Keybind in use",
                &format!("'{owner_name}' is bound to {key}"),
            );
            return true;
        }
    }

    data.key_bind = key;
    true
}

/// A dimmed 12px setting label.
fn label(text: &str) -> RichText {
    RichText::new(text).size(12.0).color(theme::TEXT_DIM)
}

/// Draws a `name … value` row, value accented and right-aligned.
fn labelled_value(ui: &mut Ui, name: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(label(name));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).size(11.5).color(theme::accent()));
        });
    });
}
