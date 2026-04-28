use crate::db::FocusEvent;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub const RECENT_SLOTS: usize = 8;

pub struct TrayHandle {
    pub _tray: TrayIcon,
    pub open_id: MenuId,
    pub quit_id: MenuId,
    pub clear_id: MenuId,
    pub pause_id: MenuId,
    pub pause: CheckMenuItem,
    pub recent: Vec<MenuItem>,
}

impl TrayHandle {
    pub fn refresh_recent(&self, events: &[FocusEvent]) {
        for (i, slot) in self.recent.iter().enumerate() {
            match events.get(i) {
                Some(ev) => {
                    let local = ev.ts.with_timezone(&chrono::Local);
                    let app = if ev.app_name.is_empty() { "(unknown)" } else { ev.app_name.as_str() };
                    let mut line = format!("{}  {}", local.format("%H:%M:%S"), app);
                    if !ev.window_title.is_empty() {
                        let title = ev.window_title.chars().take(48).collect::<String>();
                        line.push_str(" — ");
                        line.push_str(&title);
                    }
                    slot.set_text(&line);
                }
                None => slot.set_text("—"),
            }
        }
    }
}

fn aa(d: f32) -> f32 {
    (0.5 - d).clamp(0.0, 1.0)
}

fn rrect_sdf(x: f32, y: f32, cx: f32, cy: f32, hw: f32, hh: f32, r: f32) -> f32 {
    let qx = (x - cx).abs() - hw + r;
    let qy = (y - cy).abs() - hh + r;
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    let inside = qx.max(qy).min(0.0);
    outside + inside - r
}

fn make_icon() -> Icon {
    // 36x36 monochrome template: two offset rounded "windows" (back outlined,
    // front filled). macOS recolors template images for light/dark menu bars.
    let size: u32 = 36;
    let s = size as f32;
    let cx = (s - 1.0) / 2.0;
    let cy = (s - 1.0) / 2.0;
    let win_hw = s * 0.30;
    let win_hh = s * 0.26;
    let win_r = s * 0.08;
    let stroke = s * 0.085;
    let off_x = s * 0.13;
    let off_y = s * 0.14;
    let back_cx = cx - off_x;
    let back_cy = cy - off_y;
    let front_cx = cx + off_x;
    let front_cy = cy + off_y;
    // Negative-space gap so back outline doesn't touch front fill.
    let gap = s * 0.06;

    let mut buf = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let xf = x as f32;
            let yf = y as f32;

            let d_back = rrect_sdf(xf, yf, back_cx, back_cy, win_hw, win_hh, win_r);
            let d_front = rrect_sdf(xf, yf, front_cx, front_cy, win_hw, win_hh, win_r);

            // Front: filled.
            let front_cov = aa(d_front);
            // Back: outline (annulus).
            let mut back_cov = aa(d_back).min(aa(-d_back - stroke));
            // Erase back where it overlaps the front (with small gap).
            let near_front = aa(d_front - gap);
            back_cov *= 1.0 - near_front;

            let cov = back_cov.max(front_cov).clamp(0.0, 1.0);
            let a = (cov * 255.0) as u8;
            let i = ((y * size + x) * 4) as usize;
            buf[i] = 0;
            buf[i + 1] = 0;
            buf[i + 2] = 0;
            buf[i + 3] = a;
        }
    }
    Icon::from_rgba(buf, size, size).expect("icon")
}

pub fn install(_ctx: &eframe::egui::Context) -> TrayHandle {
    let menu = Menu::new();
    let open = MenuItem::new("Open FocusTrace", true, None);
    menu.append(&open).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();

    let recent_header = MenuItem::new("Recent activity", false, None);
    menu.append(&recent_header).ok();
    let mut recent = Vec::with_capacity(RECENT_SLOTS);
    for _ in 0..RECENT_SLOTS {
        let it = MenuItem::new("—", false, None);
        menu.append(&it).ok();
        recent.push(it);
    }
    menu.append(&PredefinedMenuItem::separator()).ok();

    let pause = CheckMenuItem::new("Pause logging", true, false, None);
    menu.append(&pause).ok();
    let clear = MenuItem::new("Clear logs", true, None);
    menu.append(&clear).ok();
    menu.append(&PredefinedMenuItem::separator()).ok();
    let quit = MenuItem::new("Quit", true, None);
    menu.append(&quit).ok();

    let open_id = open.id().clone();
    let quit_id = quit.id().clone();
    let clear_id = clear.id().clone();
    let pause_id = pause.id().clone();

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("FocusTrace")
        .with_icon(make_icon())
        .with_icon_as_template(true)
        .build()
        .expect("tray");

    TrayHandle {
        _tray: tray,
        open_id,
        quit_id,
        clear_id,
        pause_id,
        pause,
        recent,
    }
}

pub fn poll_menu_events(handle: &TrayHandle) -> Vec<MenuAction> {
    let mut out = Vec::new();
    while let Ok(ev) = MenuEvent::receiver().try_recv() {
        if ev.id == handle.open_id {
            out.push(MenuAction::Open);
        } else if ev.id == handle.quit_id {
            out.push(MenuAction::Quit);
        } else if ev.id == handle.clear_id {
            out.push(MenuAction::Clear);
        } else if ev.id == handle.pause_id {
            out.push(MenuAction::Pause(handle.pause.is_checked()));
        }
    }
    out
}

#[derive(Clone, Copy, Debug)]
pub enum MenuAction {
    Open,
    Quit,
    Clear,
    Pause(bool),
}
