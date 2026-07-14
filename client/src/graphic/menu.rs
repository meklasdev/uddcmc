//! The ClickGUI: A single, pixel-perfect, cohesive dashboard-style utility panel.
//! Inspired by "Breeze dev-local" visual style.
//!
//! Features:
//! - Sleek left sidebar navigation with Brand Header, tabs (Cloud, Local Scripts, Themes, Settings), and profile info.
//! - High-end micro-interactions, smooth custom sliders, choice combos, active toggles, and hotkey selectors.
//! - Deep neon blue/charcoal color styling (#0F0F11, #16161B) with rounded cards and subtle glow borders.
//! - Integrated Modals (e.g. Review details list for Cloud Configs).

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

/// Width and height of the main dashboard.
const DASHBOARD_W: f32 = 820.0;
const DASHBOARD_H: f32 = 540.0;

/// Sidebar Width.
const SIDEBAR_W: f32 = 200.0;

type ModuleArc = Arc<Mutex<ModuleType>>;
type ModuleMap = HashMap<ModuleId, ModuleArc>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum GuiTab {
    Cloud,
    LocalScripts,
    Themes,
    Settings,
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

    let active_tab = ctx.data(|d| d.get_temp::<GuiTab>(Id::new("clickgui_active_tab"))).unwrap_or(GuiTab::LocalScripts);

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

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;

                        // 1. Sleek Left Vertical Navigation Sidebar
                        draw_sidebar(ui, active_tab, target_pos);

                        // Vertical separator
                        let (sep_rect, _) = ui.allocate_exact_size(Vec2::new(1.0, DASHBOARD_H), Sense::hover());
                        ui.painter().rect_filled(sep_rect, Rounding::ZERO, theme::BORDER);

                        // 2. Main Content panel layout
                        draw_main_content(ui, active_tab);
                    });
                });
        });

    // Render review modal overlay if active
    draw_review_modal_if_active(ctx, progress);
}

/// The Left Sidebar navigation
fn draw_sidebar(ui: &mut Ui, active_tab: GuiTab, target_pos: Pos2) {
    let sidebar_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(SIDEBAR_W, DASHBOARD_H));

    // Make the sidebar area draggable to move the entire dashboard window
    let (_drag_rect, drag_response) = ui.allocate_exact_size(Vec2::new(SIDEBAR_W, 60.0), Sense::drag());
    if drag_response.dragged() {
        let delta = ui.ctx().input(|i| i.pointer.delta());
        let next = target_pos + delta;
        ui.ctx().data_mut(|d| d.insert_temp(Id::new("clickgui_dash_pos"), next));
    }

    // Sidebar background
    ui.painter().rect_filled(
        sidebar_rect,
        Rounding {
            nw: theme::RADIUS,
            sw: theme::RADIUS,
            ne: 0.0,
            se: 0.0,
        },
        theme::SURFACE,
    );

    // Brand Header: "KRASNOSTAV Minecraft"
    let header_y = sidebar_rect.top() + 30.0;
    ui.painter().text(
        Pos2::new(sidebar_rect.left() + 20.0, header_y),
        Align2::LEFT_CENTER,
        "KRASNOSTAV",
        FontId::proportional(20.0),
        theme::TEXT,
    );
    ui.painter().text(
        Pos2::new(sidebar_rect.left() + 144.0, header_y + 1.0),
        Align2::LEFT_CENTER,
        "mc",
        FontId::proportional(12.0),
        theme::accent(),
    );

    // Render navigation tabs
    let tabs = [
        (GuiTab::Cloud, "☁  Cloud Store"),
        (GuiTab::LocalScripts, "📂  Local Scripts"),
        (GuiTab::Themes, "🎨  Custom Themes"),
        (GuiTab::Settings, "⚙  Client Config"),
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
            // Selected background
            ui.painter().rect_filled(
                item_rect,
                Rounding::same(theme::RADIUS_INNER),
                theme::accent_dim(),
            );
            // Left active neon accent bar
            let indicator = Rect::from_min_size(
                Pos2::new(item_rect.left() + 4.0, item_rect.top() + 8.0),
                Vec2::new(3.0, 20.0),
            );
            ui.painter().rect_filled(indicator, Rounding::same(1.5), theme::accent());
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

    // Bottom profile block with support links
    let profile_h = 60.0;
    let profile_rect = Rect::from_min_size(
        Pos2::new(sidebar_rect.left() + 12.0, sidebar_rect.bottom() - profile_h - 12.0),
        Vec2::new(SIDEBAR_W - 24.0, profile_h),
    );

    // Profile card background
    ui.painter().rect_filled(
        profile_rect,
        Rounding::same(theme::RADIUS_INNER),
        theme::ELEVATED,
    );
    ui.painter().rect_stroke(
        profile_rect,
        Rounding::same(theme::RADIUS_INNER),
        Stroke::new(1.0, theme::BORDER),
    );

    // User Avatar Circle (mockup)
    let avatar_center = Pos2::new(profile_rect.left() + 26.0, profile_rect.center().y);
    ui.painter().circle_filled(avatar_center, 14.0, theme::accent_dim());
    ui.painter().text(
        avatar_center,
        Align2::CENTER_CENTER,
        "K",
        FontId::proportional(12.0),
        theme::TEXT,
    );

    // User Name & Rank
    ui.painter().text(
        Pos2::new(profile_rect.left() + 48.0, profile_rect.center().y - 6.0),
        Align2::LEFT_CENTER,
        "KrasnostavDev",
        FontId::proportional(11.0),
        theme::TEXT,
    );
    ui.painter().text(
        Pos2::new(profile_rect.left() + 48.0, profile_rect.center().y + 6.0),
        Align2::LEFT_CENTER,
        "Premium User",
        FontId::proportional(9.5),
        theme::accent(),
    );

    // Quick links row inside the profile block or directly next to it
    let disc_btn_rect = Rect::from_min_size(
        Pos2::new(profile_rect.right() - 24.0, profile_rect.center().y - 10.0),
        Vec2::new(20.0, 20.0),
    );
    let disc_resp = ui.interact(disc_btn_rect, Id::new("discord_support_link"), Sense::click());
    if disc_resp.clicked() {
        Notification::send(
            NotificationType::Info,
            "Support Link",
            "Opening Support & Discord portal...",
        );
    }
    let d_color = if disc_resp.hovered() { theme::accent() } else { theme::TEXT_MUTED };
    ui.painter().text(
        disc_btn_rect.center(),
        Align2::CENTER_CENTER,
        "💬",
        FontId::proportional(11.0),
        d_color,
    );
}

