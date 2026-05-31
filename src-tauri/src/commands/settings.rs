//! Settings commands.
//!
//! - `settings_get`: persistence から load (失敗時は Default を返す方針が persistence に既に組み込まれている)
//! - `settings_set`: save + AudioService 即時反映 + TimerMachine.set_config (次セッションから, E3)

use tauri::{AppHandle, State};

use crate::core::persistence;
use crate::error::AppResult;
use crate::models::{Settings, VolumeKind};
use crate::state::AppState;

#[tauri::command]
pub async fn settings_get(app: AppHandle) -> AppResult<Settings> {
    // persistence::load_settings 自体が破損時 Default::default() を返すので
    // ここは Result を介さない (Settings をそのまま返す)。
    Ok(persistence::load_settings(&app).await)
}

#[tauri::command]
pub async fn settings_set(
    settings: Settings,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    // 1) 永続化を先に行い、失敗時は他の副作用を実行しない (一貫性確保)
    persistence::save_settings(&app, &settings).await?;

    // 2) AudioService に mode / volumes を即時反映
    //    ロックスコープを限定 (with-block)
    {
        let mut audio = match state.audio.lock() {
            Ok(g) => g,
            Err(poisoned) => {
                log::warn!("settings_set: AudioService mutex poisoned, recovering");
                poisoned.into_inner()
            }
        };
        audio.set_mode(settings.audio.mode);
        audio.set_volume(VolumeKind::Master, settings.audio.master_volume);
        audio.set_volume(VolumeKind::Water, settings.audio.water_volume);
        audio.set_volume(VolumeKind::Kakon, settings.audio.kakon_volume);
    }

    // 3) TimerMachine の config を次セッションから有効化 (E3: 現セッションは継続)
    //    loop_mode は即時反映 (内部フラグの切替なので副作用なし)
    {
        let mut machine = match state.machine.lock() {
            Ok(g) => g,
            Err(poisoned) => {
                log::warn!("settings_set: TimerMachine mutex poisoned, recovering");
                poisoned.into_inner()
            }
        };
        machine.set_config((&settings.durations).into());
        machine.set_loop_mode(settings.behavior.loop_sessions);
    }

    Ok(())
}
