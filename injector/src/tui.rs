use crate::platform;
use crate::platform::ProcessInfo;
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;

enum AppState {
    Menu,
    Selecting,
    Done(String),
    Error(String),
}

pub fn run_tui() {
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide).unwrap();

    println!("DarkClient Injector (TUI)");
    println!("Press 'f' to find the PID, 'i' to inject, 'q' to quit.");

    let mut state = AppState::Menu;
    let mut processes: Vec<ProcessInfo> = Vec::new();
    let mut selected_index = 0;

    loop {
        // Render Loop
        execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0)).unwrap();

        println!("=== DarkClient Injector (TUI) ===");

        match &state {
            AppState::Menu => {
                println!("Press 's' to scan for Minecraft processes.");
                println!("Press 'q' to quit.");
            }
            AppState::Selecting => {
                if processes.is_empty() {
                    println!("No processes found. Press 'r' to rescan or 'b' to back.");
                } else {
                    println!("Select a process using Up/Down arrows and Enter:");
                    for (i, proc) in processes.iter().enumerate() {
                        if i == selected_index {
                            execute!(stdout, SetForegroundColor(Color::Green)).unwrap();
                            print!("> ");
                        } else {
                            print!("  ");
                        }
                        println!("PID: {} | Info: {}", proc.pid, proc.info);
                        execute!(stdout, ResetColor).unwrap();
                    }
                }
            }
            AppState::Done(msg) => {
                execute!(stdout, SetForegroundColor(Color::Green)).unwrap();
                println!("SUCCESS: {}", msg);
                execute!(stdout, ResetColor).unwrap();
                println!("Press any key to return to menu.");
            }
            AppState::Error(err) => {
                execute!(stdout, SetForegroundColor(Color::Red)).unwrap();
                println!("ERROR: {}", err);
                execute!(stdout, ResetColor).unwrap();
                println!("Press any key to return to menu.");
            }
        }

        // Event Loop
        if event::poll(std::time::Duration::from_millis(100)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                match state {
                    AppState::Menu => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('s') => {
                            processes = platform::find_minecraft_processes();
                            selected_index = 0;
                            state = AppState::Selecting;
                        }
                        _ => {}
                    },
                    AppState::Selecting => match key.code {
                        KeyCode::Char('b') => state = AppState::Menu,
                        KeyCode::Char('r') => processes = platform::find_minecraft_processes(),
                        KeyCode::Up if selected_index > 0 => {
                            selected_index -= 1;
                        }
                        KeyCode::Down
                            if !processes.is_empty() && selected_index < processes.len() - 1 =>
                        {
                            selected_index += 1;
                        }
                        KeyCode::Enter if !processes.is_empty() => {
                            let pid = processes[selected_index].pid;

                            // Render the injection status.
                            execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0)).unwrap();
                            println!("Injecting into PID {}...", pid);

                            match crate::inject::inject(pid) {
                                Ok(_) => state = AppState::Done(format!("Injected into {}", pid)),
                                Err(e) => state = AppState::Error(e.to_string()),
                            }
                        }
                        _ => {}
                    },
                    AppState::Done(_) | AppState::Error(_) => {
                        state = AppState::Menu;
                    }
                }
            }
        }
    }

    execute!(stdout, LeaveAlternateScreen, cursor::Show).unwrap();
    println!("Exited TUI.");
}