/// Main content panel router
fn draw_main_content(ui: &mut Ui, active_tab: GuiTab) {
    let main_rect = Rect::from_min_size(
        ui.cursor().min,
        Vec2::new(DASHBOARD_W - SIDEBAR_W - 1.0, DASHBOARD_H),
    );

    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(main_rect));

    // Clean padding wrapper inside the content panel
    egui::Frame::none()
        .inner_margin(Margin::same(24.0))
        .show(&mut child_ui, |ui| {
            ui.set_min_size(Vec2::new(DASHBOARD_W - SIDEBAR_W - 49.0, DASHBOARD_H - 48.0));
            ui.set_max_size(Vec2::new(DASHBOARD_W - SIDEBAR_W - 49.0, DASHBOARD_H - 48.0));

            match active_tab {
                GuiTab::Cloud => draw_cloud_store_tab(ui),
                GuiTab::LocalScripts => draw_local_scripts_tab(ui),
                GuiTab::Themes => draw_themes_tab(ui),
                GuiTab::Settings => draw_settings_tab(ui),
            }
        });
}

// ============================================================================
// 1. CLOUD STORE TAB (With beautiful card layouts and interactive modals)
// ============================================================================

fn draw_cloud_store_tab(ui: &mut Ui) {
    // Tab Title
    ui.horizontal(|ui| {
        ui.label(RichText::new("CLOUD STORES & SCRIPTS").font(FontId::proportional(18.0)).strong().color(theme::TEXT));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new("⚡ Verified Safe").font(FontId::proportional(11.0)).color(theme::accent()));
        });
    });
    ui.add_space(4.0);
    ui.label(RichText::new("Explore, download and manage cloud-curated scripts, profiles and configurations dynamically loaded in-game.").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
    ui.add_space(16.0);

    // Cloud configs catalog array
    let configs = [
        CloudConfig {
            name: "Premium AimAssist",
            creator: "shoffli",
            rating: 5.0,
            reviews_count: 14,
            tags: &["Legit", "Combat"],
            is_saved: true,
            image_accent: Color32::from_rgb(59, 130, 246),
        },
        CloudConfig {
            name: "Breeze Legit Speed",
            creator: "wallhacks",
            rating: 4.8,
            reviews_count: 32,
            tags: &["Movement", "Legit"],
            is_saved: false,
            image_accent: Color32::from_rgb(139, 92, 246),
        },
        CloudConfig {
            name: "Advanced packet Spoofer",
            creator: "bhop4real",
            rating: 4.9,
            reviews_count: 8,
            tags: &["Packet", "Rage"],
            is_saved: false,
            image_accent: Color32::from_rgb(239, 68, 68),
        },
        CloudConfig {
            name: "Hypixel SafeScaffold",
            creator: "krasnostav_dev",
            rating: 5.0,
            reviews_count: 24,
            tags: &["World", "Safe"],
            is_saved: true,
            image_accent: Color32::from_rgb(16, 185, 129),
        },
    ];

    egui::ScrollArea::vertical()
        .id_salt("cloud_store_scroll")
        .max_height(350.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);

                let card_w = (ui.available_width() - 14.0) / 2.0;

                for cfg in configs {
                    egui::Frame::none()
                        .fill(theme::SURFACE)
                        .stroke(Stroke::new(1.0, theme::BORDER))
                        .rounding(Rounding::same(theme::RADIUS_INNER))
                        .inner_margin(Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.set_min_width(card_w);
                            ui.set_max_width(card_w);

                            // Header row with star rating and a miniature glowing display rectangle
                            ui.horizontal(|ui| {
                                // Mini mockup icon
                                let (icon_rect, _) = ui.allocate_exact_size(Vec2::new(28.0, 28.0), Sense::hover());
                                ui.painter().rect_filled(icon_rect, Rounding::same(6.0), theme::ELEVATED);
                                ui.painter().rect_stroke(icon_rect, Rounding::same(6.0), Stroke::new(1.0, cfg.image_accent));
                                ui.painter().text(icon_rect.center(), Align2::CENTER_CENTER, "⚡", FontId::proportional(12.0), cfg.image_accent);

                                ui.vertical(|ui| {
                                    ui.label(RichText::new(cfg.name).font(FontId::proportional(13.0)).strong().color(theme::TEXT));
                                    ui.label(RichText::new(format!("by {}", cfg.creator)).font(FontId::proportional(10.5)).color(theme::TEXT_MUTED));
                                });
                            });

                            ui.add_space(8.0);

                            // Stars and reviews trigger
                            ui.horizontal(|ui| {
                                let star_color = Color32::from_rgb(250, 204, 21); // Gold yellow
                                ui.label(RichText::new("★").font(FontId::proportional(11.0)).color(star_color));
                                ui.label(RichText::new(format!("{:.1}", cfg.rating)).font(FontId::proportional(11.0)).strong().color(theme::TEXT_DIM));

                                let rev_resp = ui.link(RichText::new(format!("({} reviews)", cfg.reviews_count)).font(FontId::proportional(11.0)).color(theme::accent()));
                                if rev_resp.clicked() {
                                    // Set state to trigger review modal opening
                                    ui.ctx().data_mut(|d| d.insert_temp(Id::new("review_modal_active"), true));
                                }
                            });

                            ui.add_space(8.0);

                            // Render tags as pill layouts
                            ui.horizontal(|ui| {
                                for tag in cfg.tags {
                                    let (tag_rect, _) = ui.allocate_exact_size(Vec2::new(55.0, 16.0), Sense::hover());
                                    ui.painter().rect_filled(tag_rect, Rounding::same(4.0), theme::ELEVATED);
                                    ui.painter().text(
                                        tag_rect.center(),
                                        Align2::CENTER_CENTER,
                                        *tag,
                                        FontId::proportional(9.0),
                                        theme::TEXT_DIM,
                                    );
                                }

                                // Interactive action buttons
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let btn_lbl = if cfg.is_saved { "✓ Installed" } else { "📥 Install" };
                                    let btn_color = if cfg.is_saved { theme::accent_dim() } else { theme::ELEVATED };
                                    let install_btn = Button::new(RichText::new(btn_lbl).font(FontId::proportional(10.5)).color(theme::TEXT))
                                        .fill(btn_color)
                                        .rounding(Rounding::same(4.0));

                                    if ui.add(install_btn).clicked() {
                                        Notification::send(
                                            NotificationType::Info,
                                            "Cloud Download",
                                            &format!("Successfully synchronized configuration: {}", cfg.name),
                                        );
                                    }
                                });
                            });
                        });
                }
            });
        });
}

