//! トキオドシ — Tauri アプリのエントリポイント。
//!
//! ここは集約のみに留め、ロジックは `core/` (Tauri 非依存)
//! と `commands/` (薄い IPC 層) に分離する。
//! 詳細: `docs/spec/backend/design.md`

mod commands;
mod core;
mod error;
mod models;
mod state;
mod tray;

use commands::{audio, history, settings, timer};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_sql::Builder::new().build())
        .setup(|app| {
            // TODO(backend): クラッシュリカバリ (active_session 復元)
            // TODO(backend): tray::setup(app.handle())?
            // TODO(backend): core::ticker::spawn(app.handle().clone())
            // TODO(backend): AudioService の初期化
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            timer::timer_start,
            timer::timer_pause,
            timer::timer_reset,
            timer::timer_skip,
            timer::timer_get_state,
            settings::settings_get,
            settings::settings_set,
            history::history_list,
            audio::audio_set_mode,
            audio::audio_set_volume,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
