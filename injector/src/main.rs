mod platform;
mod tui;

use crate::platform::ProcessInfo;
use eframe::{CreationContext, Frame};
use egui::Context;
use log::LevelFilter;
use simplelog::{Config, WriteLogger};
use std::fs::File;

fn main() {
    if !is_elevated() {
        #[cfg(target_family = "unix")]
        eprintln!("❌ Please run this program with sudo: `sudo ./injector`");

        #[cfg(target_family = "windows")]
        eprintln!(
            "❌ Please run this program as Administrator (Right click → Run as administrator)"
        );

        return; // Exit the program if not elevated
    }

    // Initialize the logger with a default configuration
    WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create("app.log").unwrap(),
    )
    .unwrap();

    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--tui".to_string()) {
        tui::run_tui();
        return;
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([450.0, 320.0])
            .with_min_inner_size([300.0, 220.0]),
        ..Default::default()
    };

    eframe::run_native(
        "DarkClient Injector",
        native_options,
        Box::new(|creation_context| Ok(Box::new(InjectorGUI::new(creation_context)))),
    )
    .expect("Failed to run the GUI");
}

pub struct InjectorGUI {
    status: String,
    found_processes: Vec<ProcessInfo>,
    selected_pid: Option<u32>,
}

impl InjectorGUI {
    pub fn new(_creation_context: &CreationContext<'_>) -> Self {
        Self {
            status: "Ready:".to_owned(),
            found_processes: Vec::new(),
            selected_pid: None,
        }
    }

    fn scan(&mut self) {
        self.found_processes = platform::find_minecraft_processes();
        if self.found_processes.is_empty() {
            self.status = String::from("No Minecraft processes found.");
            self.selected_pid = None;
        } else {
            self.status = format!("Found {} processes.", self.found_processes.len());
            if self.selected_pid.is_none() {
                self.selected_pid = Some(self.found_processes[0].pid);
            }
        }
    }
}

impl eframe::App for InjectorGUI {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DarkClient Injector");

            ui.horizontal(|ui| {
                if ui.button("🔄 Scan").clicked() {
                    self.scan();
                }
                if !self.found_processes.is_empty() {
                    ui.label(format!("Found: {}", self.found_processes.len()));
                }
            });

            ui.add_space(10.0);

            if !self.found_processes.is_empty() {
                egui::ComboBox::from_id_salt("pid_select")
                    .width(300.0)
                    .selected_text(match self.selected_pid {
                        Some(pid) => {
                            let p = self.found_processes.iter().find(|p| p.pid == pid);
                            match p {
                                Some(proc) => format!("PID {}: {}", proc.pid, proc.info),
                                None => "Select process".to_string(),
                            }
                        }
                        None => "Select process".to_string(),
                    })
                    .show_ui(ui, |ui| {
                        for proc in &self.found_processes {
                            ui.selectable_value(
                                &mut self.selected_pid,
                                Some(proc.pid),
                                format!("PID {}: {}", proc.pid, proc.info),
                            );
                        }
                    });
            } else {
                ui.label("No processes found.");
            }

            ui.add_space(20.0);

            let btn = ui.add_enabled(self.selected_pid.is_some(), egui::Button::new("💉 INJECT"));
            if btn.clicked() {
                if let Some(pid) = self.selected_pid {
                    self.status = format!("Injecting into {}...", pid);
                    ctx.request_repaint();

                    match platform::inject(pid) {
                        Ok(_) => self.status = "✅ Injection Successful!".to_owned(),
                        Err(e) => self.status = format!("❌ Error: {}", e),
                    }
                }
            }

            ui.separator();
            ui.label(&self.status);
        });
    }
}

#[cfg(target_family = "unix")]
fn is_elevated() -> bool {
    extern "C" {
        fn geteuid() -> u32;
    }
    unsafe { geteuid() == 0 }
}

#[cfg(target_family = "windows")]
fn is_elevated() -> bool {
    is_elevated::is_elevated()
}
