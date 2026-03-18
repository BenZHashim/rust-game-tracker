// Hide the console window so the tracker runs silently in the background.
#![windows_subsystem = "windows"]

// tracker.rs — the standalone background tracker process.
//
// This binary has no GUI. It runs in a loop, watches for game processes,
// and sends desktop notifications when limits are hit.
//
// It communicates with the UI through two files:
//   playtime.json  — this process WRITES (the UI reads it)
//   config.json    — this process READS (the UI writes it)
//
// `use game_tracker::*` imports everything marked `pub` from src/lib.rs.
// The crate name "game_tracker" comes from the `name` field in Cargo.toml.

use game_tracker::{
    load_config, load_state, save_state,
    get_today_string, format_time,
};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::thread;
use std::time::Duration;
use winapi::shared::minwindef::{LPARAM, LRESULT, TRUE, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::processthreadsapi::GetCurrentThreadId;
use winapi::um::winuser::{
    AttachThreadInput, BringWindowToTop, CallNextHookEx, GetForegroundWindow,
    GetWindowThreadProcessId, MessageBoxW, SetForegroundWindow, SetWindowPos,
    SetWindowsHookExW, UnhookWindowsHookEx,
    HCBT_ACTIVATE, HWND_TOPMOST, MB_ICONWARNING, MB_OK, MB_TOPMOST,
    SWP_NOMOVE, SWP_NOSIZE, WH_CBT,
};

// Windows API functions expect UTF-16 null-terminated strings, not Rust's UTF-8.
// This helper converts a &str into the Vec<u16> format Windows wants.
fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn cbt_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if code == HCBT_ACTIVATE as i32 {
            let hwnd = wparam as HWND;

            // Windows' foreground lock normally stops background processes from
            // stealing focus. Temporarily attaching our input queue to the
            // foreground thread's queue grants us an exemption.
            let fg_hwnd   = GetForegroundWindow();
            let fg_thread = GetWindowThreadProcessId(fg_hwnd, std::ptr::null_mut());
            let my_thread = GetCurrentThreadId();

            if fg_thread != my_thread {
                AttachThreadInput(fg_thread, my_thread, TRUE);
            }

            SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            SetForegroundWindow(hwnd);
            BringWindowToTop(hwnd);

            if fg_thread != my_thread {
                AttachThreadInput(fg_thread, my_thread, 0); // 0 = FALSE, detach
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
    }
}

fn show_alert(title: &str, body: &str) {
    let title_w = to_wide(title);
    let body_w  = to_wide(body);
    unsafe {
        // Install the hook on this thread only, then show the dialog.
        // The hook fires before the window is visible, guaranteeing it's
        // already topmost by the time it appears on screen.
        let hook = SetWindowsHookExW(
            WH_CBT,
            Some(cbt_hook),
            std::ptr::null_mut(),
            GetCurrentThreadId(),
        );
        MessageBoxW(
            std::ptr::null_mut(),
            body_w.as_ptr(),
            title_w.as_ptr(),
            MB_OK | MB_TOPMOST | MB_ICONWARNING,
        );
        if !hook.is_null() {
            UnhookWindowsHookEx(hook);
        }
    }
}
use sysinfo::System;

fn main() {
    let mut sys = System::new_all();
    let poll_interval = 5u64;

    let mut last_notified_time: HashMap<String, u64> = HashMap::new();

    println!("Tracker started. Watching config.json for games...");

    loop {
        thread::sleep(Duration::from_secs(poll_interval));

        let config = load_config();
        // Reload from disk every tick so changes from the UI (like a per-game
        // reset) are respected rather than overwritten by our in-memory copy.
        let mut state = load_state();

        sys.refresh_processes();

        let today = get_today_string();
        if state.date != today {
            println!("New day: {}. Resetting playtime.", today);
            last_notified_time.clear();
        }

        let mut running: HashSet<String> = HashSet::new();
        for (_pid, process) in sys.processes() {
            let pname = process.name().to_lowercase();
            for game in &config.games {
                if pname.contains(&game.name) {
                    running.insert(game.name.clone());
                }
            }
        }


        let mut data_changed = false;
        for game_name in &running {

            let current_time = state.times.entry(game_name.clone()).or_insert(0);
            *current_time += poll_interval;
            data_changed = true;


            if let Some(game_cfg) = config.games.iter().find(|g| &g.name == game_name) {
                let ct = *current_time;
                let limit = game_cfg.limit_seconds;

                if limit > 0 && ct >= limit {
                    let last_nag = *last_notified_time.get(game_name.as_str()).unwrap_or(&0);
                    let cooldown = config.reminder_interval_mins * 60;
                    if last_nag == 0 || ct.saturating_sub(last_nag) >= cooldown {
                        println!(
                            "{} has hit its limit ({})! Sending notification.",
                            game_name,
                            format_time(limit)
                        );
                        let body = format!(
                            "You've hit your limit for {}. Time to wrap up!",
                            game_name
                        );
                        // Spawn a thread so the blocking dialog doesn't
                        // freeze the tracker loop while waiting for a click.
                        thread::spawn(move || {
                            show_alert("Time Limit Reached!", &body);
                        });
                        last_notified_time.insert(game_name.clone(), ct);
                    }
                }
            }
        }


        if data_changed {
            save_state(&state);
        }

        if !running.is_empty() {
            for game_name in &running {
                let total = state.times.get(game_name).copied().unwrap_or(0);
                println!("{}: {}", game_name, format_time(total));
            }
        }
    }
}
