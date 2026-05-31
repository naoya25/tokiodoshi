//! Tauri `#[tauri::command]` 関数群 (薄い IPC 層)。
//!
//! 各サブモジュールは関数を `pub` で公開しており、`lib.rs::run()` から
//! `tauri::generate_handler![commands::timer::timer_start, ...]` で参照する。
//! 上位から `commands::*` の名前でもアクセスできるように `pub use` も行う。

pub mod audio;
pub mod history;
pub mod settings;
pub mod timer;

pub use audio::{audio_set_mode, audio_set_volume};
pub use history::history_list;
pub use settings::{settings_get, settings_set};
pub use timer::{timer_get_state, timer_pause, timer_reset, timer_skip, timer_start};
