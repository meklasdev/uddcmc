//! The ClickGUI: A single, pixel-perfect, cohesive dashboard-style utility panel.
//! Rebranded for "KRASNOSTAV Minecraft" with a Single-Window layout.
//!
//! Features:
//! - Sleek left sidebar navigation with Wolf Logo, tabs (Modules, Configs, Scripts, Community), and Profile block.
//! - MainWindow featuring a dedicated top Title Bar with "KRASNOSTAV dev-local" and mock window control buttons.
//! - Modules organized into custom Category Cards (Combat, Movement, etc.) with star ratings and item lists.
//! - Detailed Settings Modal (NoFall Example) with glassmorphism overlay sliding centrally from the right.
//! - Integrated Configs and Community Review boards nested directly into the main view grid.
//! - Custom UI styling matching the deep anthracite/grey base (#0F0F11, #16161B), vibrant blue active accents, and glowing teal accents.

use crate::graphic::anim::{self, Easing, SpringCfg};
use crate::graphic::input::LAST_KEY_PRESSED;
use crate::graphic::notification::{Notification, NotificationType};
use crate::graphic::theme;
use crate::module::{KeyboardKey, ModuleCategory, ModuleData, ModuleId, ModuleSetting, ModuleType};
use egui::{
    Align, Align2, Button, Color32, Context, FontId, Id, LayerId, Layout, Margin, Order,
    Pos2, Rect, Response, RichText, Rounding, Sense, Stroke, Ui, Vec2,
};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

pub static PANIC_REQUESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Total layout size
const DASHBOARD_W: f32 = 840.0;
const DASHBOARD_H: f32 = 560.0;
const TITLEBAR_H: f32 = 40.0;
const SIDEBAR_W: f32 = 200.0;

type ModuleArc = Arc<Mutex<ModuleType>>;
type ModuleMap = HashMap<ModuleId, ModuleArc>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum GuiTab {
    Modules,
    Configs,
    Scripts,
    Community,
}

/// Dynamic review details structure.
struct ConfigReview {
    user: &'static str,
    rating: usize,
    text: &'static str,
    date: &'static str,
    avatar_color: Color32,
}

/// Cloud Config Entry.
struct CloudConfig {
    name: &'static str,
    creator: &'static str,
    rating: f32,
    reviews_count: usize,
    tags: &'static [&'static str],
    is_saved: bool,
    image_accent: Color32,
}

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

/// Custom premium toggle switch track + knob with subtle glow.
fn draw_custom_toggle(ui: &mut Ui, enabled: bool, id: Id) -> Response {
    let size = Vec2::new(32.0, 16.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    let ctx = ui.ctx();
    let val = anim::toggle(ctx, id, enabled, 0.16, Easing::InOut);

    let painter = ui.painter();

    // Background track color (deep charcoal -> glowing electric/neon blue)
    let track_color = theme::lerp_color(
        Color32::from_rgb(32, 33, 40),
        theme::accent(),
        val,
    );
    painter.rect_filled(rect, Rounding::same(8.0), track_color);

    // Subtle neon border glow when active
    if val > 0.001 {
        painter.rect_stroke(
            rect,
            Rounding::same(8.0),
            Stroke::new(1.0, theme::with_alpha(theme::accent(), val * 0.4)),
        );
    }

    // Moving Knob
    let knob_radius = 6.0;
    let min_x = rect.left() + 8.0;
    let max_x = rect.right() - 8.0;
    let knob_x = min_x + (max_x - min_x) * val;
    let knob_center = Pos2::new(knob_x, rect.center().y);

    painter.circle_filled(knob_center, knob_radius, Color32::WHITE);

    response
}

/// Backdrop dimming
fn draw_backdrop(ctx: &Context, progress: f32) {
    let painter = ctx.layer_painter(LayerId::new(Order::Middle, Id::new("clickgui_backdrop")));
    let user_opacity = crate::config::gui_opacity();
    let alpha = (180.0 * progress * user_opacity) as u8;
    painter.rect_filled(
        ctx.screen_rect(),
        Rounding::ZERO,
        Color32::from_black_alpha(alpha),
    );
}

/// Master function to render the integrated dashboard.
pub fn draw(ctx: &Context, progress: f32) {
    draw_backdrop(ctx, progress);

    let active_tab = ctx.data(|d| d.get_temp::<GuiTab>(Id::new("clickgui_active_tab"))).unwrap_or(GuiTab::Modules);

    // Draggable position target for the unified window, centered initially.
    let screen_rect = ctx.screen_rect();
    let initial_pos = Pos2::new(
        (screen_rect.width() - DASHBOARD_W) / 2.0,
        (screen_rect.height() - DASHBOARD_H) / 2.0,
    );
    let target_pos = ctx.data(|d| d.get_temp::<Pos2>(Id::new("clickgui_dash_pos"))).unwrap_or(initial_pos);
    let pos = anim::spring_pos(ctx, Id::new("clickgui_dash_anim_pos"), target_pos, SpringCfg::PANEL);

    // Smooth vertical pop in
    let render_pos = pos + Vec2::new(0.0, 20.0 * (1.0 - progress));

    egui::Area::new(Id::new("clickgui_dashboard_root"))
        .current_pos(render_pos)
        .order(Order::Foreground)
        .show(ctx, |ui| {
            ui.set_opacity(progress);

            egui::Frame::none()
                .fill(theme::BASE)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .rounding(Rounding::same(theme::RADIUS))
                .shadow(ui.style().visuals.window_shadow)
                .show(ui, |ui| {
                    ui.set_min_size(Vec2::new(DASHBOARD_W, DASHBOARD_H));
                    ui.set_max_size(Vec2::new(DASHBOARD_W, DASHBOARD_H));

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;

                        // 1. Built-in Top Title Bar (MainWindow Header)
                        draw_title_bar(ui, target_pos);

                        // Horizontal separator below Title Bar
                        let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(DASHBOARD_W, 1.0), Sense::hover());
                        ui.painter().rect_filled(sep_rect, Rounding::ZERO, theme::BORDER);

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::ZERO;

                            // 2. Fixed Left Sidebar
                            draw_sidebar(ui, active_tab, target_pos);

                            // Vertical separator
                            let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(1.0, DASHBOARD_H - TITLEBAR_H), Sense::hover());
                            ui.painter().rect_filled(sep_rect, Rounding::ZERO, theme::BORDER);

                            // 3. Main Content panel layout
                            draw_main_content(ui, active_tab);
                        });
                    });
                });
        });

    // Render detailed settings modal overlay if active
    draw_settings_modal_if_active(ctx, progress);
}

