use crate::client::DarkClient;
use crate::module::{ModuleCategory, ModuleSetting};
use egui::{Align2, Color32, Context, Id, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

pub fn draw(ctx: &Context, anim_progress: f32) {
    // 1. Sfondo scuro semitrasparente
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

    // 2. Pulsanti Globali (In alto a destra)
    egui::Area::new(Id::new("global_buttons"))
        .anchor(Align2::RIGHT_TOP, Vec2::new(-10.0, 10.0))
        .show(ctx, |ui| {
            ui.set_opacity(anim_progress);
            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("PANIC").color(Color32::RED)).clicked() {
                    std::thread::spawn(|| crate::graphic::ui_manager::call_panic());
                }
                if ui.button("Reset UI").clicked() {
                    // Egui salva le posizioni in memoria. Per resettarle:
                    ctx.memory_mut(|mem| mem.reset_areas());
                }
            });
        });

    // 3. Finestre per ogni Categoria
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
        // Se la finestra successiva sfora lo schermo, scendiamo di una "riga"
        if curr_x + win_w > logical_width && curr_x > 50.0 {
            curr_x = 50.0;
            curr_y += row_height; // Altezza stimata per evitare che si tocchino scendendo
        }
        let title = category.display_name();

        // Offset iniziale animato
        let y_offset = 20.0 * (1.0 - anim_progress);
        let start_pos = Pos2::new(curr_x, curr_y + y_offset);

        egui::Window::new(title)
            .id(Id::new(title))
            .default_pos(start_pos)
            .default_width(win_w)
            .min_width(win_w)
            .max_width(win_w)
            .resizable(false)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.set_opacity(anim_progress);

                // Filtriamo i moduli per questa finestra
                let mut cat_modules: Vec<_> = client_modules_guard
                    .values()
                    .filter(|m| m.lock().unwrap().get_module_data().category == *category)
                    .collect();
                cat_modules.sort_by(|a, b| {
                    a.lock().unwrap().get_module_data().name.cmp(&b.lock().unwrap().get_module_data().name)
                });

                for module in cat_modules {
                    let (mod_name, is_enabled) = {
                        let mut lock = module.lock().unwrap();
                        let data = lock.get_module_data_mut();
                        let mod_name = data.name.clone();
                        let is_enabled = data.enabled;
                        (mod_name, is_enabled)
                    };

                    // Creiamo una riga custom per gestire click SINISTRO (Toggle) e DESTRO (Espandi)
                    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 20.0), Sense::click());

                    // Gestione Colori Background in base a hover
                    let bg_color = if response.hovered() {
                        Color32::from_rgb(64, 64, 64)
                    } else {
                        Color32::from_rgb(38, 38, 38)
                    };
                    ui.painter().rect_filled(rect, Rounding::same(4.0), bg_color);

                    // Testo Modulo
                    let text_color = if is_enabled { Color32::YELLOW } else { Color32::WHITE };
                    let text_pos = rect.min + Vec2::new(5.0, 3.0);
                    ui.painter().text(
                        text_pos,
                        egui::Align2::LEFT_TOP,
                        &mod_name,
                        egui::FontId::proportional(14.0),
                        text_color,
                    );

                    let mut lock = module.lock().unwrap();

                    let is_expanded_id = Id::new(&mod_name).with("expanded");
                    let mut is_expanded = ui.data(|d| d.get_temp::<bool>(is_expanded_id).unwrap_or(false));

                    // Definiamo un'area immaginaria di 25x20 pixel sull'estrema destra del rettangolo
                    let arrow_rect = Rect::from_min_max(
                        rect.max - Vec2::new(25.0, 20.0),
                        rect.max,
                    );

                    // Gestione avanzata del Click
                    if response.clicked() {
                        // Prendiamo le coordinate esatte del click
                        let click_pos = response.interact_pointer_pos().unwrap_or(Pos2::ZERO);

                        // CASO 1: Click Destro, OPPURE Click Sinistro proprio sopra la freccetta
                        if response.clicked_by(egui::PointerButton::Secondary) ||
                            (response.clicked_by(egui::PointerButton::Primary) && arrow_rect.contains(click_pos)) {

                            is_expanded = !is_expanded;
                            ui.data_mut(|d| d.insert_temp(is_expanded_id, is_expanded));

                        // CASO 2: Click Sinistro sul resto del corpo del bottone
                        } else if response.clicked_by(egui::PointerButton::Primary) {
                            let new_state = !is_enabled;
                            lock.get_module_data_mut().set_enabled(new_state);
                            if new_state { let _ = lock.on_start(); } else { let _ = lock.on_stop(); }
                        }
                    }

                    let data = lock.get_module_data_mut();

                    // Se ha settaggi, gestiamo l'espansione
                    if !data.settings.is_empty() {
                        let arrow = if is_expanded { "v" } else { ">" };

                        // Per feedback visivo, se il mouse è esattamente sopra l'area della freccia, la illuminiamo
                        let arrow_color = if arrow_rect.contains(ui.ctx().pointer_hover_pos().unwrap_or(Pos2::ZERO)) {
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

                        // Se è espanso, mostriamo i settaggi usando i widget nativi di egui
                        if is_expanded {
                            ui.horizontal(|ui| {
                                ui.add_space(10.0); // Indentazione
                                ui.vertical(|ui| {
                                    // 1. Diciamo agli slider di essere larghi solo 60 pixel
                                    ui.style_mut().spacing.slider_width = 60.0;

                                    // 2. Se un testo è troppo lungo, lo tagliamo coi puntini (...) anziché allargare la tab
                                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);

                                    // 3. Assicuriamoci che i combobox non esplodano in larghezza
                                    ui.style_mut().spacing.interact_size.x = 80.0;

                                    for setting in &mut data.settings {
                                        match setting {
                                            ModuleSetting::Toggle { name, value } => {
                                                ui.checkbox(value, name.as_str());
                                            }
                                            ModuleSetting::Slider { name, value, min, max } => {
                                                ui.vertical(|ui| {
                                                    // Riga 1: Nome a sinistra, Valore a destra
                                                    ui.horizontal(|ui| {
                                                        ui.label(name.as_str());

                                                        // Spinge il valore tutto a destra
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            // Formattiamo a 2 decimali per evitare numeri infiniti
                                                            ui.label(format!("{:.2}", value));
                                                        });
                                                    });

                                                    // Riga 2: Lo slider vero e proprio
                                                    // Diciamo allo slider di prendere tutta la larghezza rimasta e nascondiamo
                                                    // il numerino di default (visto che l'abbiamo appena disegnato noi sopra)
                                                    let available_w = ui.available_width();
                                                    ui.style_mut().spacing.slider_width = available_w;

                                                    ui.add(
                                                        egui::Slider::new(value, min.clone()..=max.clone())
                                                            .show_value(false) // Nasconde il testo del valore integrato
                                                            .text("")          // Rimuove la label integrata
                                                    );
                                                });
                                            }
                                            ModuleSetting::Choice { name, value, options } => {
                                                ui.vertical(|ui| {
                                                    ui.label(name.as_str()); // Nome sopra

                                                    // Combobox largo tutto lo spazio sotto
                                                    egui::ComboBox::from_id_salt(name.as_str()) // id_source invece di label per non mettere il testo a lato
                                                        .width(ui.available_width())
                                                        .selected_text(options.get(*value).map(|s| s.as_str()).unwrap_or("??"))
                                                        .show_ui(ui, |ui| {
                                                            for (idx, opt) in options.iter().enumerate() {
                                                                ui.selectable_value(value, idx, opt.as_str());
                                                            }
                                                        });
                                                });
                                            }
                                            ModuleSetting::Color { name, .. } => {
                                                ui.label(format!("{}: [Color Settings soon]", name));
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

        // Avanziamo verso destra per la prossima categoria
        curr_x += win_w + gap_x;
    }
}