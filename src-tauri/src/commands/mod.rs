//! Tauri `#[tauri::command]` 関数群 (薄い IPC 層)。
//!
//! 各サブモジュールは関数を `pub` で公開しており、`lib.rs::run()` の
//! `tauri::generate_handler![commands::timer::timer_start, ...]` からフルパスで参照する。
//! re-export は不要なので置かない (生やすと `unused_imports` の警告が出る)。

pub mod audio;
pub mod history;
pub mod settings;
pub mod timer;
