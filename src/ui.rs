use crate::autostart;
use crate::db::{Db, FocusEvent};
use crate::settings::{SearchScope, Settings};
use crate::tray::{poll_menu_events, MenuAction, TrayHandle};
use crossbeam_channel::Receiver;
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortKey { Time, App, Title, Prev }

#[derive(PartialEq, Eq)]
enum Tab { Logs, Settings }

fn header_cell(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    let avail = ui.available_size();
    let text = if active {
        egui::RichText::new(label)
            .strong()
            .color(egui::Color32::WHITE)
    } else {
        egui::RichText::new(label).strong()
    };
    let mut button = egui::Button::new(text);
    if active {
        button = button.fill(egui::Color32::from_rgb(60, 110, 180));
    } else {
        button = button.fill(egui::Color32::TRANSPARENT);
    }
    ui.add_sized(avail, button).clicked()
}

fn scope_label(s: SearchScope) -> &'static str {
    match s {
        SearchScope::All => "All columns",
        SearchScope::Time => "Time",
        SearchScope::App => "App",
        SearchScope::Title => "Window Title",
        SearchScope::Prev => "Previous App",
    }
}

pub struct App {
    db: Arc<Db>,
    rx: Receiver<FocusEvent>,
    reopen_rx: Receiver<()>,
    events: Vec<FocusEvent>,
    search: String,
    search_scope: SearchScope,
    sort: Option<(SortKey, bool)>,
    tab: Tab,
    settings: Settings,
    tray: TrayHandle,
    quitting: bool,
    paused: bool,
    window_hidden: bool,
}

impl App {
    pub fn new(
        db: Arc<Db>,
        rx: Receiver<FocusEvent>,
        reopen_rx: Receiver<()>,
        tray: TrayHandle,
    ) -> Self {
        let events = db.load_all().unwrap_or_default();
        let settings = Settings::load();
        tray.refresh_recent(&events);
        tray.set_visible(settings.show_tray);
        Self {
            db,
            rx,
            reopen_rx,
            events,
            search: String::new(),
            search_scope: settings.search_scope,
            sort: None,
            tab: Tab::Logs,
            settings,
            tray,
            quitting: false,
            paused: false,
            window_hidden: false,
        }
    }

    fn drain_incoming(&mut self) -> bool {
        let mut any = false;
        while let Ok(ev) = self.rx.try_recv() {
            if self.paused {
                continue;
            }
            let mut ev = ev;
            if let Ok(id) = self.db.insert(&ev) {
                ev.id = id;
                self.events.insert(0, ev);
                any = true;
            }
        }
        any
    }

    fn filtered_sorted(&self) -> Vec<&FocusEvent> {
        let q = self.search.to_lowercase();
        let scope = self.search_scope;
        let mut v: Vec<&FocusEvent> = self.events.iter().filter(|e| {
            if q.is_empty() { return true; }
            let app = e.app_name.to_lowercase();
            let title = e.window_title.to_lowercase();
            let prev = e.previous_app.to_lowercase();
            let bundle = e.bundle_id.to_lowercase();
            let ts = e.ts.to_rfc3339().to_lowercase();
            match scope {
                SearchScope::All => app.contains(&q) || title.contains(&q) || prev.contains(&q) || bundle.contains(&q) || ts.contains(&q),
                SearchScope::Time => ts.contains(&q),
                SearchScope::App => app.contains(&q) || bundle.contains(&q),
                SearchScope::Title => title.contains(&q),
                SearchScope::Prev => prev.contains(&q),
            }
        }).collect();

        if let Some((key, asc)) = self.sort {
            v.sort_by(|a, b| {
                let o = match key {
                    SortKey::Time => a.ts.cmp(&b.ts),
                    SortKey::App => a.app_name.cmp(&b.app_name),
                    SortKey::Title => a.window_title.cmp(&b.window_title),
                    SortKey::Prev => a.previous_app.cmp(&b.previous_app),
                };
                if asc { o } else { o.reverse() }
            });
        }
        v
    }

    fn show_window(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.window_hidden = false;
    }

    fn cycle_sort(&mut self, key: SortKey) {
        self.sort = match self.sort {
            Some((k, true)) if k == key => Some((key, false)),
            Some((k, false)) if k == key => None,
            _ => Some((key, true)),
        };
    }

