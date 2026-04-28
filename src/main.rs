#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
#[cfg(target_os = "macos")]
mod ax;
mod db;
mod focus;
#[cfg(target_os = "macos")]
mod reopen;
mod settings;
mod tray;
mod ui;

use crossbeam_channel::unbounded;
use db::Db;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    let db = Arc::new(Db::open().expect("open db"));
    let (tx, rx) = unbounded();
    let (reopen_tx, reopen_rx) = unbounded::<()>();

    #[cfg(target_os = "macos")]
    {
        if !ax::is_trusted() {
            ax::prompt_trust();
        }
    }

    #[cfg(target_os = "macos")]
    let _observer = focus::install(tx.clone());
    #[cfg(target_os = "macos")]
    let _reopen_observer = reopen::install(reopen_tx.clone());
    #[cfg(not(target_os = "macos"))]
    {
        let _ = tx;
        let _ = reopen_tx;
    }

    let opts = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_min_inner_size([520.0, 320.0])
            .with_title("FocusTrace"),
        ..Default::default()
    };

    eframe::run_native(
        "FocusTrace",
        opts,
        Box::new(move |cc| {
            let tray_handle = tray::install(&cc.egui_ctx);
            Ok(Box::new(ui::App::new(db, rx, reopen_rx, tray_handle)))
        }),
    )
}