/// The MainWindow Title Bar (górny pasek tytułowy)
fn draw_title_bar(ui: &mut Ui, target_pos: Pos2) {
    let titlebar_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(DASHBOARD_W, TITLEBAR_H));

    // Draggable area
    let drag_resp = ui.allocate_rect(titlebar_rect, Sense::drag());
    if drag_resp.dragged() {
        let delta = ui.ctx().input(|i| i.pointer.delta());
        let next = target_pos + delta;

        let screen_rect = ui.ctx().screen_rect();
        let min_x = screen_rect.min.x - DASHBOARD_W + 40.0;
        let max_x = screen_rect.max.x - 40.0;
        let min_y = screen_rect.min.y;
        let max_y = screen_rect.max.y - TITLEBAR_H;

        let clamped_next = Pos2::new(
            next.x.clamp(min_x, max_x),
            next.y.clamp(min_y, max_y),
        );

        ui.ctx().data_mut(|d| d.insert_temp(Id::new("clickgui_dash_pos"), clamped_next));
    }

    // Titlebar background
    ui.painter().rect_filled(
        titlebar_rect,
        Rounding {
            nw: theme::RADIUS,
            ne: theme::RADIUS,
            sw: 0.0,
            se: 0.0,
        },
        theme::SURFACE,
    );

    // Left text: Branding + dev-local indicator
    ui.painter().text(
        Pos2::new(titlebar_rect.left() + 20.0, titlebar_rect.center().y),
        Align2::LEFT_CENTER,
        "🐺 KRASNOSTAV",
        FontId::proportional(14.0),
        theme::TEXT,
    );
    ui.painter().text(
        Pos2::new(titlebar_rect.left() + 118.0, titlebar_rect.center().y + 1.0),
        Align2::LEFT_CENTER,
        "dev-local",
        FontId::proportional(10.0),
        theme::TEAL,
    );

    // Right mock window control buttons (─, ▢, ✕)
    let btn_margin_right = 16.0;
    let btn_spacing = 16.0;
    let control_buttons = [("✕", theme::PANIC), ("▢", theme::TEXT_DIM), ("─", theme::TEXT_DIM)];

    let mut btn_x = titlebar_rect.right() - btn_margin_right;
    for (lbl, color) in control_buttons {
        let text_pos = Pos2::new(btn_x - 8.0, titlebar_rect.center().y);
        ui.painter().text(
            text_pos,
            Align2::CENTER_CENTER,
            lbl,
            FontId::proportional(11.0),
            color,
        );
        btn_x -= btn_spacing;
    }

    ui.advance_cursor_after_rect(titlebar_rect);
}

