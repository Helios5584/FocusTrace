# FocusTrace

macOS menu-bar app that logs every application focus change with timestamp, app name, window title, and previous app. Local SQLite storage, no network.

## Features

- Logs `NSWorkspaceDidActivateApplication` events to SQLite
- Window title capture via Accessibility API (per-PID `AXFocusedWindow` -> `AXTitle`)
- egui table UI: search (scoped: All / Time / App / Title / Previous), tri-state column sort
- Menu-bar tray icon; close window hides instead of quits
- Autostart at login via LaunchAgent
- Settings tab with live Accessibility-permission status and instructions

## Requirements

- macOS 11+ (Apple Silicon)
- Rust stable, `aarch64-apple-darwin` target
- Accessibility permission granted to the built `.app` (window titles only)

## Build

```sh
cargo build --release
```

## Bundle `.app`

```sh
./scripts/bundle.sh
```

Outputs `dist/FocusTrace.app`. Ad-hoc signed with identifier `com.focustrace.app`. Persistent Accessibility trust across rebuilds requires Developer ID signing; ad-hoc rebuilds may need re-granting.

## Permissions

First launch prompts for Accessibility. Without it: App + Transition columns still log; Window Title stays blank for non-FocusTrace apps. Grant via System Settings -> Privacy & Security -> Accessibility, then relaunch (trust read once at process start).

## Data

- SQLite: `~/Library/Application Support/FocusTrace/focus.sqlite`
- Settings JSON: same dir
- LaunchAgent plist: `~/Library/LaunchAgents/com.focustrace.app.plist` (when autostart enabled)

Schema:

```sql
CREATE TABLE focus_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts TEXT NOT NULL,
    app_name TEXT NOT NULL,
    bundle_id TEXT NOT NULL,
    window_title TEXT NOT NULL,
    previous_app TEXT NOT NULL
);
```

## Layout

- [src/main.rs](src/main.rs) — entry, eframe bootstrap
- [src/focus.rs](src/focus.rs) — `NSWorkspace` activation observer
- [src/ax.rs](src/ax.rs) — Accessibility API window-title lookup
- [src/db.rs](src/db.rs) — SQLite open/insert/load
- [src/ui.rs](src/ui.rs) — egui app, Logs + Settings tabs
- [src/tray.rs](src/tray.rs) — menu-bar icon
- [src/autostart.rs](src/autostart.rs) — LaunchAgent toggle
- [src/settings.rs](src/settings.rs) — persisted prefs
- [macos/Info.plist](macos/Info.plist) — `LSUIElement=true` (no Dock icon)
- [scripts/bundle.sh](scripts/bundle.sh) — release build + `.app` assembly

## License

Not specified.
