//! macOS メニューバー (Tray) の実装。
//!
//! TODO(backend): docs/spec/backend/design.md の `Tray 設計` 節と `tasks.md` B5 を参照
//! - TrayIconBuilder でアイコン + title (ゼロ埋め "00:00") 表示
//! - メニュー (開始 / 一時停止 / リセット / スキップ / ウィンドウを開く / 設定 / 終了)
//! - update_title(remaining) / update_icon(phase) を公開

#![allow(dead_code)]

use tauri::AppHandle;

use crate::error::AppResult;

pub fn setup(_app: &AppHandle) -> AppResult<()> {
    // TODO(backend): TrayIconBuilder で初期化
    Ok(())
}

pub fn update_title(_remaining_seconds: u32) {
    // TODO(backend): tray.set_title(format!("{:02}:{:02}", m, s))
}

pub fn update_icon_for_phase(_phase: crate::models::Phase) {
    // TODO(backend): Idle/Work/Break の 3 種 template image を切替
}