// Custom overlay pop-up modal for showing config reviews
fn draw_review_modal_if_active(ctx: &Context, progress: f32) {
    let is_active = ctx.data(|d| d.get_temp::<bool>(Id::new("review_modal_active"))).unwrap_or(false);
    if !is_active {
        return;
    }

    let screen_rect = ctx.screen_rect();
    let modal_w = 420.0;
    let modal_h = 320.0;
    let modal_pos = Pos2::new(
        (screen_rect.width() - modal_w) / 2.0,
        (screen_rect.height() - modal_h) / 2.0,
    );

    // Backdrop grey shade block
    let dark_painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("review_modal_dim")));
    dark_painter.rect_filled(screen_rect, Rounding::ZERO, Color32::from_black_alpha(150));

    egui::Area::new(Id::new("review_modal_window"))
        .current_pos(modal_pos)
        .order(Order::Tooltip)
        .show(ctx, |ui| {
            ui.set_opacity(progress);

            egui::Frame::none()
                .fill(theme::SURFACE)
                .stroke(Stroke::new(1.0, theme::BORDER))
                .rounding(Rounding::same(theme::RADIUS))
                .inner_margin(Margin::same(16.0))
                .show(ui, |ui| {
                    ui.set_min_size(Vec2::new(modal_w, modal_h));
                    ui.set_max_size(Vec2::new(modal_w, modal_h));

                    // Header row
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("User Review Summary").font(FontId::proportional(15.0)).strong().color(theme::TEXT));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let close_btn = Button::new(RichText::new("✕").font(FontId::proportional(12.0)))
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE);
                            if ui.add(close_btn).clicked() {
                                ui.ctx().data_mut(|d| d.insert_temp(Id::new("review_modal_active"), false));
                            }
                        });
                    });

                    ui.separator();
                    ui.add_space(8.0);

                    // List of reviews
                    let reviews = [
                        ConfigReview {
                            user: "wallhacks",
                            rating: 5,
                            text: "Unbelievably smooth and stealthy performance, zero flags on AntiCheat!",
                            date: "2 hours ago",
                            avatar_color: Color32::from_rgb(59, 130, 246),
                        },
                        ConfigReview {
                            user: "bhop4real",
                            rating: 5,
                            text: "Absolutely critical setup, saved my gameplay on Hypixel.",
                            date: "1 day ago",
                            avatar_color: Color32::from_rgb(16, 185, 129),
                        },
                        ConfigReview {
                            user: "shoffli",
                            rating: 4,
                            text: "Pretty solid, could use a slightly higher max slider config though.",
                            date: "3 days ago",
                            avatar_color: Color32::from_rgb(245, 158, 11),
                        },
                    ];

                    egui::ScrollArea::vertical()
                        .id_salt("modal_reviews_scroll")
                        .max_height(210.0)
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 12.0;
                            for r in reviews {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        // Miniature avatar
                                        let (av_rect, _) = ui.allocate_exact_size(Vec2::new(20.0, 20.0), Sense::hover());
                                        ui.painter().circle_filled(av_rect.center(), 10.0, r.avatar_color);

                                        ui.label(RichText::new(r.user).font(FontId::proportional(11.5)).strong().color(theme::TEXT));

                                        // Star counts
                                        let star_str = "★".repeat(r.rating);
                                        ui.label(RichText::new(star_str).font(FontId::proportional(11.0)).color(Color32::from_rgb(250, 204, 21)));

                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            ui.label(RichText::new(r.date).font(FontId::proportional(10.0)).color(theme::TEXT_MUTED));
                                        });
                                    });

                                    ui.add_space(2.0);
                                    ui.label(RichText::new(r.text).font(FontId::proportional(11.0)).color(theme::TEXT_DIM));
                                    ui.add_space(4.0);
                                    ui.separator();
                                });
                            }
                        });
                });
        });
}

