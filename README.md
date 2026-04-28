# FocusTrace

macOS menu-bar app that logs every application focus change to a local SQLite database. Timestamp, app name, bundle id, window title, previous app. No network.

## Features

- `NSWorkspaceDidActivateApplication` observer → SQLite
- Window title via Accessibility API (`AXFocusedWindow` → `AXTitle`)
- egui table: scoped search (All / Time / App / Title / Previous), tri-state column sort
- Menu-bar tray with recent-activity preview, pause, clear, quit
- Settings tab (scrollable): autostart at login, hide menu-bar icon, start minimized, live Accessibility-permission status
- Window close keeps the process running; relaunching the .app re-shows the UI

## Requirements

- macOS 11+ Apple Silicon
- Rust stable + `aarch64-apple-darwin`
- Accessibility permission (window titles only)

## Build

```sh
cargo build --release          # binary
./scripts/bundle.sh            # dist/FocusTrace.app (ad-hoc signed)
```

Persistent Accessibility trust across rebuilds needs Developer ID signing. Ad-hoc rebuilds may require re-granting permission.

## Permissions

First launch prompts for Accessibility. Without it the App and Transition columns still log; Window Title stays blank for non-FocusTrace apps. Grant via *System Settings → Privacy & Security → Accessibility*, then relaunch (trust is read once at startup).

## Files

- DB: `~/Library/Application Support/FocusTrace/focus.sqlite`
- Settings: `~/Library/Application Support/FocusTrace/settings.json`
- LaunchAgent: `~/Library/LaunchAgents/com.focustrace.agent.plist` (when autostart on)

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

## License

Not specified.