    fn header_label(&self, key: SortKey, name: &str) -> String {
        match self.sort {
            Some((k, true)) if k == key => format!("{name} ^"),
            Some((k, false)) if k == key => format!("{name} v"),
            _ => name.to_string(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let added = self.drain_incoming();
        if added {
            self.tray.refresh_recent(&self.events);
        }

        for action in poll_menu_events(&self.tray) {
            match action {
                MenuAction::Open => {
                    self.show_window(ctx);
                }
                MenuAction::Quit => {
                    self.quitting = true;
                }
                MenuAction::Pause(p) => {
                    self.paused = p;
                }
                MenuAction::Clear => {
                    if self.db.clear().is_ok() {
                        self.events.clear();
                        self.tray.refresh_recent(&self.events);
                    }
                }
            }
        }

        // Re-show window when user re-launches the .app while we're already running.
        // Drain to coalesce; only act when window is currently hidden so normal
        // activation clicks don't ping-pong focus.
        let mut reopen_pending = false;
        while self.reopen_rx.try_recv().is_ok() {
            reopen_pending = true;
        }
        if reopen_pending && self.window_hidden {
            self.show_window(ctx);
        }

        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if close_requested && !self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.window_hidden = true;
        }
        if self.quitting {
            // Hard-exit. ViewportCommand::Close doesn't reliably terminate
            // LSUIElement bundles, leaving the menu-bar icon stranded.
            std::process::exit(0);
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Logs, "Logs");
                ui.selectable_value(&mut self.tab, Tab::Settings, "Settings");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.tab {
                Tab::Logs => self.render_logs(ui),
                Tab::Settings => self.render_settings(ui),
            }
        });
    }
}

