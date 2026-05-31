//! 永続化 (settings: store, history: sqlite, active_session: store)
//!
//! TODO(backend): docs/spec/backend/design.md `Persistence 設計` 節 を参照

#![allow(dead_code)]

use crate::error::AppResult;
use crate::models::{ActiveSession, SessionRecord, Settings};
use tauri::AppHandle;

pub async fn load_settings(_app: &AppHandle) -> AppResult<Settings> {
    // TODO(backend): store から読み、失敗時は Default::default() を返す
    Ok(Settings::default())
}

pub async fn save_settings(_app: &AppHandle, _s: &Settings) -> AppResult<()> {
    unimplemented!("backend担当者が実装")
}

pub async fn save_active_session(_app: &AppHandle, _s: &ActiveSession) -> AppResult<()> {
    unimplemented!("backend担当者が実装")
}

pub async fn load_active_session(_app: &AppHandle) -> AppResult<Option<ActiveSession>> {
    Ok(None)
}

pub async fn clear_active_session(_app: &AppHandle) -> AppResult<()> {
    Ok(())
}

pub async fn record_session(_app: &AppHandle, _r: &SessionRecord) -> AppResult<i64> {
    unimplemented!("backend担当者が実装")
}

pub async fn list_sessions(
    _app: &AppHandle,
    _from: &str,
    _to: &str,
) -> AppResult<Vec<SessionRecord>> {
    Ok(vec![])
}

/// SQL migration 定義。
/// TODO(backend): tauri-plugin-sql の Migration vec! を返す
pub fn migrations() -> Vec<()> {
    vec![]
}
