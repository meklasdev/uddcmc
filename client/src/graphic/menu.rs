use crate::client::DarkClient;
use crate::graphic::ui_engine::WindowAnimState;
use crate::module::{ModuleCategory, ModuleSetting};
use egui::{Align2, Color32, Context, Id, Pos2, Rect, Rounding, Sense, Vec2};
use std::collections::HashMap;

pub fn draw(
    ctx: &Context,
    anim_progress: f32,
    window_anim_states: &mut HashMap<String, WindowAnimState>,
) {
    egui::Area::new(Id::new("dark_overlay"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Background)
        .interactable(false)
        .show(ctx, |ui| {
            ui.painter().rect_filled(
                ui.ctx().screen_rect(),
                0.0,
                Color32::from_black_alpha((180.0 * anim_progress) as u8),
            );
        });

    egui::Area::new(Id::new("global_buttons"))
        .anchor(Align2::RIGHT_TOP, Vec2::new(-10.0, 10.0))
        .show(ctx, |ui| {
            ui.set_opacity(anim_progress);
            ui.horizontal(|ui| {
                if ui
                    .button(egui::RichText::new("PANIC").color(Color32::RED))
                    .clicked()
                {
                    std::thread::spawn(|| crate::graphic::ui_engine::call_panic());
                }
                ui.add_space(20.0);
                if ui.button("Reset UI").clicked() {
                    window_anim_states.clear();
                    ctx.memory_mut(|mem| mem.reset_areas());
                    ctx.data_mut(|d| d.clear());
                }
            });
        });

    let client_modules_guard = DarkClient::instance().modules.read().unwrap();

    let categories = [
        ModuleCategory::COMBAT,
        ModuleCategory::MOVEMENT,
        ModuleCategory::RENDER,
        ModuleCategory::PLAYER,
        ModuleCategory::WORLD,
    ];

    let logical_width = ctx.screen_rect().width();

    let mut curr_x = 50.0;
    let mut curr_y = 50.0;
    let win_w = 160.0;
    let gap_x = 20.0;
    let row_height = 280.0;

    for category in categories.iter() {
        if curr_x + win_w > logical_width && curr_x > 50.0 {
            curr_x = 50.0;
            curr_y += row_height;
        }
        let title = category.display_name();

        let area_id = Id::new(title).with("area");

        let mut target_pos = Pos2::new(curr_x, curr_y);
        target_pos = ctx
            .data(|d| d.get_temp::<Pos2>(area_id))
            .unwrap_or(target_pos);

        let dt = ctx.input(|i| i.stable_dt).min(0.1);

        let win_state = window_anim_states
            .entry(title.to_string())
            .or_insert(WindowAnimState {
                actual_pos: target_pos,
                velocity: Vec2::ZERO,
            });

        if anim_progress < 0.01 {
            win_state.actual_pos = target_pos;
            win_state.velocity = Vec2::ZERO;
        } else {
            let stiffness = 280.0;
            let damping = 18.0;

            let substeps = 4;
            let sub_dt = dt / (substeps as f32);
            for _ in 0..substeps {
                let displacement = win_state.actual_pos - target_pos;
                let spring_force = -stiffness * displacement;
                let damping_force = -damping * win_state.velocity;
                let acceleration = spring_force + damping_force;

                win_state.velocity += acceleration * sub_dt;
                win_state.actual_pos += win_state.velocity * sub_dt;
            }
        }

        let y_offset_spawn = Vec2::new(0.0, 20.0 * (1.0 - anim_progress));
        let final_render_pos = win_state.actual_pos + y_offset_spawn;

        egui::Area::new(area_id)
            .current_pos(final_render_pos) // The whole UI draws here
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                ui.set_opacity(anim_progress);

                let frame = egui::Frame::window(&ctx.style())
                    .fill(Color32::from_rgb(22, 22, 22))
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(35, 35, 35)))
                    .rounding(egui::Rounding::ZERO)
                    .inner_margin(egui::Margin::symmetric(0.0, 0.0));

                frame.show(ui, |ui| {
                    ui.set_min_width(win_w);
                    ui.set_max_width(win_w);

                    ui.set_min_height(35.0);

                    // Title Bar
                    let (title_rect, title_resp) =
                        ui.allocate_exact_size(Vec2::new(win_w, 24.0), Sense::drag());

                    let is_drag_active = title_resp.dragged() || ui.ctx().is_being_dragged(title_resp.id);
                    
                    if is_drag_active {
                        target_pos += ui.ctx().input(|i| i.pointer.delta());
                        ctx.data_mut(|d| d.insert_temp(area_id, target_pos));
                    }

                    // Draw Title text
                    ui.painter().text(
                        title_rect.min + Vec2::new(8.0, 5.0),
                        Align2::LEFT_TOP,
                        title,
                        egui::FontId::proportional(14.0),
                        Color32::from_rgb(26, 171, 138), // Teal Accent for headers
                    );

                    // Separator line
                    ui.painter().line_segment(
                        [title_rect.left_bottom(), title_rect.right_bottom()],
                        egui::Stroke::new(1.0, Color32::from_rgb(35, 35, 35)),
                    );

                    let mut cat_modules: Vec<_> = client_modules_guard
                        .values()
                        .filter(|m| m.lock().unwrap().get_module_data().category == *category)
                        .collect();
                    cat_modules.sort_by(|a, b| {
                        a.lock()
                            .unwrap()
                            .get_module_data()
                            .name
                            .cmp(&b.lock().unwrap().get_module_data().name)
                    });

                    for module in cat_modules {
                        let (mod_name, is_enabled) = {
                            let lock = module.lock().unwrap();
                            let data = lock.get_module_data();
                            (data.name.clone(), data.enabled)
                        };

                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(win_w, 22.0), Sense::click());

                        let bg_color = if response.hovered() {
                            Color32::from_rgb(37, 37, 37)
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(rect, Rounding::ZERO, bg_color);

                        let text_color = if is_enabled {
                            Color32::from_rgb(26, 171, 138)
                        } else {
                            Color32::from_rgb(200, 200, 200)
                        };

                        ui.painter().text(
                            rect.min + egui::vec2(8.0, 4.0),
                            egui::Align2::LEFT_TOP,
                            &mod_name,
                            egui::FontId::proportional(14.0),
                            text_color,
                        );

                        let mut lock = module.lock().unwrap();
                        let is_expanded_id = Id::new(&mod_name).with("expanded");
                        let mut is_expanded =
                            ui.data(|d| d.get_temp::<bool>(is_expanded_id).unwrap_or(false));

                        let arrow_rect =
                            Rect::from_min_max(rect.max - Vec2::new(25.0, 20.0), rect.max);

                        if response.clicked() {
                            let click_pos = response.interact_pointer_pos().unwrap_or(Pos2::ZERO);
                            if response.clicked_by(egui::PointerButton::Secondary)
                                || (response.clicked_by(egui::PointerButton::Primary)
                                    && arrow_rect.contains(click_pos))
                            {
                                is_expanded = !is_expanded;
                                ui.data_mut(|d| d.insert_temp(is_expanded_id, is_expanded));
                            } else if response.clicked_by(egui::PointerButton::Primary) {
                                let new_state = !is_enabled;
                                lock.get_module_data_mut().set_enabled(new_state);
                                if new_state {
                                    let _ = lock.on_start();
                                } else {
                                    let _ = lock.on_stop();
                                }
                            }
                        }

                        let data = lock.get_module_data_mut();

                        if !data.settings.is_empty() {
                            let arrow = if is_expanded { "v" } else { ">" };
                            let arrow_color = if arrow_rect
                                .contains(ui.ctx().pointer_hover_pos().unwrap_or(Pos2::ZERO))
                            {
                                Color32::WHITE
                            } else {
                                Color32::GRAY
                            };

                            ui.painter().text(
                                rect.max - Vec2::new(15.0, 17.0),
                                egui::Align2::LEFT_TOP,
                                arrow,
                                egui::FontId::proportional(14.0),
                                arrow_color,
                            );

                            if is_expanded {
                                let settings_frame = egui::Frame::none()
                                    .fill(Color32::from_rgb(15, 15, 15))
                                    .inner_margin(egui::Margin::symmetric(8.0, 6.0));

                                settings_frame.show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        ui.style_mut().spacing.slider_width = 60.0;
                                        ui.style_mut().wrap_mode =
                                            Some(egui::TextWrapMode::Truncate);
                                        ui.style_mut().spacing.interact_size.x = 80.0;

                                        // 1. Static Keybind Row
                                        ui.horizontal(|ui| {
                                            ui.label("Bind");
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                let mut keybind_str = data.key_bind.to_string();
                                                let binding_id = ui.id().with(format!("{}_binding", mod_name));
                                                let is_binding = ui.data(|d| d.get_temp::<bool>(binding_id).unwrap_or(false));
                                                
                                                if is_binding {
                                                    keybind_str = "_".to_string();
                                                    // Consume any latest key press since they clicked "Bind"
                                                    let pressed = crate::graphic::input::LAST_KEY_PRESSED.swap(-1, std::sync::atomic::Ordering::Relaxed);
                                                    if pressed != -1 {
                                                        let new_key = crate::module::KeyboardKey::from(pressed);
                                                        
                                                        if new_key == crate::module::KeyboardKey::KeyEscape {
                                                            data.key_bind = crate::module::KeyboardKey::KeyNone; // Unbind
                                                            ui.data_mut(|d| d.insert_temp(binding_id, false));
                                                        } else {
                                                            // Check for duplicates
                                                            let mut is_duplicate = false;
                                                            let mut duplicate_name = String::new();
                                                            
                                                            for other_mod in client_modules_guard.values() {
                                                                if std::sync::Arc::ptr_eq(module, other_mod) {
                                                                    continue;
                                                                }
                                                                let other_lock = other_mod.lock().unwrap();
                                                                let other_data = other_lock.get_module_data();
                                                                if other_data.key_bind == new_key {
                                                                    is_duplicate = true;
                                                                    duplicate_name = other_data.name.clone();
                                                                    break;
                                                                }
                                                            }
                                                            
                                                            if is_duplicate {
                                                                crate::graphic::notification::Notification::send(
                                                                    crate::graphic::notification::NotificationType::Alert,
                                                                    "Keybind Conflict",
                                                                    &format!("Cheat '{}' already uses key '{}'", duplicate_name, new_key.to_string())
                                                                );
                                                            } else {
                                                                data.key_bind = new_key;
                                                                ui.data_mut(|d| d.insert_temp(binding_id, false));
                                                            }
                                                        }
                                                    }
                                                }
                                                
                                                // Small clean minimal button for the bind
                                                let btn = ui.add(egui::Button::new(
                                                    egui::RichText::new(&keybind_str).color(Color32::from_rgb(160, 160, 160))
                                                ).fill(Color32::from_rgb(25, 25, 25)).stroke(egui::Stroke::NONE));
                                                
                                                if btn.clicked() {
                                                    // Toggle binding mode
                                                    let new_state = !is_binding;
                                                    ui.data_mut(|d| d.insert_temp(binding_id, new_state));
                                                    if new_state {
                                                        // Flush any old presses
                                                        crate::graphic::input::LAST_KEY_PRESSED.store(-1, std::sync::atomic::Ordering::Relaxed);
                                                    }
                                                }
                                            });
                                        });

                                        for setting in &mut data.settings {
                                            match setting {
                                                ModuleSetting::Toggle { name, value } => {
                                                    ui.checkbox(value, name.as_str());
                                                }
                                                ModuleSetting::Slider {
                                                    name,
                                                    value,
                                                    min,
                                                    max,
                                                } => {
                                                    ui.vertical(|ui| {
                                                        ui.horizontal(|ui| {
                                                            ui.label(name.as_str());
                                                            ui.with_layout(
                                                                egui::Layout::right_to_left(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    ui.label(format!(
                                                                        "{:.2}",
                                                                        value
                                                                    ));
                                                                },
                                                            );
                                                        });
                                                        let available_w = ui.available_width();
                                                        ui.style_mut().spacing.slider_width =
                                                            available_w;
                                                        ui.add(
                                                            egui::Slider::new(
                                                                value,
                                                                min.clone()..=max.clone(),
                                                            )
                                                            .show_value(false)
                                                            .text(""),
                                                        );
                                                    });
                                                }
                                                ModuleSetting::Choice {
                                                    name,
                                                    value,
                                                    options,
                                                } => {
                                                    ui.vertical(|ui| {
                                                        ui.label(name.as_str());
                                                        egui::ComboBox::from_id_salt(name.as_str())
                                                            .width(ui.available_width())
                                                            .selected_text(
                                                                options
                                                                    .get(*value)
                                                                    .map(|s| s.as_str())
                                                                    .unwrap_or("??"),
                                                            )
                                                            .show_ui(ui, |ui| {
                                                                for (idx, opt) in
                                                                    options.iter().enumerate()
                                                                {
                                                                    ui.selectable_value(
                                                                        value,
                                                                        idx,
                                                                        opt.as_str(),
                                                                    );
                                                                }
                                                            });
                                                    });
                                                }
                                                ModuleSetting::Color { name, .. } => {
                                                    ui.label(format!(
                                                        "{}: [Color Settings soon]",
                                                        name
                                                    ));
                                                }
                                            }
                                        }
                                        ui.add_space(5.0);
                                    });
                                });
                            }
                        }
                    }
                });
            });

        curr_x += win_w + gap_x;
    }
}