impl App {
    fn render_logs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search);
            ui.label("in");
            let prev_scope = self.search_scope;
            egui::ComboBox::from_id_salt("scope")
                .selected_text(scope_label(self.search_scope))
                .show_ui(ui, |ui| {
                    for s in [SearchScope::All, SearchScope::Time, SearchScope::App, SearchScope::Title, SearchScope::Prev] {
                        ui.selectable_value(&mut self.search_scope, s, scope_label(s));
                    }
                });
            if self.search_scope != prev_scope {
                self.settings.search_scope = self.search_scope;
                self.settings.save();
            }
            if ui.button("Clear search").clicked() { self.search.clear(); }
            ui.separator();
            ui.label(format!("{} events", self.events.len()));
            if ui.button("Clear all logs").clicked() && self.db.clear().is_ok() {
                self.events.clear();
            }
        });
        ui.separator();

        let rows = self.filtered_sorted();

        let time_label = self.header_label(SortKey::Time, "Time");
        let app_label = self.header_label(SortKey::App, "App");
        let title_label = self.header_label(SortKey::Title, "Window Title");
        let prev_label = self.header_label(SortKey::Prev, "Transition");

        let mut clicked: Option<SortKey> = None;

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::initial(170.0).at_least(120.0))
            .column(Column::initial(180.0).at_least(100.0))
            .column(Column::initial(280.0).at_least(120.0))
            .column(Column::remainder().at_least(180.0))
            .header(26.0, |mut h| {
                let active_key = self.sort.map(|(k, _)| k);
                h.col(|ui| {
                    if header_cell(ui, &time_label, active_key == Some(SortKey::Time)) {
                        clicked = Some(SortKey::Time);
                    }
                });
                h.col(|ui| {
                    if header_cell(ui, &app_label, active_key == Some(SortKey::App)) {
                        clicked = Some(SortKey::App);
                    }
                });
                h.col(|ui| {
                    if header_cell(ui, &title_label, active_key == Some(SortKey::Title)) {
                        clicked = Some(SortKey::Title);
                    }
                });
                h.col(|ui| {
                    if header_cell(ui, &prev_label, active_key == Some(SortKey::Prev)) {
                        clicked = Some(SortKey::Prev);
                    }
                });
            })
            .body(|body| {
                let row_h = 20.0;
                body.rows(row_h, rows.len(), |mut row| {
                    let e = rows[row.index()];
                    let local = e.ts.with_timezone(&chrono::Local);
                    row.col(|ui| { ui.label(local.format("%Y-%m-%d %H:%M:%S").to_string()); });
                    row.col(|ui| { ui.label(&e.app_name); });
                    row.col(|ui| { ui.label(&e.window_title); });
                    let prev = if e.previous_app.is_empty() { "-" } else { e.previous_app.as_str() };
                    row.col(|ui| { ui.label(format!("{} -> {}", prev, e.app_name)); });
                });
            });

        if let Some(k) = clicked { self.cycle_sort(k); }
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.add_space(8.0);

        #[cfg(target_os = "macos")]
        {
            ui.heading("Accessibility Permission");
            let trusted = crate::ax::is_trusted();
            ui.horizontal(|ui| {
                ui.label("Status:");
                if trusted {
                    ui.colored_label(egui::Color32::from_rgb(80, 200, 120), "GRANTED");
                } else {
                    ui.colored_label(egui::Color32::from_rgb(220, 100, 100), "NOT GRANTED");
                }
            });
            ui.add_space(4.0);
            ui.label("Why: macOS requires Accessibility permission for FocusTrace to read the");
            ui.label("focused window title of other applications. Without it, the App and");
            ui.label("Transition columns still log, but the Window Title column stays blank for");
            ui.label("every app except FocusTrace itself.");

            ui.add_space(8.0);
            ui.label("How to grant:");
            ui.label("  1. Open the Apple menu -> System Settings.");
            ui.label("  2. Sidebar: Privacy & Security -> Accessibility.");
            ui.label("  3. If FocusTrace is listed, toggle it ON.");
            ui.label("  4. If FocusTrace is NOT listed, click the + button, navigate to the");
            ui.label("     .app file (path shown below), select it, then toggle it ON.");
            ui.label("  5. Quit FocusTrace fully via the menu bar icon -> Quit.");
            ui.label("  6. Relaunch FocusTrace. Trust is read once at process start.");

            ui.add_space(8.0);
            ui.label("If permission was previously granted but the badge above says NOT GRANTED:");
            ui.label("  - The .app was rebuilt and its code signature changed. macOS treats it");
            ui.label("    as a different app. Remove the old FocusTrace entry in the");
            ui.label("    Accessibility list (select it, click -), then re-add the rebuilt .app.");
            ui.label("  - Persistent trust across rebuilds requires Developer ID code signing.");
            ui.label("    Ad-hoc signed builds may need re-granting after each rebuild.");

            ui.add_space(6.0);
            if !trusted && ui.button("Request prompt now").clicked() {
                crate::ax::prompt_trust();
            }
            ui.add_space(6.0);
            if let Ok(exe) = std::env::current_exe() {
                let app_path = exe
                    .ancestors()
                    .find(|p| p.extension().map(|e| e == "app").unwrap_or(false))
                    .map(|p| p.to_path_buf())
                    .unwrap_or(exe);
                ui.label(format!("App path: {}", app_path.display()));
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
        }

        let mut autostart = self.settings.autostart;
        if ui.checkbox(&mut autostart, "Start FocusTrace at login").changed() {
            self.settings.autostart = autostart;
            self.settings.save();
            let res = if autostart { autostart::enable() } else { autostart::disable() };
            if let Err(e) = res {
                eprintln!("autostart toggle failed: {e}");
            }
        }
        ui.label(format!("LaunchAgent plist: {}", autostart::plist_path().display()));

        ui.add_space(8.0);
        let mut show_tray = self.settings.show_tray;
        if ui.checkbox(&mut show_tray, "Show menu bar icon").changed() {
            self.settings.show_tray = show_tray;
            self.settings.save();
            self.tray.set_visible(show_tray);
        }
        ui.label("FocusTrace keeps logging in the background after the window is closed.");
        ui.label("Re-launching the app from Finder/Spotlight brings the window back.");
        if !self.settings.show_tray {
            ui.label("With the menu bar icon hidden, the only way to quit is the button below");
            ui.label("(or relaunching the app and using this Settings tab).");
        }

        ui.add_space(8.0);
        if ui.button("Quit FocusTrace").clicked() {
            self.quitting = true;
        }

        ui.add_space(12.0);
        ui.separator();
        ui.label(format!("Database: {}", crate::db::data_path().display()));

        ui.add_space(12.0);
        ui.label("Required: Accessibility permission may be requested by macOS for full window introspection.");
        ui.label("System Settings → Privacy & Security → Accessibility → enable FocusTrace.");
    }
}