/// The Fixed Left Sidebar navigation
fn draw_sidebar(ui: &mut Ui, active_tab: GuiTab, _target_pos: Pos2) {
    let sidebar_h = DASHBOARD_H - TITLEBAR_H;
    let sidebar_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(SIDEBAR_W, sidebar_h));

    // Sidebar background
    ui.painter().rect_filled(
        sidebar_rect,
        Rounding {
            nw: 0.0,
            sw: theme::RADIUS,
            ne: 0.0,
            se: 0.0,
        },
        theme::SURFACE,
    );

    // Top logo & Text block
    let logo_y = sidebar_rect.top() + 30.0;
    ui.painter().text(
        Pos2::new(sidebar_rect.left() + 20.0, logo_y),
        Align2::LEFT_CENTER,
        "🐺 KRASNOSTAV",
        FontId::proportional(18.0),
        theme::TEXT,
    );
    ui.painter().text(
        Pos2::new(sidebar_rect.left() + 20.0, logo_y + 18.0),
        Align2::LEFT_CENTER,
        "dev-local environment",
        FontId::proportional(9.5),
        theme::TEAL,
    );

    // Render navigation tabs: Modules, Configs, Scripts, Community
    let tabs = [
        (GuiTab::Modules, "⚔  Modules"),
        (GuiTab::Configs, "⚙  Configs"),
        (GuiTab::Scripts, "📜  Scripts"),
        (GuiTab::Community, "👥  Community"),
    ];

    let mut item_y = sidebar_rect.top() + 85.0;
    for (tab_type, label) in tabs {
        let is_selected = active_tab == tab_type;
        let item_rect = Rect::from_min_size(
            Pos2::new(sidebar_rect.left() + 12.0, item_y),
            Vec2::new(SIDEBAR_W - 24.0, 36.0),
        );
        let tab_id = Id::new("tab_btn").with(label);
        let tab_response = ui.interact(item_rect, tab_id, Sense::click());

        let hover_factor = anim::toggle(
            ui.ctx(),
            Id::new("tab_hover").with(label),
            tab_response.hovered(),
            0.12,
            Easing::Out,
        );

        if tab_response.clicked() {
            ui.ctx().data_mut(|d| d.insert_temp(Id::new("clickgui_active_tab"), tab_type));
        }

        // Draw tab background
        if is_selected {
            // Selected background (vibrant blue or neon dim)
            ui.painter().rect_filled(
                item_rect,
                Rounding::same(theme::RADIUS_INNER),
                theme::accent_dim(),
            );
            // Left active glowing teal bar
            let indicator = Rect::from_min_size(
                Pos2::new(item_rect.left() + 4.0, item_rect.top() + 8.0),
                Vec2::new(3.0, 20.0),
            );
            ui.painter().rect_filled(indicator, Rounding::same(1.5), theme::TEAL);
        } else if hover_factor > 0.001 {
            // Hover background
            ui.painter().rect_filled(
                item_rect,
                Rounding::same(theme::RADIUS_INNER),
                theme::with_alpha(theme::SURFACE_HOVER, hover_factor),
            );
        }

        // Tab Text
        let text_color = if is_selected {
            theme::TEXT
        } else {
            theme::lerp_color(theme::TEXT_MUTED, theme::TEXT_DIM, hover_factor)
        };
        ui.painter().text(
            item_rect.left_center() + Vec2::new(16.0, 0.0),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(12.5),
            text_color,
        );

        item_y += 42.0;
    }

    // Integrated Footer (Zintegrowana Stopka)
    // Horizontal row of mock profile "👤", help "❓", settings "⚙" icons
    let footer_h = 75.0;
    let footer_rect = Rect::from_min_size(
        Pos2::new(sidebar_rect.left() + 12.0, sidebar_rect.bottom() - footer_h - 10.0),
        Vec2::new(SIDEBAR_W - 24.0, footer_h),
    );

    // Profile card area inside footer
    let icon_row_y = footer_rect.top() + 12.0;
    let icons = [("👤", "Profile info"), ("❓", "Help & Docs"), ("⚙", "Client Setup")];
    let mut icon_x = footer_rect.left() + 16.0;
    for (icon_sym, desc) in icons {
        let icon_box = Rect::from_min_size(Pos2::new(icon_x, icon_row_y - 8.0), Vec2::new(20.0, 20.0));
        let resp = ui.interact(icon_box, Id::new("footer_icon").with(icon_sym), Sense::click());
        if resp.clicked() {
            Notification::send(NotificationType::Info, desc, "Consulting integrated system diagnostics...");
        }
        let col = if resp.hovered() { theme::TEAL } else { theme::TEXT_DIM };
        ui.painter().text(icon_box.center(), Align2::CENTER_CENTER, icon_sym, FontId::proportional(12.0), col);
        icon_x += 32.0;
    }

    // Prominent Wide Red PANIC button
    let panic_rect = Rect::from_min_size(
        Pos2::new(footer_rect.left(), footer_rect.bottom() - 28.0),
        Vec2::new(footer_rect.width(), 26.0),
    );
    let panic_resp = ui.interact(panic_rect, Id::new("sidebar_panic_btn"), Sense::click());
    let panic_hover = anim::toggle(ui.ctx(), Id::new("panic_h"), panic_resp.hovered(), 0.12, Easing::Out);

    let panic_bg = theme::lerp_color(theme::PANIC, Color32::from_rgb(220, 38, 38), panic_hover);
    ui.painter().rect_filled(panic_rect, Rounding::same(6.0), panic_bg);
    ui.painter().text(
        panic_rect.center(),
        Align2::CENTER_CENTER,
        "⚡ PANIC",
        FontId::proportional(11.0),
        Color32::WHITE,
    );
    if panic_resp.clicked() {
        PANIC_REQUESTED.store(true, Ordering::Relaxed);
    }

    ui.advance_cursor_after_rect(sidebar_rect);
}

/// Main content panel router
fn draw_main_content(ui: &mut Ui, active_tab: GuiTab) {
    let main_h = DASHBOARD_H - TITLEBAR_H;
    let main_rect = Rect::from_min_size(
        ui.cursor().min,
        Vec2::new(DASHBOARD_W - SIDEBAR_W - 1.0, main_h),
    );

    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(main_rect));

    // Clean padding wrapper inside the content panel
    egui::Frame::none()
        .inner_margin(Margin::same(24.0))
        .show(&mut child_ui, |ui| {
            ui.set_min_size(Vec2::new(DASHBOARD_W - SIDEBAR_W - 49.0, main_h - 48.0));
            ui.set_max_size(Vec2::new(DASHBOARD_W - SIDEBAR_W - 49.0, main_h - 48.0));

            match active_tab {
                GuiTab::Modules => draw_modules_grid_tab(ui),
                GuiTab::Configs => draw_configs_tab(ui),
                GuiTab::Scripts => draw_scripts_tab(ui),
                GuiTab::Community => draw_community_tab(ui),
            }
        });
}

