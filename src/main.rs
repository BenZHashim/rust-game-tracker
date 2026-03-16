use std::thread;
use std::time::Duration;
use std::fmt::Write; // Required to use the write! macro on strings
use sysinfo::System;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::Path;
use std::collections::{HashMap, HashSet};
use notify_rust::Notification;
use chrono::Local;

// A constant for our filename so we don't misspell it
const SAVE_FILE: &str = "playtime.json";
const TIME_LIMIT_SECONDS: u64 = 15; 
const NOTIFICATION_COOLDOWN_SECONDS: u64 = 30;

#[derive(Serialize, Deserialize)]
struct TrackerState {
    date: String,
    times: HashMap<String, u64>,
}

fn get_today_string() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn load_state() -> TrackerState {
    let today = get_today_string();

    if Path::new(SAVE_FILE).exists() {
        let json_str = fs::read_to_string(SAVE_FILE).unwrap_or_default();
        
        if let Ok(data) = serde_json::from_str::<TrackerState>(&json_str) {
            // If the saved file is from today, load it normally
            if data.date == today {
                return data;
            }
            // If it's from yesterday, we just fall through and return a fresh state
            println!("New day detected! Resetting timers for {}.", today);
        }
    }
    
    // Return a fresh slate
    TrackerState {
        date: today,
        times: HashMap::new(),
    }
}

fn save_state(state: &TrackerState) {
    let json_str = serde_json::to_string_pretty(state).unwrap();
    fs::write(SAVE_FILE, json_str).unwrap();
}

fn handle_daily_reset(state: &mut TrackerState, last_notified_time: &mut HashMap<String, u64>) {
    let today = get_today_string();
    
    // Check if the date in our state matches the actual calendar date
    if state.date != today {
        println!("New day detected: {}. Resetting all data.", today);
        
        // 1. Update the state's internal date
        state.date = today;
        
        // 2. Clear the playtime data
        state.times.clear();
        
        // 3. Reset notification cooldowns so the user can be warned today
        last_notified_time.clear();
        
        // 4. Immediately overwrite the file with the empty state
        save_state(state);
    }
}

fn handle_notifications(
    game: &str, 
    current_time: u64, 
    last_notified_time: &mut HashMap<String, u64>
) {
    if current_time >= TIME_LIMIT_SECONDS {
        let last_nag = *last_notified_time.get(game).unwrap_or(&0);
        
        if last_nag == 0 || (current_time - last_nag) >= NOTIFICATION_COOLDOWN_SECONDS {
            
            println!("Triggering desktop notification for {}!", game);
            
            // Fire the notification
            Notification::new()
                .summary("Time Limit Reached!")
                .body(&format!("You're over your limit for {}. Time to wrap up!", game))
                .appname("Game Tracker")
                .show()
                .unwrap();

            // Update the dictionary with the new nag time
            last_notified_time.insert(game.to_string(), current_time);
        }
    }
}

// We change the function to take a MUTABLE REFERENCE to our pre-allocated string.
// It doesn't return anything anymore; it just modifies the buffer in place.
fn format_time_into(total_seconds: u64, buffer: &mut String) {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    // 1. Clear the string's length back to 0 (this does NOT free the heap memory)
    buffer.clear(); 

    // write! returns a Result (in case writing fails), so we call .unwrap() 
    write!(buffer, "{:02}:{:02}:{:02}", hours, minutes, seconds).unwrap();
}

fn main() {
    let mut sys = System::new_all();
    
    // Define an array of targets to watch for
    let target_games = ["stellaris", "deadcells", "claude", "chrome"]; 
    let poll_interval = 5; 

    // Load the master ledger into memory
    let mut state: TrackerState = load_state();
    let mut last_notified_time: HashMap<String, u64> = HashMap::new(); // Track last notification times
    
    // We can reuse a single buffer for all string formatting
    let mut time_buffer = String::with_capacity(8);

    println!("Starting tracker... looking for: {:?}", target_games);

    loop {
        // 0. RESET: Check if we're on a new day
        handle_daily_reset(&mut state, &mut last_notified_time);

        sys.refresh_processes();
        
        // A fresh set for this specific 5-second tick
        let mut running_this_tick = HashSet::new();

        // 1. SCAN: Find all running targets
        for (_pid, process) in sys.processes() {
            let process_name = process.name().to_lowercase();
            
            for &game in &target_games {
                if process_name.contains(game) {
                    // HashSet automatically ignores duplicates.
                    // If we find 10 "chrome" processes, it only gets added once.
                    running_this_tick.insert(game);
                }
            }
        }

        // 2. UPDATE: Add time to the running games
        let mut data_changed = false;

        for game in running_this_tick {
            // The .entry() API is a brilliant Rust feature. 
            // It gets the value if it exists, or inserts 0 if it doesn't, 
            // and then returns a mutable reference so we can add to it.
            let current_time = state.times.entry(game.to_string()).or_insert(0); 
            
            *current_time += poll_interval;
            data_changed = true;

            // Format and print
            format_time_into(*current_time, &mut time_buffer);
            println!("{} is running! Total time: {}", game, time_buffer);

            // Time limit warning!
            println!("DEBUG -> Time: {} | Limit: {}", *current_time, TIME_LIMIT_SECONDS);
            handle_notifications(game, *current_time, &mut last_notified_time);
        }

        // 3. SAVE: Only write to the disk if a game was actually running
        if data_changed {
            save_state(&state);
        }

        thread::sleep(Duration::from_secs(poll_interval));
    }
}