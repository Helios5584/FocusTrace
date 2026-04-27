use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct TrayHandle {
    pub _tray: TrayIcon,
    pub open_id: tray_icon::menu::MenuId,
    pub quit_id: tray_icon::menu::MenuId,
}

fn make_icon() -> Icon {
    // 16x16 RGBA dark dot on transparent.
    let size: u32 = 16;
    let mut buf = vec![0u8; (size * size * 4) as usize];
    let cx = 7.5f32;
    let cy = 7.5f32;
    let r = 6.5f32;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let i = ((y * size + x) * 4) as usize;
            if dist <= r {
                buf[i] = 220;
                buf[i + 1] = 220;
                buf[i + 2] = 220;
                buf[i + 3] = 255;
            }
        }
    }
    Icon::from_rgba(buf, size, size).expect("icon")
}

pub fn install(_ctx: &eframe::egui::Context) -> TrayHandle {
    let menu = Menu::new();
    let open = MenuItem::new("Open FocusTrace", true, None);
    let quit = MenuItem::new("Quit", true, None);
    menu.append(&open).ok();
    menu.append(&quit).ok();
    let open_id = open.id().clone();
    let quit_id = quit.id().clone();

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("FocusTrace")
        .with_icon(make_icon())
        .build()
        .expect("tray");

    TrayHandle { _tray: tray, open_id, quit_id }
}

pub fn poll_menu_events(handle: &TrayHandle) -> Vec<MenuAction> {
    let mut out = Vec::new();
    while let Ok(ev) = MenuEvent::receiver().try_recv() {
        if ev.id == handle.open_id {
            out.push(MenuAction::Open);
        } else if ev.id == handle.quit_id {
            out.push(MenuAction::Quit);
        }
    }
    out
}

#[derive(Clone, Copy, Debug)]
pub enum MenuAction { Open, Quit }