// ============================================================================
// 1. MODULES GRID TAB (Main Card Grid / Orthographic technical view)
// ============================================================================

fn draw_modules_grid_tab(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("SYSTEM MODULE CATALOG").font(FontId::proportional(18.0)).strong().color(theme::TEXT));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new("⚡ orthographic schema view").font(FontId::proportional(11.0)).color(theme::TEAL));
        });
    });
    ui.add_space(4.0);
    ui.label(RichText::new("Select and configure internal subsystems of KRASNOSTAV client overlay.").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
    ui.add_space(16.0);

    let registry = crate::state::client().modules.by_id();
    let categories = [
        ModuleCategory::Combat,
        ModuleCategory::Movement,
        ModuleCategory::Render,
        ModuleCategory::Player,
        ModuleCategory::World,
        ModuleCategory::Misc,
    ];

    egui::ScrollArea::vertical()
        .id_salt("modules_grid_scroll")
        .max_height(400.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(16.0, 16.0);
                let card_w = (ui.available_width() - 16.0) / 2.0;

                for cat in categories {
                    // Extract members for category
                    let mut members: Vec<(ModuleId, String, bool)> = registry
                        .iter()
                        .filter_map(|(id, arc)| {
                            let m = arc.lock().unwrap();
                            if m.get_module_data().category == cat {
                                Some((*id, id.display_name().to_string(), m.get_module_data().enabled))
                            } else {
                                None
                            }
                        })
                        .collect();
                    members.sort_by_key(|(_, name, _)| name.clone());

                    if members.is_empty() {
                        continue;
                    }

                    // Render Component Card (Category Box)
                    egui::Frame::none()
                        .fill(theme::SURFACE)
                        .stroke(Stroke::new(1.0, theme::BORDER))
                        .rounding(Rounding::same(theme::RADIUS_INNER))
                        .inner_margin(Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.set_min_width(card_w);
                            ui.set_max_width(card_w);

                            // Category Card Title and Rating Indicator
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{} {}", category_icon(cat), cat.display_name().to_uppercase())).font(FontId::proportional(12.0)).strong().color(theme::TEXT));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    // 5 gold stars rating indicator
                                    ui.label(RichText::new("★★★★★").font(FontId::proportional(11.0)).color(Color32::from_rgb(250, 204, 21)));
                                });
                            });
                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(8.0);

                            // ItemList of sub-modules
                            ui.vertical(|ui| {
                                for (id, name, enabled) in members {
                                    let item_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(card_w - 24.0, 28.0));
                                    let item_resp = ui.interact(item_rect, Id::new("mod_grid_item").with(id), Sense::click());

                                    let hover_val = anim::toggle(ui.ctx(), Id::new("mod_grid_h").with(id), item_resp.hovered(), 0.12, Easing::Out);
                                    let fill_c = if item_resp.hovered() {
                                        theme::with_alpha(theme::SURFACE_HOVER, hover_val)
                                    } else {
                                        Color32::TRANSPARENT
                                    };

                                    ui.painter().rect_filled(item_rect, Rounding::same(4.0), fill_c);

                                    // Left label name & active state color
                                    let lbl_col = if enabled { theme::accent() } else { theme::TEXT_DIM };
                                    ui.painter().text(
                                        item_rect.left_center() + Vec2::new(8.0, 0.0),
                                        Align2::LEFT_CENTER,
                                        name.clone(),
                                        FontId::proportional(11.5),
                                        lbl_col,
                                    );

                                    // Right toggle button + keybind indicator
                                    let right_align_x = item_rect.right() - 8.0;

                                    // Open modal on item click
                                    if item_resp.clicked() {
                                        ui.ctx().data_mut(|d| {
                                            d.insert_temp(Id::new("clickgui_selected_module"), id);
                                            d.insert_temp(Id::new("clickgui_show_modal"), true);
                                        });
                                    }

                                    // Indicator for keybind or click
                                    ui.painter().text(
                                        Pos2::new(right_align_x - 40.0, item_rect.center().y),
                                        Align2::RIGHT_CENTER,
                                        "⚙",
                                        FontId::proportional(11.0),
                                        theme::TEXT_MUTED,
                                    );

                                    // Switch Toggle directly inside the item
                                    let switch_zone = Rect::from_min_size(
                                        Pos2::new(right_align_x - 28.0, item_rect.center().y - 6.0),
                                        Vec2::new(26.0, 12.0),
                                    );
                                    let switch_resp = ui.interact(switch_zone, Id::new("mod_grid_sw").with(id), Sense::click());
                                    if switch_resp.clicked() {
                                        if let Some(arc) = registry.get(&id) {
                                            let mut m = arc.lock().unwrap();
                                            let next_st = !enabled;
                                            m.get_module_data_mut().set_enabled(next_st);
                                            if crate::state::client().modules.is_active() {
                                                let res = if next_st { m.on_start() } else { m.on_stop() };
                                                if let Err(e) = res {
                                                    m.get_module_data_mut().set_enabled(enabled);
                                                    Notification::send(
                                                        NotificationType::Warning,
                                                        "Transition Failed",
                                                        &format!("Failed to toggle module {}: {}", name, e),
                                                    );
                                                }
                                            }
                                        }
                                    }

                                    let sw_val = anim::toggle(ui.ctx(), Id::new("mod_grid_sw_anim").with(id), enabled, 0.14, Easing::InOut);
                                    let track_col = theme::lerp_color(Color32::from_rgb(38, 39, 48), theme::accent(), sw_val);
                                    ui.painter().rect_filled(switch_zone, Rounding::same(6.0), track_col);

                                    let kb_rad = 4.0;
                                    let kb_min = switch_zone.left() + 5.0;
                                    let kb_max = switch_zone.right() - 5.0;
                                    let kb_x = kb_min + (kb_max - kb_min) * sw_val;
                                    ui.painter().circle_filled(Pos2::new(kb_x, switch_zone.center().y), kb_rad, Color32::WHITE);

                                    ui.advance_cursor_after_rect(item_rect);
                                    ui.add_space(4.0);
                                }
                            });
                        });
                }
            });
        });
}