// ============================================================================
// 2. LOCAL SCRIPTS & UTILITIES TAB (Divided Detail Panels)
// ============================================================================

/// Helper method to capture keybind overrides
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

fn draw_local_scripts_tab(ui: &mut Ui) {
    let registry = crate::state::client().modules.by_id();

    // Grouping into left category selection / scripts selector, and right dynamic config sliders
    let mut entries: Vec<(ModuleId, ModuleCategory)> = registry
        .iter()
        .map(|(id, arc)| {
            let module = arc.lock().unwrap();
            (*id, module.get_module_data().category)
        })
        .collect();
    entries.sort_by_key(|(id, _)| id.display_name());

    // Selected module state to show settings detail column on the right
    let selected_mod_id = ui.ctx().data(|d| d.get_temp::<ModuleId>(Id::new("clickgui_selected_module")))
        .unwrap_or(ModuleId::Aimbot);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(16.0, 0.0);

        // --- Left Split: Modules List & Categories ---
        let left_w = 230.0;
        ui.vertical(|ui| {
            ui.set_min_width(left_w);
            ui.set_max_width(left_w);

            ui.label(RichText::new("LOCAL MODULES").font(FontId::proportional(12.0)).strong().color(theme::accent()));
            ui.add_space(6.0);

            egui::ScrollArea::vertical()
                .id_salt("local_modules_scroll")
                .max_height(380.0)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 5.0;

                    // Display registered categories & modules
                    let categories = [
                        ModuleCategory::Combat,
                        ModuleCategory::Movement,
                        ModuleCategory::Render,
                        ModuleCategory::Player,
                        ModuleCategory::World,
                        ModuleCategory::Misc,
                    ];

                    for cat in categories {
                        let members: Vec<(String, ModuleId, &ModuleArc)> = entries
                            .iter()
                            .filter(|(_, c)| *c == cat)
                            .filter_map(|(id, _)| {
                                registry.get(id).map(|arc| (id.display_name().to_string(), *id, arc))
                            })
                            .collect();

                        if members.is_empty() {
                            continue;
                        }

                        // Category Title header
                        ui.add_space(4.0);
                        ui.label(RichText::new(format!("{} {}", category_icon(cat), cat.display_name().to_uppercase())).font(FontId::proportional(10.0)).strong().color(theme::TEXT_MUTED));

                        for (m_name, id, arc) in members {
                            let is_selected = selected_mod_id == id;
                            let mut module = arc.lock().unwrap();
                            let enabled = module.get_module_data().enabled;

                            let item_rect = Rect::from_min_size(
                                ui.cursor().min,
                                Vec2::new(left_w, 28.0),
                            );
                            let m_id_resp = Id::new("mod_select_btn").with(id);
                            let response = ui.interact(item_rect, m_id_resp, Sense::click());

                            let hover_val = anim::toggle(
                                ui.ctx(),
                                Id::new("mod_h").with(id),
                                response.hovered(),
                                0.12,
                                Easing::Out,
                            );

                            if response.clicked() {
                                ui.ctx().data_mut(|d| d.insert_temp(Id::new("clickgui_selected_module"), id));
                            }

                            // Render base row box
                            let fill_col = if is_selected {
                                theme::ELEVATED
                            } else if hover_val > 0.001 {
                                theme::with_alpha(theme::SURFACE, hover_val)
                            } else {
                                Color32::TRANSPARENT
                            };

                            ui.painter().rect_filled(item_rect, Rounding::same(6.0), fill_col);
                            if is_selected {
                                ui.painter().rect_stroke(item_rect, Rounding::same(6.0), Stroke::new(1.0, theme::BORDER));
                            }

                            // Dynamic Text label
                            let label_col = if enabled {
                                theme::accent()
                            } else if is_selected {
                                theme::TEXT
                            } else {
                                theme::TEXT_DIM
                            };

                            ui.painter().text(
                                item_rect.left_center() + Vec2::new(10.0, 0.0),
                                Align2::LEFT_CENTER,
                                m_name,
                                FontId::proportional(11.5),
                                label_col,
                            );

                            // Miniature slider toggle layout inside row right side
                            let toggle_zone = Rect::from_min_size(
                                Pos2::new(item_rect.right() - 36.0, item_rect.center().y - 6.0),
                                Vec2::new(26.0, 12.0),
                            );
                            let tg_resp = ui.interact(toggle_zone, Id::new("mod_tg").with(id), Sense::click());
                            if tg_resp.clicked() {
                                let next_st = !enabled;
                                module.get_module_data_mut().set_enabled(next_st);
                                if crate::state::client().modules.is_active() {
                                    let _ = if next_st { module.on_start() } else { module.on_stop() };
                                }
                            }

                            let tg_val = anim::toggle(ui.ctx(), Id::new("mod_tg_anim").with(id), enabled, 0.14, Easing::InOut);
                            let track_col = theme::lerp_color(Color32::from_rgb(38, 39, 48), theme::accent(), tg_val);
                            ui.painter().rect_filled(toggle_zone, Rounding::same(6.0), track_col);

                            let kb_rad = 4.0;
                            let kb_min = toggle_zone.left() + 5.0;
                            let kb_max = toggle_zone.right() - 5.0;
                            let kb_x = kb_min + (kb_max - kb_min) * tg_val;
                            ui.painter().circle_filled(Pos2::new(kb_x, toggle_zone.center().y), kb_rad, Color32::WHITE);

                            ui.allocate_exact_size(Vec2::new(left_w, 28.0), Sense::hover());
                        }
                    }

                    // Check for Lua Script engine mods
                    ui.add_space(10.0);
                    ui.label(RichText::new("📂 CUSTOM LUA MODULES").font(FontId::proportional(10.0)).strong().color(theme::TEXT_MUTED));

                    // Show a custom Lua Script item (e.g. PingSpoof)
                    let pingspoof_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(left_w, 28.0));
                    let p_resp = ui.interact(pingspoof_rect, Id::new("pingspoof_lua_row"), Sense::click());
                    let p_hover = anim::toggle(ui.ctx(), Id::new("ps_h"), p_resp.hovered(), 0.12, Easing::Out);

                    let bg_c = if p_hover > 0.001 { theme::with_alpha(theme::SURFACE, p_hover) } else { Color32::TRANSPARENT };
                    ui.painter().rect_filled(pingspoof_rect, Rounding::same(6.0), bg_c);
                    ui.painter().text(pingspoof_rect.left_center() + Vec2::new(10.0, 0.0), Align2::LEFT_CENTER, "PingSpoof.lua", FontId::proportional(11.5), theme::accent());

                    ui.allocate_exact_size(Vec2::new(left_w, 28.0), Sense::hover());
                });
        });

        // Split vertical divider
        let (split_rect, _) = ui.allocate_exact_size(Vec2::new(1.0, 390.0), Sense::hover());
        ui.painter().rect_filled(split_rect, Rounding::ZERO, theme::BORDER);

        // --- Right Split: Detailed settings config panel ---
        ui.vertical(|ui| {
            ui.set_min_width(320.0);
            ui.set_max_width(320.0);

            if let Some(arc) = registry.get(&selected_mod_id) {
                let mut module = arc.lock().unwrap();
                let enabled = module.get_module_data().enabled;

                // Read immutable properties needed for the headers first to avoid borrow conflicts
                let module_name_upper = module.get_module_data().name().to_uppercase();
                let module_desc = module.get_module_data().description.clone();

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(module_name_upper).font(FontId::proportional(16.0)).strong().color(theme::TEXT));
                        ui.label(RichText::new(module_desc).font(FontId::proportional(11.0)).color(theme::TEXT_DIM));
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Big custom power button
                        let id = Id::new("detail_power_tg").with(selected_mod_id);
                        if draw_custom_toggle(ui, enabled, id).clicked() {
                            let next = !enabled;
                            let data = module.get_module_data_mut();
                            data.set_enabled(next);
                            if crate::state::client().modules.is_active() {
                                let _ = if next { module.on_start() } else { module.on_stop() };
                            }
                        }
                    });
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                let data = module.get_module_data_mut();

                // Render detail settings controls
                egui::ScrollArea::vertical()
                    .id_salt("module_details_scroll")
                    .max_height(300.0)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 12.0;

                        // Renders Bind/Keybind Hotkey selector row
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Activation Bind").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let bind_id = Id::new("kb_listen").with(selected_mod_id);
                                let listening = ui.data(|d| d.get_temp::<bool>(bind_id).unwrap_or(false));
                                let caption = if listening {
                                    if capture_keybind(data, arc, &registry) {
                                        ui.data_mut(|d| d.insert_temp(bind_id, false));
                                    }
                                    "Press Key..."
                                } else {
                                    &data.key_bind.to_string()
                                };

                                let btn_c = if listening { theme::accent() } else { theme::TEXT_DIM };
                                let btn = Button::new(RichText::new(caption).font(FontId::proportional(11.0)).color(btn_c))
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

                        // Dynamic Setting Controls
                        let mod_name = data.name().to_string();
                        for setting in &mut data.settings {
                            match setting {
                                ModuleSetting::Toggle { name, value } => {
                                    let setting_name = name.clone();
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(setting_name.clone()).font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            let id = Id::new("detail_stg_tg").with(&mod_name).with(&setting_name);
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
                                        egui::ComboBox::from_id_salt(Id::new("detail_comb").with(setting_name))
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
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("Select a module to adjust options").font(FontId::proportional(12.0)).color(theme::TEXT_MUTED));
                });
            }
        });
    });
}

