// lib.rs — shared types and file I/O used by both the tracker and UI binaries.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use chrono::Local;

fn default_reminder_interval_mins() -> u64 { 1 }

// Returns the directory that contains the running executable.
pub fn data_dir() -> PathBuf {
    std::env::current_exe()
        .expect("cannot resolve executable path")
        .parent()
        .expect("executable has no parent directory")
        .to_path_buf()
}

pub fn save_file_path() -> PathBuf { data_dir().join("playtime.json") }
pub fn config_file_path() -> PathBuf { data_dir().join("config.json") }



#[derive(Serialize, Deserialize, Clone)]
pub struct GameConfig {
    pub name: String,
    pub limit_seconds: u64,
}


#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub games: Vec<GameConfig>,
    #[serde(default = "default_reminder_interval_mins")]
    pub reminder_interval_mins: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            games: vec![],
            reminder_interval_mins: 1,
        }
    }
}

// Today's playtime ledger written by the tracker and read by the UI.
#[derive(Serialize, Deserialize, Clone)]
pub struct TrackerState {
    pub date: String,
    pub times: HashMap<String, u64>,
}



pub fn get_today_string() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

// Format a number of seconds as HH:MM:SS.
pub fn format_time(total_seconds: u64) -> String {
    let hours   = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}


pub fn load_state() -> TrackerState {
    let today = get_today_string();
    let path = save_file_path();

    if path.exists() {
        let json_str = fs::read_to_string(&path).unwrap_or_default();
        if let Ok(data) = serde_json::from_str::<TrackerState>(&json_str) {
            if data.date == today {
                return data;
            }
            // File is from a previous day — fall through to return a fresh state.
        }
    }

    TrackerState { date: today, times: HashMap::new() }
}

pub fn save_state(state: &TrackerState) {
    let json_str = serde_json::to_string_pretty(state).unwrap();
    fs::write(save_file_path(), json_str).unwrap();
}

pub fn load_config() -> Config {
    let path = config_file_path();

    if path.exists() {
        let json_str = fs::read_to_string(&path).unwrap_or_default();
        if let Ok(config) = serde_json::from_str::<Config>(&json_str) {
            return config;
        }
    }
    Config::default()
}

pub fn save_config(config: &Config) {
    let json_str = serde_json::to_string_pretty(config).unwrap();
    fs::write(config_file_path(), json_str).unwrap();
}
