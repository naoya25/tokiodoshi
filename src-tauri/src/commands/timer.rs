//! Timer commands (フロントから invoke される薄い IPC 層)
//!
//! TODO(backend): 内部で `core::timer_machine::TimerMachine` を呼ぶ
//! 全コマンドは `Result<TimerState, AppError>` を返す

use crate::error::AppResult;
use crate::models::TimerState;

#[tauri::command]
pub fn timer_start() -> AppResult<TimerState> {
    // TODO(backend): machine.start() を呼び、emit 'timer:state_changed'
    log::info!("[stub] timer_start called");
    Ok(TimerState::default())
}

#[tauri::command]
pub fn timer_pause() -> AppResult<TimerState> {
    log::info!("[stub] timer_pause called");
    Ok(TimerState::default())
}

#[tauri::command]
pub fn timer_reset() -> AppResult<TimerState> {
    log::info!("[stub] timer_reset called");
    Ok(TimerState::default())
}

#[tauri::command]
pub fn timer_skip() -> AppResult<TimerState> {
    log::info!("[stub] timer_skip called");
    Ok(TimerState::default())
}

#[tauri::command]
pub fn timer_get_state() -> AppResult<TimerState> {
    log::info!("[stub] timer_get_state called");
    Ok(TimerState::default())
}
