//! History commands
//!
//! TODO(backend): tauri-plugin-sql で SQLite から SELECT

use crate::error::AppResult;
use crate::models::SessionRecord;

#[tauri::command]
pub fn history_list(from: String, to: String) -> AppResult<Vec<SessionRecord>> {
    log::info!("[stub] history_list called: {} → {}", from, to);
    Ok(vec![])
}
