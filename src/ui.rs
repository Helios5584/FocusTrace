use crate::autostart;
use crate::db::{Db, FocusEvent};
use crate::settings::Settings;
use crate::tray::{poll_menu_events, MenuAction, TrayHandle};
use crossbeam_channel::Receiver;
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortKey { Time, App, Title, Prev }

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortDir { Asc, Desc }

#[derive(PartialEq, Eq)]
enum Tab { Logs, Settings }

pub struct App {
    db: Arc<Db>,
    rx: Receiver<FocusEvent>,
    events: Vec<FocusEvent>,
    search: String,
    sort_key: SortKey,
    sort_dir: SortDir,
    tab: Tab,
    settings: Settings,
    tray: TrayHandle,
}

impl App {
    pub fn new(db: Arc<Db>, rx: Receiver<FocusEvent>, tray: TrayHandle) -> Self {
        let events = db.load_all().unwrap_or_default();
        let settings = Settings::load();
        Self {
            db,
            rx,
            events,
            search: String::new(),
            sort_key: SortKey::Time,
            sort_dir: SortDir::Desc,
            tab: Tab::Logs,
            settings,
            tray,
        }
    }

    fn drain_incoming(&mut self) {
        while let Ok(ev) = self.rx.try_recv() {
            let mut ev = ev;
            if let Ok(id) = self.db.insert(&ev) {
                ev.id = id;
                self.events.insert(0, ev);
            }
        }
    }

    fn filtered_sorted(&self) -> Vec<&FocusEvent> {
        let q = self.search.to_lowercase();
        let mut v: Vec<&FocusEvent> = self.events.iter().filter(|e| {
            if q.is_empty() { return true; }
            e.app_name.to_lowercase().contains(&q)
                || e.window_title.to_lowercase().contains(&q)
                || e.previous_app.to_lowercase().contains(&q)
                || e.bundle_id.to_lowercase().contains(&q)
                || e.ts.to_rfc3339().to_lowercase().contains(&q)
        }).collect();

        v.sort_by(|a, b| {
            let o = match self.sort_key {
                SortKey::Time => a.ts.cmp(&b.ts),
                SortKey::App => a.app_name.cmp(&b.app_name),
                SortKey::Title => a.window_title.cmp(&b.window_title),
                SortKey::Prev => a.previous_app.cmp(&b.previous_app),
            };
            if self.sort_dir == SortDir::Desc { o.reverse() } else { o }
        });
        v
    }

    fn header_btn(&mut self, ui: &mut egui::Ui, label: &str, key: SortKey) {
        let arrow = if self.sort_key == key {
            if self.sort_dir == SortDir::Desc { " ▼" } else { " ▲" }
        } else { "" };
        if ui.button(format!("{label}{arrow}")).clicked() {
            if self.sort_key == key {
                self.sort_dir = if self.sort_dir == SortDir::Desc { SortDir::Asc } else { SortDir::Desc };
            } else {
                self.sort_key = key;
                self.sort_dir = SortDir::Desc;
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_incoming();
        for action in poll_menu_events(&self.tray) {
            match action {
                MenuAction::Open => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                MenuAction::Quit => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
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
            if ui.button("Clear search").clicked() { self.search.clear(); }
            ui.separator();
            ui.label(format!("{} events", self.events.len()));
            if ui.button("Clear all logs").clicked() {
                if self.db.clear().is_ok() { self.events.clear(); }
            }
        });
        ui.separator();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            self.header_btn(ui, "Sort: Time", SortKey::Time);
            self.header_btn(ui, "App", SortKey::App);
            self.header_btn(ui, "Title", SortKey::Title);
            self.header_btn(ui, "Prev App", SortKey::Prev);
        });

        let rows: Vec<FocusEvent> = self.filtered_sorted().into_iter().cloned().collect();

        TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .column(Column::initial(170.0).at_least(120.0))
            .column(Column::initial(180.0).at_least(100.0))
            .column(Column::initial(280.0).at_least(120.0))
            .column(Column::remainder().at_least(180.0))
            .header(22.0, |mut h| {
                h.col(|ui| { ui.strong("Time"); });
                h.col(|ui| { ui.strong("App"); });
                h.col(|ui| { ui.strong("Window Title"); });
                h.col(|ui| { ui.strong("Transition"); });
            })
            .body(|body| {
                let row_h = 20.0;
                body.rows(row_h, rows.len(), |mut row| {
                    let i = row.index();
                    let e = &rows[i];
                    let local = e.ts.with_timezone(&chrono::Local);
                    row.col(|ui| { ui.label(local.format("%Y-%m-%d %H:%M:%S").to_string()); });
                    row.col(|ui| { ui.label(&e.app_name); });
                    row.col(|ui| { ui.label(&e.window_title); });
                    let prev = if e.previous_app.is_empty() { "—".to_string() } else { e.previous_app.clone() };
                    row.col(|ui| { ui.label(format!("{} → {}", prev, e.app_name)); });
                });
            });
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.add_space(8.0);

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

        ui.add_space(12.0);
        ui.separator();
        ui.label(format!("Database: {}", crate::db::data_path().display()));

        ui.add_space(12.0);
        ui.label("Required: Accessibility permission may be requested by macOS for full window introspection.");
        ui.label("System Settings → Privacy & Security → Accessibility → enable FocusTrace.");
    }
}