// ============================================================================
// 2. DETAILED SETTINGS MODAL (NoFall / Sub-module configurations)
// ============================================================================

fn draw_settings_modal_if_active(ctx: &Context, progress: f32) {
    let show_modal = ctx.data(|d| d.get_temp::<bool>(Id::new("clickgui_show_modal"))).unwrap_or(false);
    if !show_modal {
        return;
    }

    let selected_mod_id = ctx.data(|d| d.get_temp::<ModuleId>(Id::new("clickgui_selected_module"))).unwrap_or(ModuleId::NoFall);
    let registry = crate::state::client().modules.by_id();

    let screen_rect = ctx.screen_rect();
    // Centered horizontally slightly offset to the right as requested (glassmorphism overlay central-right)
    let modal_w = 380.0;
    let modal_h = 440.0;
    let modal_pos = Pos2::new(
        screen_rect.right() - modal_w - 40.0,
        screen_rect.top() + (screen_rect.height() - modal_h) / 2.0,
    );

    // Semi-transparent backdrop block for glassmorphism layout
    egui::Area::new(Id::new("settings_modal_backdrop"))
        .order(Order::Foreground)
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            ui.allocate_rect(screen_rect, egui::Sense::click());
        });

    let dim_painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("settings_modal_dim")));
    dim_painter.rect_filled(screen_rect, Rounding::ZERO, Color32::from_black_alpha(110));

    egui::Area::new(Id::new("settings_modal_overlay"))
        .current_pos(modal_pos)
        .order(Order::Tooltip)
        .show(ctx, |ui| {
            ui.set_opacity(progress);

            // Glassmorphism wrapper (translucent elevated card background with strong borders)
            egui::Frame::none()
                .fill(Color32::from_rgba_unmultiplied(22, 22, 27, 240)) // Glassy Surface
                .stroke(Stroke::new(1.5, theme::accent())) // Glowing blue accent border
                .rounding(Rounding::same(theme::RADIUS))
                .inner_margin(Margin::same(16.0))
                .show(ui, |ui| {
                    ui.set_min_size(Vec2::new(modal_w, modal_h));
                    ui.set_max_size(Vec2::new(modal_w, modal_h));

                    if let Some(arc) = registry.get(&selected_mod_id) {
                        let mut module = arc.lock().unwrap();
                        let enabled = module.get_module_data().enabled;
                        let module_name = module.get_module_data().name().to_string();
                        let description = module.get_module_data().description.clone();

                        // Modal Header: "NoFall + icon + close box"
                        ui.horizontal(|ui| {
                            // Sub-module icon representation
                            ui.label(RichText::new("🛠").font(FontId::proportional(15.0)).color(theme::TEAL));
                            ui.add_space(4.0);
                            ui.label(RichText::new(module_name.clone()).font(FontId::proportional(16.0)).strong().color(theme::TEXT));

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let close_btn = Button::new(RichText::new("✕").font(FontId::proportional(12.0)))
                                    .fill(Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE);
                                if ui.add(close_btn).clicked() {
                                    ui.ctx().data_mut(|d| d.insert_temp(Id::new("clickgui_show_modal"), false));
                                }
                            });
                        });
                        ui.add_space(2.0);
                        ui.label(RichText::new(description).font(FontId::proportional(10.5)).color(theme::TEXT_MUTED));

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // Slider, toggles, multi-choice options
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 12.0;

                            // Activation Bind Option
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Keybind Activator").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let bind_id = Id::new("kb_listen_modal").with(selected_mod_id);
                                    let listening = ui.data(|d| d.get_temp::<bool>(bind_id).unwrap_or(false));
                                    let data = module.get_module_data_mut();

                                    let caption = if listening {
                                        if capture_keybind(data, arc, &registry) {
                                            ui.data_mut(|d| d.insert_temp(bind_id, false));
                                        }
                                        "Waiting..."
                                    } else {
                                        &data.key_bind.to_string()
                                    };

                                    let btn_col = if listening { theme::TEAL } else { theme::TEXT_DIM };
                                    let btn = Button::new(RichText::new(caption).font(FontId::proportional(11.0)).color(btn_col))
                                        .fill(theme::SURFACE)
                                        .rounding(Rounding::same(4.0));
                                    if ui.add(btn).clicked() {
                                        ui.data_mut(|d| d.insert_temp(bind_id, !listening));
                                        if !listening {
                                            LAST_KEY_PRESSED.store(-1, Ordering::Relaxed);
                                        }
                                    }
                                });
                            });

                            // Enabled State Toggle
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Module Subsystem Active").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let id = Id::new("modal_power_tg").with(selected_mod_id);
                                    if draw_custom_toggle(ui, enabled, id).clicked() {
                                        let next = !enabled;
                                        module.get_module_data_mut().set_enabled(next);
                                        if crate::state::client().modules.is_active() {
                                            let res = if next { module.on_start() } else { module.on_stop() };
                                            if let Err(e) = res {
                                                module.get_module_data_mut().set_enabled(enabled);
                                                Notification::send(
                                                    NotificationType::Warning,
                                                    "Transition Failed",
                                                    &format!("Failed to toggle module {}: {}", module_name, e),
                                                );
                                            }
                                        }
                                    }
                                });
                            });

                            ui.separator();

                            // Controls lists
                            let data = module.get_module_data_mut();

                            egui::ScrollArea::vertical()
                                .id_salt("modal_details_scroll")
                                .max_height(240.0)
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.y = 12.0;

                                    if data.settings.is_empty() {
                                        ui.centered_and_justified(|ui| {
                                            ui.label(RichText::new("No advanced properties found").font(FontId::proportional(11.0)).color(theme::TEXT_MUTED));
                                        });
                                    }

                                    for setting in &mut data.settings {
                                        match setting {
                                            ModuleSetting::Toggle { name, value } => {
                                                let setting_name = name.clone();
                                                ui.horizontal(|ui| {
                                                    ui.label(RichText::new(setting_name.clone()).font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
                                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                        let id = Id::new("modal_stg_tg").with(&module_name).with(&setting_name);
                                                        if draw_custom_toggle(ui, *value, id).clicked() {
                                                            *value = !*value;
                                                        }
                                                    });
                                                });
                                            }
                                            ModuleSetting::Slider { name, value, min, max } => {
                                                let setting_name = name.clone();
                                                let slider_min = *min;
                                                let slider_max = *max;
                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new(setting_name).font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
                                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                            ui.label(RichText::new(format!("{:.2}", value)).font(FontId::proportional(11.5)).strong().color(theme::accent()));
                                                        });
                                                    });
                                                    let mut val_f64 = *value as f64;
                                                    if ui.add(egui::Slider::new(&mut val_f64, (slider_min as f64)..=(slider_max as f64)).show_value(false)).changed() {
                                                        *value = val_f64 as f32;
                                                    }
                                                });
                                            }
                                            ModuleSetting::Choice { name, value, options } => {
                                                let setting_name = name.clone();
                                                let active_option = options.get(*value).map(String::as_str).unwrap_or("—");
                                                let options_clone = options.clone();
                                                ui.vertical(|ui| {
                                                    ui.label(RichText::new(setting_name.clone()).font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
                                                    egui::ComboBox::from_id_salt(Id::new("modal_comb").with(setting_name))
                                                        .width(ui.available_width())
                                                        .selected_text(RichText::new(active_option).font(FontId::proportional(11.5)))
                                                        .show_ui(ui, |ui| {
                                                            for (idx, opt) in options_clone.iter().enumerate() {
                                                                ui.selectable_value(value, idx, opt.as_str());
                                                            }
                                                        });
                                                });
                                            }
                                            ModuleSetting::Color { name, value } => {
                                                let setting_name = name.clone();
                                                ui.horizontal(|ui| {
                                                    ui.label(RichText::new(setting_name).font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
                                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                                        let mut rgba = egui::Rgba::from_rgba_unmultiplied(value[0], value[1], value[2], 1.0);
                                                        if egui::color_picker::color_edit_button_rgba(ui, &mut rgba, egui::color_picker::Alpha::Opaque).changed() {
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
                                });
                        });
                    }
                });
        });
}

