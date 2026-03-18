# Game Tracker

A lightweight Windows tool that tracks how long you play games each day and alerts you when you hit your self-imposed limits.

Built in Rust as a first Rust project.

---

## Features

- Tracks daily playtime per game by watching running processes
- Desktop alert when a time limit is reached
- Configurable reminder interval (how often to re-alert after the limit is hit)
- Per-game playtime reset
- Resets automatically at midnight each day
- Runs as two separate processes — the tracker works silently in the background, the UI is only needed when you want to change settings

---

## Architecture

```
tracker.exe          ui.exe
(background)    <--> (on demand)
     |                   |
     +---config.json-----+   UI writes, tracker reads
     |                   |
     +---playtime.json---+   Tracker writes, UI reads
```

The two processes communicate entirely through JSON files — no sockets, no shared memory. This means the tracker keeps running whether or not the UI is open.

---

## Requirements

- Windows 10 or 11
- [Rust](https://rustup.rs/) (to build from source)

---

## Building from Source

```powershell
git clone https://github.com/BenZHashim/rust-game-tracker.git
cd rust-game-tracker
cargo build --release
```

Binaries will be at `target/release/tracker.exe` and `target/release/ui.exe`.

---

## Installation

Run the install script once from PowerShell:

```powershell
.\install.ps1
```

This will:
- Build the release binaries
- Add `tracker.exe` to your Windows Startup folder so it runs automatically at login
- Create a `Game Tracker` shortcut on your Desktop to launch the UI

---

## Usage

**Tracker** — starts automatically at login. Runs silently with no window. You can verify it is running in Task Manager.

**UI** — open via the desktop shortcut. From here you can:
- See today's playtime for each tracked game
- Add games by their process name (e.g. `eldenring.exe`)
- Set a daily time limit per game (hours + minutes)
- Reset a game's playtime to zero
- Set how often you get reminded after hitting a limit

Game names are matched against running process names, so you can use a partial name (e.g. `eldenring` matches `eldenring.exe`).

---

## Updating

After pulling new changes, run:

```powershell
.\update.ps1
```

This rebuilds the release binaries and restarts the tracker. The desktop shortcut does not need to be updated.

---

## Uninstalling

```powershell
.\uninstall.ps1
```

This stops the tracker, removes the startup entry, and removes the desktop shortcut. Your `config.json` and `playtime.json` data files are left in place.

---

## License

MIT — see [LICENSE](LICENSE).
