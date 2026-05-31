//! Audio commands
//!
//! TODO(backend): `core::audio_service::AudioService` 経由で rodio を操作

use crate::error::AppResult;
use crate::models::AudioMode;

#[tauri::command]
pub fn audio_set_mode(mode: AudioMode) -> AppResult<()> {
    log::info!("[stub] audio_set_mode called: {:?}", mode);
    Ok(())
}

#[tauri::command]
pub fn audio_set_volume(kind: String, value: f32) -> AppResult<()> {
    log::info!("[stub] audio_set_volume called: {}={}", kind, value);
    // TODO(backend): kind を "master" | "water" | "kakon" で分岐
    Ok(())
}