// ============================================================================
// 3. CONFIGS TAB (Integrated profile boards & setup)
// ============================================================================

fn draw_configs_tab(ui: &mut Ui) {
    ui.label(RichText::new("PROFILE MANAGEMENT").font(FontId::proportional(18.0)).strong().color(theme::TEXT));
    ui.add_space(4.0);
    ui.label(RichText::new("Create, manage and load localized setup configurations directly within the main grid.").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
    ui.add_space(16.0);

    let active_prof = crate::config::active_profile();
    let list_prof = crate::config::list_profiles();

    egui::ScrollArea::vertical()
        .id_salt("configs_scroll")
        .max_height(400.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);
                let card_w = (ui.available_width() - 14.0) / 2.0;

                // Dedicated Config card directly in grid
                for prof in list_prof {
                    let is_active = prof == active_prof;

                    egui::Frame::none()
                        .fill(if is_active { theme::ELEVATED } else { theme::SURFACE })
                        .stroke(Stroke::new(1.0, if is_active { theme::TEAL } else { theme::BORDER }))
                        .rounding(Rounding::same(theme::RADIUS_INNER))
                        .inner_margin(Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.set_min_width(card_w);
                            ui.set_max_width(card_w);

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("📁 {}", prof)).font(FontId::proportional(13.0)).strong().color(theme::TEXT));
                                if is_active {
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(RichText::new("● Active").font(FontId::proportional(10.0)).color(theme::TEAL));
                                    });
                                }
                            });

                            ui.add_space(8.0);
                            ui.label(RichText::new("Locally cached settings and parameters.").font(FontId::proportional(10.5)).color(theme::TEXT_MUTED));
                            ui.add_space(12.0);

                            // Row of interactive actions
                            ui.horizontal(|ui| {
                                let load_btn = Button::new(RichText::new("Load").font(FontId::proportional(10.5)).color(theme::TEXT))
                                    .fill(theme::SURFACE)
                                    .rounding(Rounding::same(4.0));
                                if ui.add_enabled(!is_active, load_btn).clicked() {
                                    match crate::config::switch_profile(&prof) {
                                        Ok(_) => {
                                            Notification::send(NotificationType::Info, "Profile Loaded", &format!("Successfully switched parameters to profile: {prof}"));
                                        }
                                        Err(e) => {
                                            Notification::send(NotificationType::Warning, "Profile Load Failed", &format!("Failed to switch profile: {e}"));
                                        }
                                    }
                                }

                                let del_btn = Button::new(RichText::new("Delete").font(FontId::proportional(10.5)).color(theme::PANIC))
                                    .fill(theme::SURFACE)
                                    .rounding(Rounding::same(4.0));
                                if ui.add_enabled(prof != "Default", del_btn).clicked() {
                                    match crate::config::delete_profile(&prof) {
                                        Ok(_) => {
                                            Notification::send(NotificationType::Warning, "Profile Deleted", &format!("Deleted local config file: {prof}"));
                                        }
                                        Err(e) => {
                                            Notification::send(NotificationType::Warning, "Profile Delete Failed", &format!("Failed to delete profile: {e}"));
                                        }
                                    }
                                }
                            });
                        });
                }

                // Add New Profile card
                egui::Frame::none()
                    .fill(theme::SURFACE)
                    .stroke(Stroke::new(1.0, theme::BORDER))
                    .rounding(Rounding::same(theme::RADIUS_INNER))
                    .inner_margin(Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.set_min_width(card_w);
                        ui.set_max_width(card_w);

                        ui.label(RichText::new("＋ Create New Profile").font(FontId::proportional(13.0)).strong().color(theme::TEXT));
                        ui.add_space(8.0);

                        let mut new_p_name = ui.data(|d| d.get_temp::<String>(Id::new("dash_new_profile_input"))).unwrap_or_default();
                        let text_edit = ui.text_edit_singleline(&mut new_p_name);
                        if text_edit.changed() {
                            ui.data_mut(|d| d.insert_temp(Id::new("dash_new_profile_input"), new_p_name.clone()));
                        }

                        ui.add_space(10.0);
                        let create_btn = Button::new(RichText::new("Add Profile Setup").font(FontId::proportional(10.5)).color(theme::TEXT))
                            .fill(theme::ELEVATED)
                            .rounding(Rounding::same(4.0));

                        if ui.add(create_btn).clicked() && !new_p_name.trim().is_empty() {
                            let name_clean = new_p_name.trim().to_string();
                            match crate::config::create_profile(&name_clean) {
                                Ok(_) => {
                                    ui.data_mut(|d| d.insert_temp(Id::new("dash_new_profile_input"), String::new()));
                                    Notification::send(NotificationType::Info, "Profile Initialized", &format!("Initialized new local setup: {name_clean}"));
                                }
                                Err(e) => {
                                    Notification::send(NotificationType::Warning, "Profile Creation Failed", &format!("Failed to create profile: {e}"));
                                }
                            }
                        }
                    });
            });
        });
}

