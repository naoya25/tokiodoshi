//! Settings commands
//!
//! TODO(backend): tauri-plugin-store で永続化 (`core::persistence`)

use crate::error::AppResult;
use crate::models::Settings;

#[tauri::command]
pub fn settings_get() -> AppResult<Settings> {
    log::info!("[stub] settings_get called");
    Ok(Settings::default())
}

#[tauri::command]
pub fn settings_set(settings: Settings) -> AppResult<()> {
    log::info!("[stub] settings_set called: {:?}", settings);
    // TODO(backend): persistence::save_settings + AudioService.set_mode/set_volume 反映
    Ok(())
}
