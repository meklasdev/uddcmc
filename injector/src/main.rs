mod app;
mod inject;
mod platform;
mod tui;

use eframe::Frame;
use egui::Context;
use log::{error, LevelFilter};

use crate::app::InjectorApp;

fn main() {
    if !platform::is_elevated() {
        eprintln!("{}", elevation_hint());
        return;
    }

    if let Err(e) = protocol::init_file_logger("app.log", LevelFilter::Debug) {
        eprintln!("continuing without file logging: {e}");
    }

    if std::env::args().any(|arg| arg == "--tui") {
        tui::run_tui();
        return;
    }

    if let Err(e) = run_gui() {
        error!("GUI terminated with an error: {e}");
        eprintln!("Error: {e}");
    }
}

/// Launches the egui front-end.
fn run_gui() -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 340.0])
            .with_min_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "DarkClient Injector",
        native_options,
        Box::new(|_cc| Ok(Box::new(InjectorGui::default()))),
    )
}

/// Platform-specific hint shown when the injector lacks the privileges it
/// needs to attach to another process.
fn elevation_hint() -> &'static str {
    #[cfg(windows)]
    {
        "This program must run as Administrator (right click → Run as administrator)."
    }
    #[cfg(not(windows))]
    {
        "This program must run with root privileges: sudo ./injector"
    }
}

/// Thin egui wrapper around [`InjectorApp`]. The polished layout lands in a
/// dedicated `gui` module in the next refactor phase.
#[derive(Default)]
struct InjectorGui {
    app: InjectorApp,
}

impl eframe::App for InjectorGui {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.app.poll();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DarkClient Injector");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let scan = egui::Button::new("🔄 Scan");
                if ui.add_enabled(!self.app.is_busy(), scan).clicked() {
                    self.app.scan();
                }
                ui.label(format!("{} process(es)", self.app.processes().len()));
            });

            ui.add_space(8.0);
            self.process_picker(ui);
            ui.add_space(16.0);

            let can_inject = self.app.selected_pid().is_some() && !self.app.is_busy();
            if ui
                .add_enabled(can_inject, egui::Button::new("💉 Inject"))
                .clicked()
            {
                self.app.start_injection();
            }

            ui.separator();
            ui.label(self.app.status().message());
        });

        // Keep repainting while a worker thread is running so its result is
        // picked up promptly.
        if self.app.is_busy() {
            ctx.request_repaint();
        }
    }
}

impl InjectorGui {
    /// Renders the process selection combo box.
    fn process_picker(&mut self, ui: &mut egui::Ui) {
        let selected = self.app.selected_pid();
        let label = selected
            .and_then(|pid| self.app.processes().iter().find(|p| p.pid == pid))
            .map(|p| format!("PID {} — {}", p.pid, p.info))
            .unwrap_or_else(|| "Select a process".to_string());

        let mut picked = selected;
        egui::ComboBox::from_id_salt("process")
            .width(360.0)
            .selected_text(label)
            .show_ui(ui, |ui| {
                for proc in self.app.processes() {
                    ui.selectable_value(
                        &mut picked,
                        Some(proc.pid),
                        format!("PID {} — {}", proc.pid, proc.info),
                    );
                }
            });

        if let Some(pid) = picked {
            if Some(pid) != selected {
                self.app.select(pid);
            }
        }
    }
}