// ============================================================================
// 4. SCRIPTS TAB (Visual lua script indicators)
// ============================================================================

fn draw_scripts_tab(ui: &mut Ui) {
    ui.label(RichText::new("DYNAMIC LUA ENGINE").font(FontId::proportional(18.0)).strong().color(theme::TEXT));
    ui.add_space(4.0);
    ui.label(RichText::new("Directly manage custom script parameters and variables loaded dynamically into the client backend.").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
    ui.add_space(16.0);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);
        let card_w = (ui.available_width() - 14.0) / 2.0;

        // Simulated local script card
        egui::Frame::none()
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .rounding(Rounding::same(theme::RADIUS_INNER))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_min_width(card_w);
                ui.set_max_width(card_w);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("📜 PingSpoof.lua").font(FontId::proportional(13.0)).strong().color(theme::TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("● Active").font(FontId::proportional(10.0)).color(theme::TEAL));
                    });
                });
                ui.add_space(6.0);
                ui.label(RichText::new("Spoof client ping packet latency with user defined thresholds.").font(FontId::proportional(10.5)).color(theme::TEXT_MUTED));
                ui.add_space(12.0);

                let mut delay = ui.data(|d| d.get_temp::<f32>(Id::new("pingspoof_delay"))).unwrap_or(120.0);
                ui.label(RichText::new(format!("Latency Buffer: {:.0} ms", delay)).font(FontId::proportional(11.0)).color(theme::TEXT_DIM));
                if ui.add(egui::Slider::new(&mut delay, 20.0..=500.0).show_value(false)).changed() {
                    ui.ctx().data_mut(|d| d.insert_temp(Id::new("pingspoof_delay"), delay));
                }
            });

        // Script compilation status card
        egui::Frame::none()
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .rounding(Rounding::same(theme::RADIUS_INNER))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_min_width(card_w);
                ui.set_max_width(card_w);

                ui.label(RichText::new("⚙ Compiler Status").font(FontId::proportional(13.0)).strong().color(theme::TEXT));
                ui.add_space(6.0);
                ui.label(RichText::new("Lua environment fully operational. Registered hooks: onPacketSend, onUpdate.").font(FontId::proportional(10.5)).color(theme::TEXT_MUTED));
                ui.add_space(12.0);

                if ui.button(RichText::new("Re-scan assets/scripts/").font(FontId::proportional(10.5))).clicked() {
                    Notification::send(NotificationType::Info, "Script reloaded", "Parsed script catalogs successfully.");
                }
            });
    });
}

