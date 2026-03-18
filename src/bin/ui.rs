// Hide the console window so only the egui window appears.
#![windows_subsystem = "windows"]

// ui.rs — the GUI process for managing games and viewing playtime.
//
// This process does NOT run the tracker. It is purely a viewer/editor.
// It communicates with the tracker through two files:
//   config.json    — this process WRITES (the tracker reads it)
//   playtime.json  — this process READS (the tracker writes it)
//
// Because these are separate processes with no shared memory, the UI
// simply reloads both files from disk every 5 seconds to stay current.

use eframe::egui;
use game_tracker::{
    load_config, load_state, save_config, save_state,
    format_time, Config, GameConfig, TrackerState,
};
use std::time::{Duration, Instant};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 520.0])
            .with_title("Game Tracker"),
        ..Default::default()
    };

    eframe::run_native(
        "Game Tracker",
        options,
        // This closure is the "app factory" — eframe calls it once to create
        // our app. The `Ok(Box::new(...))` wraps it in a Result and a heap
        // allocation, which is what the eframe API requires.
        Box::new(|_cc| Ok(Box::new(GameTrackerApp::new()))),
    )
}

// `GameTrackerApp` holds everything the UI needs between frames.
// With separate processes, this is just plain data — no Mutex needed.
struct GameTrackerApp {
    config: Config,
    playtime: TrackerState,
    last_refresh: Instant,     // when we last reloaded from disk
    new_game_name: String,
    new_game_limit_hours: u32,
    new_game_limit_minutes: u32,
}

impl GameTrackerApp {
    fn new() -> Self {
        Self {
            config: load_config(),
            playtime: load_state(),
            // `Instant::now()` is a monotonic clock — good for measuring
            // elapsed time. We use it to know when to refresh from disk.
            last_refresh: Instant::now(),
            new_game_name: String::new(),
            new_game_limit_hours: 2,
            new_game_limit_minutes: 0,
        }
    }

    // Reload both files from disk if 5 seconds have passed.
    fn maybe_refresh(&mut self) {
        if self.last_refresh.elapsed() >= Duration::from_secs(5) {
            self.config = load_config();
            self.playtime = load_state();
            self.last_refresh = Instant::now();
        }
    }
}

impl eframe::App for GameTrackerApp {
    // `update` is called by egui every frame. We keep it cheap:
    // most frames just draw the UI; disk reads happen only every 5 seconds.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Schedule a repaint in 5 seconds so the display stays current
        // even when the user isn't interacting with the window.
        ctx.request_repaint_after(Duration::from_secs(5));

        self.maybe_refresh();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Game Tracker");
            ui.label(format!("Tracking for: {}", self.playtime.date));
            ui.separator();

            ui.label(egui::RichText::new("Tracked Games").strong());
            ui.add_space(4.0);

            let n = self.config.games.len();
            let mut to_remove: Option<usize> = None;
            let mut to_reset: Option<usize> = None;
            let mut config_changed = false;

            if n == 0 {
                ui.label("No games tracked yet. Add one below.");
            } else {
                // 5-column grid: Name | Time Today | Limit h | Limit m | Remove
                //
                // We use separate columns for hours and minutes rather than a
                // nested ui.horizontal() to keep the borrow checker happy —
                // a nested closure would need two simultaneous mutable borrows
                // of the same data.
                egui::Grid::new("games_grid")
                    .num_columns(5)
                    .striped(true)
                    .min_col_width(60.0)
                    .show(ui, |ui| {
                        ui.strong("Game");
                        ui.strong("Time Today");
                        ui.strong("Limit h");
                        ui.strong("Limit m");
                        ui.strong("");
                        ui.end_row();

                        for i in 0..n {
                            let name  = &self.config.games[i].name;
                            let time  = self.playtime.times.get(name).copied().unwrap_or(0);
                            let limit = self.config.games[i].limit_seconds;
                            let over  = limit > 0 && time >= limit;

                            if over {
                                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), name);
                            } else {
                                ui.label(name);
                            }

                            ui.label(format_time(time));

                            // `lh` and `lm` are local copies. DragValue mutates them,
                            // and if they changed we write back to the config.
                            let mut lh = (limit / 3600) as u32;
                            let mut lm = ((limit % 3600) / 60) as u32;

                            if ui.add(egui::DragValue::new(&mut lh).range(0u32..=24u32).suffix("h")).changed() {
                                self.config.games[i].limit_seconds = lh as u64 * 3600 + lm as u64 * 60;
                                config_changed = true;
                            }
                            if ui.add(egui::DragValue::new(&mut lm).range(0u32..=59u32).suffix("m")).changed() {
                                self.config.games[i].limit_seconds = lh as u64 * 3600 + lm as u64 * 60;
                                config_changed = true;
                            }

                            // Both action buttons share a cell via ui.horizontal.
                            // The nested closure captures only `to_remove` and
                            // `to_reset` — local Option variables, not `self` —
                            // so there's no borrow conflict.
                            ui.horizontal(|ui| {
                                if ui.small_button("Reset").clicked() {
                                    to_reset = Some(i);
                                }
                                if ui.small_button("Remove").clicked() {
                                    to_remove = Some(i);
                                }
                            });

                            ui.end_row();
                        }
                    });
            }

            // We can't remove from a Vec while iterating over it, so we
            // record the index and remove it here after the loop.
            if let Some(idx) = to_remove {
                self.config.games.remove(idx);
                config_changed = true;
            }
            if config_changed {
                save_config(&self.config);
            }

            // Zero out a single game's playtime and write it to disk.
            // The tracker will pick up the new value on its next tick.
            if let Some(idx) = to_reset {
                let name = self.config.games[idx].name.clone();
                self.playtime.times.insert(name, 0);
                save_state(&self.playtime);
            }

            ui.separator();
            ui.separator();
            ui.label(egui::RichText::new("Settings").strong());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Reminder interval:");
                if ui.add(egui::DragValue::new(&mut self.config.reminder_interval_mins)
                    .range(1u64..=120u64)
                    .suffix(" min"))
                    .changed()
                {
                    save_config(&self.config);
                }
            });

            ui.add_space(8.0);
            ui.label(egui::RichText::new("Add Game").strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Process name:");
                ui.text_edit_singleline(&mut self.new_game_name);
            });

            ui.horizontal(|ui| {
                ui.label("Daily limit:");
                ui.add(egui::DragValue::new(&mut self.new_game_limit_hours).range(0u32..=24u32).suffix("h"));
                ui.add(egui::DragValue::new(&mut self.new_game_limit_minutes).range(0u32..=59u32).suffix("m"));
            });

            let trimmed = self.new_game_name.trim().to_lowercase();
            let duplicate = self.config.games.iter().any(|g| g.name == trimmed);

            ui.add_enabled_ui(!trimmed.is_empty() && !duplicate, |ui| {
                if ui.button("Add Game").clicked() {
                    let limit = self.new_game_limit_hours as u64 * 3600
                        + self.new_game_limit_minutes as u64 * 60;
                    self.config.games.push(GameConfig { name: trimmed.clone(), limit_seconds: limit });
                    save_config(&self.config);
                    self.new_game_name.clear();
                }
            });

            if duplicate && !self.new_game_name.trim().is_empty() {
                ui.colored_label(egui::Color32::YELLOW, "Already being tracked.");
            }
        });
    }
}