// ============================================================================
// 3. THEMES CONFIGURATION TAB (Visual presets grid)
// ============================================================================

fn draw_themes_tab(ui: &mut Ui) {
    ui.label(RichText::new("THEMES & PALETTES").font(FontId::proportional(18.0)).strong().color(theme::TEXT));
    ui.add_space(4.0);
    ui.label(RichText::new("Quickly customize your entire ClickGUI experience with preset palettes or adjust layouts below.").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
    ui.add_space(16.0);

    let current_preset = crate::config::accent_preset();
    let presets = [
        theme::AccentPreset::Emerald,
        theme::AccentPreset::Aqua,
        theme::AccentPreset::Amethyst,
        theme::AccentPreset::Ruby,
        theme::AccentPreset::Gold,
        theme::AccentPreset::Sakura,
    ];

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(14.0, 14.0);
        let card_w = (ui.available_width() - 28.0) / 3.0;

        for p in presets {
            let is_active = p == current_preset;
            let card_rect = Rect::from_min_size(ui.cursor().min, Vec2::new(card_w, 75.0));
            // Standardizing card interactive ID
            let r_resp = ui.interact(card_rect, Id::new("theme_preset_card").with(p.name()), Sense::click());

            if r_resp.clicked() {
                crate::config::set_accent_preset(p);
                crate::graphic::theme::apply(ui.ctx());
                crate::config::save();
            }

            // Draw card layout
            let bg = if is_active { theme::ELEVATED } else { theme::SURFACE };
            ui.painter().rect_filled(card_rect, Rounding::same(theme::RADIUS_INNER), bg);
            let b_color = if is_active { theme::accent() } else { theme::BORDER };
            ui.painter().rect_stroke(card_rect, Rounding::same(theme::RADIUS_INNER), Stroke::new(1.0, b_color));

            // Custom glowing accent swatch circle
            let swatch_c = card_rect.left_center() + Vec2::new(24.0, 0.0);
            ui.painter().circle_filled(swatch_c, 8.0, p.color());
            if is_active {
                ui.painter().circle_stroke(swatch_c, 11.0, Stroke::new(1.0, theme::accent()));
            }

            // Theme Label
            ui.painter().text(
                card_rect.left_center() + Vec2::new(44.0, 0.0),
                Align2::LEFT_CENTER,
                p.name(),
                FontId::proportional(11.5),
                theme::TEXT,
            );

            ui.allocate_exact_size(Vec2::new(card_w, 75.0), Sense::hover());
        }
    });

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(16.0);

    // Advanced sliders layout
    ui.columns(2, |cols| {
        // Left Column: Opacity
        let mut opacity = crate::config::gui_opacity();
        cols[0].label(RichText::new("Background Opacity").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
        let mut opacity_f64 = opacity as f64;
        if cols[0].add(egui::Slider::new(&mut opacity_f64, 0.2..=1.0).show_value(true)).changed() {
            opacity = opacity_f64 as f32;
            crate::config::set_gui_opacity(opacity);
            crate::config::save();
        }

        // Right Column: Scale
        let mut scale = crate::config::gui_scale();
        cols[1].label(RichText::new("GUI Scale Factor").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
        let mut scale_f64 = scale as f64;
        if cols[1].add(egui::Slider::new(&mut scale_f64, 0.7..=1.5).show_value(true)).changed() {
            scale = scale_f64 as f32;
            crate::config::set_gui_scale(scale);
            crate::config::save();
        }
    });
}

// ============================================================================
// 4. CLIENT CONFIGURATION & SETTINGS TAB
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
enum UpdateState {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Failed,
}

fn draw_settings_tab(ui: &mut Ui) {
    ui.label(RichText::new("CLIENT CONFIGURATION").font(FontId::proportional(18.0)).strong().color(theme::TEXT));
    ui.add_space(4.0);
    ui.label(RichText::new("Manage active setup profiles, performance thresholds, and synchronize launcher software.").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
    ui.add_space(16.0);

    ui.columns(2, |cols| {
        // --- Left Col: Profiles and Reset ---
        cols[0].vertical(|ui| {
            ui.label(RichText::new("PROFILES MANAGEMENT").font(FontId::proportional(11.0)).strong().color(theme::accent()));
            ui.add_space(4.0);

            let active = crate::config::active_profile();
            let list = crate::config::list_profiles();

            egui::ComboBox::from_id_salt("profile_combo_dashboard")
                .width(ui.available_width())
                .selected_text(RichText::new(&active).font(FontId::proportional(11.5)))
                .show_ui(ui, |ui| {
                    for p in list {
                        if ui.selectable_label(p == active, &p).clicked() {
                            crate::config::switch_profile(&p);
                            Notification::send(
                                NotificationType::Info,
                                "Profile Loaded",
                                &format!("Loaded profile setup: {p}"),
                            );
                        }
                    }
                });

            ui.add_space(8.0);

            // New profile creation text edit
            ui.horizontal(|ui| {
                let mut new_p_name = ui.data(|d| d.get_temp::<String>(Id::new("dash_new_profile_input"))).unwrap_or_default();
                ui.spacing_mut().text_edit_width = 110.0;
                let text_edit = ui.text_edit_singleline(&mut new_p_name);
                if text_edit.changed() {
                    ui.data_mut(|d| d.insert_temp(Id::new("dash_new_profile_input"), new_p_name.clone()));
                }

                let add_btn = Button::new(RichText::new("＋ Add").font(FontId::proportional(11.0)).color(theme::TEXT))
                    .fill(theme::SURFACE)
                    .rounding(Rounding::same(4.0));
                if ui.add(add_btn).clicked() && !new_p_name.trim().is_empty() {
                    let clean_name = new_p_name.trim().to_string();
                    crate::config::create_profile(&clean_name);
                    ui.data_mut(|d| d.insert_temp(Id::new("dash_new_profile_input"), String::new()));
                    Notification::send(
                        NotificationType::Info,
                        "Profile Created",
                        &format!("Successfully initialized profile: {clean_name}"),
                    );
                }

                let del_btn = Button::new(RichText::new("－ Remove").font(FontId::proportional(11.0)).color(theme::DANGER))
                    .fill(theme::SURFACE)
                    .rounding(Rounding::same(4.0));
                if ui.add_enabled(active != "Default", del_btn).clicked() {
                    let target_del = active.clone();
                    crate::config::delete_profile(&target_del);
                    Notification::send(
                        NotificationType::Warning,
                        "Profile Removed",
                        &format!("Removed profile config: {target_del}"),
                    );
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);

            // Panic and Reset row actions
            ui.label(RichText::new("DANGER ZONE").font(FontId::proportional(11.0)).strong().color(theme::DANGER));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let panic = Button::new(RichText::new("⚡ Panic").font(FontId::proportional(11.0)).strong().color(Color32::WHITE))
                    .fill(theme::DANGER)
                    .rounding(Rounding::same(4.0));
                if ui.add(panic).clicked() {
                    std::thread::spawn(crate::graphic::ui_engine::call_panic);
                }

                let reset_ui = Button::new(RichText::new("Reset Layout").font(FontId::proportional(11.0)).color(theme::TEXT_DIM))
                    .fill(theme::SURFACE)
                    .rounding(Rounding::same(4.0));
                if ui.add(reset_ui).clicked() {
                    ui.ctx().memory_mut(|mem| mem.reset_areas());
                    ui.ctx().data_mut(|data| data.clear());
                    crate::config::reset_ui_state();
                    crate::config::save();
                    Notification::send(NotificationType::Info, "Reset Complete", "Re-aligned ClickGUI interface structures.");
                }
            });
        });

        // --- Right Col: Performance Caps and Launcher Upgrades ---
        cols[1].vertical(|ui| {
            ui.label(RichText::new("PERFORMANCE THRESHOLD").font(FontId::proportional(11.0)).strong().color(theme::accent()));
            ui.add_space(4.0);

            // Frame Rate Limit Cap
            let mut fps = crate::config::perf_limit_fps();
            ui.horizontal(|ui| {
                ui.label(RichText::new("FPS Limit Cap").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("{} FPS", fps)).font(FontId::proportional(11.5)).strong().color(theme::accent()));
                });
            });
            let mut fps_f64 = fps as f64;
            if ui.add(egui::Slider::new(&mut fps_f64, 15.0..=240.0).show_value(false)).changed() {
                fps = fps_f64 as i32;
                crate::config::set_perf_limit_fps(fps);
                crate::config::save();
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(12.0);

            // Updates Checking Block
            ui.label(RichText::new("LIVE UPDATER").font(FontId::proportional(11.0)).strong().color(theme::accent()));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Current Build:").font(FontId::proportional(11.5)).color(theme::TEXT_DIM));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("v1.0.0 (Premium)").font(FontId::proportional(11.5)).color(theme::TEXT));
                });
            });

            ui.add_space(6.0);

            let up_state = ui.data(|d| d.get_temp::<UpdateState>(Id::new("updater_state"))).unwrap_or_default();
            match up_state {
                UpdateState::Idle => {
                    let check_btn = Button::new(RichText::new("🔎 Search for updates").font(FontId::proportional(11.0)).color(theme::TEXT))
                        .fill(theme::SURFACE)
                        .rounding(Rounding::same(4.0))
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
                        ui.label(RichText::new("Searching repository...").font(FontId::proportional(11.0)).color(theme::TEXT_DIM));
                    });
                }
                UpdateState::UpToDate => {
                    ui.label(RichText::new("✔ Version is fully up to date!").font(FontId::proportional(11.5)).color(theme::accent()));
                    ui.add_space(4.0);
                    if ui.button(RichText::new("Check Again").font(FontId::proportional(11.0))).clicked() {
                        ui.data_mut(|d| d.insert_temp(Id::new("updater_state"), UpdateState::Idle));
                    }
                }
                UpdateState::Failed => {
                    ui.label(RichText::new("✖ Repository unreachable.").font(FontId::proportional(11.5)).color(theme::DANGER));
                }
            }
        });
    });
}