// ============================================================================
// 5. COMMUNITY TAB (Integrated reviews feed / top ratings panel directly in grid)
// ============================================================================

fn draw_community_tab(ui: &mut Ui) {
    ui.label(RichText::new("INTEGRATED COMMUNITY BOARD").font(FontId::proportional(18.0)).strong().color(theme::TEXT));
    ui.add_space(4.0);
    ui.label(RichText::new("Browse high performance community presets, reviews, and configuration files directly in the main layout.").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
    ui.add_space(16.0);

    // Grid layout containing reviews and config sharing
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);
        let card_w = (ui.available_width() - 14.0) / 2.0;

        // PRESENTS PREVIEW CARD
        egui::Frame::none()
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .rounding(Rounding::same(theme::RADIUS_INNER))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_min_width(card_w);
                ui.set_max_width(card_w);

                ui.label(RichText::new("👥 Verified Community Configs").font(FontId::proportional(13.0)).strong().color(theme::TEXT));
                ui.add_space(4.0);
                ui.label(RichText::new("Top rated presets downloaded by users:").font(FontId::proportional(10.0)).color(theme::TEXT_MUTED));
                ui.add_space(8.0);

                let presets = [
                    ("Legit Hypixel", "★★★★★ (48)", theme::TEAL),
                    ("AAC Speedrun", "★★★★☆ (12)", theme::accent()),
                ];

                for (name, rat, accent_c) in presets {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(name).font(FontId::proportional(11.0)).strong().color(theme::TEXT));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let dl_btn = Button::new(RichText::new("Preview").font(FontId::proportional(9.5)))
                                .fill(theme::ELEVATED)
                                .stroke(Stroke::new(1.0, accent_c));
                            if ui.add(dl_btn).clicked() {
                                Notification::send(NotificationType::Info, "Preset Preview", &format!("Preview Mode: Visualizing details for community setup: {name} (interactive imports are unavailable)"));
                            }
                            ui.label(RichText::new(rat).font(FontId::proportional(10.0)).color(Color32::from_rgb(250, 204, 21)));
                        });
                    });
                    ui.add_space(6.0);
                }
            });

        // REVIEWS FEED CARD
        egui::Frame::none()
            .fill(theme::SURFACE)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .rounding(Rounding::same(theme::RADIUS_INNER))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_min_width(card_w);
                ui.set_max_width(card_w);

                ui.label(RichText::new("💬 Presets & Setup Reviews").font(FontId::proportional(13.0)).strong().color(theme::TEXT));
                ui.add_space(8.0);

                // Embedded review board
                let reviews = [
                    ConfigReview {
                        user: "shoffli",
                        rating: 5,
                        text: "Absolutely stellar Bypass config, zero lag flags on Hypixel!",
                        date: "1h ago",
                        avatar_color: theme::TEAL,
                    },
                    ConfigReview {
                        user: "wallhacks",
                        rating: 4,
                        text: "Excellent aimassist, really smooth mouse speed interpolation.",
                        date: "5h ago",
                        avatar_color: theme::accent(),
                    },
                ];

                for r in reviews {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let (av_rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                            ui.painter().circle_filled(av_rect.center(), 7.0, r.avatar_color);

                            ui.label(RichText::new(r.user).font(FontId::proportional(11.0)).strong().color(theme::TEXT));
                            let rating_val = r.rating.clamp(0, 5);
                            let stars = "★".repeat(rating_val) + &"☆".repeat(5 - rating_val);
                            ui.label(RichText::new(stars).font(FontId::proportional(10.0)).color(Color32::from_rgb(250, 204, 21)));

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new(r.date).font(FontId::proportional(9.5)).color(theme::TEXT_MUTED));
                            });
                        });
                        ui.label(RichText::new(r.text).font(FontId::proportional(10.0)).color(theme::TEXT_DIM));
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                    });
                }
            });
    });
}

// ============================================================================
// HELPER METHODS
// ============================================================================

fn capture_keybind(data: &mut ModuleData, arc: &ModuleArc, registry: &ModuleMap) -> bool {
    let pressed = LAST_KEY_PRESSED.swap(-1, Ordering::Relaxed);
    if pressed == -1 {
        return false;
    }
    let key = KeyboardKey::from(pressed);
    if key == KeyboardKey::KeyNone {
        return false;
    }
    if key == KeyboardKey::KeyEscape {
        data.key_bind = KeyboardKey::KeyNone;
        return true;
    }

    for other in registry.values() {
        if Arc::ptr_eq(arc, other) {
            continue;
        }
        let owner = other.lock().unwrap();
        if owner.get_module_data().key_bind == key {
            let o_name = owner.get_module_data().name().to_string();
            drop(owner);
            Notification::send(NotificationType::Warning, "Keybind in use", &format!("'{o_name}' already bound to {key}"));
            return true;
        }
    }
    data.key_bind = key;
    true
}
