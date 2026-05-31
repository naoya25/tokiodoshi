//! Timer commands (フロントから invoke される薄い IPC 層)
//!
//! 各コマンドは `State<'_, AppState>` + `AppHandle` を取り、
//! `TimerMachine` を mutate した結果の `Vec<TimerEvent>` を
//! `core::ticker::dispatch_event` に通して emit + 副作用を走らせる。
//!
//! ## なぜ ticker に委譲しないか
//! ticker は 250ms 周期なので、IPC レスポンスから emit/副作用までに
//! 最大 250ms のラグが生じる。フロントは invoke の戻り値を待った直後に
//! `useTimer` の state を更新するため、ここで同期的に emit を済ませて
//! "コマンド完了 = 状態変化が emit 済み" の不変を満たす。
//!
//! ## ロックスコープ
//! `state.machine.lock()` は短く保つ (events 取得 + snapshot まで)。
//! emit + audio + persistence の I/O はロック外で実行する
//! (ticker 側と同じ pattern, design.md の方針)。

use std::time::SystemTime;

use tauri::{AppHandle, State};

use crate::core::ticker::dispatch_event;
use crate::core::timer_machine::TimerMachine;
use crate::error::AppResult;
use crate::models::{TimerEvent, TimerState};
use crate::state::AppState;

/// machine をロックして `f` を実行し、events と snapshot を取り出す。
/// ロックは `f` 終了とともに解放する (I/O はロック外)。
fn run_machine<F>(state: &AppState, f: F) -> (Vec<TimerEvent>, TimerState)
where
    F: FnOnce(&mut TimerMachine) -> Vec<TimerEvent>,
{
    let mut machine = match state.machine.lock() {
        Ok(g) => g,
        Err(poisoned) => {
            log::error!("timer command: TimerMachine mutex poisoned, recovering");
            poisoned.into_inner()
        }
    };
    let events = f(&mut machine);
    let snapshot = machine.state();
    (events, snapshot)
}

/// `Vec<TimerEvent>` を 1 件ずつ dispatch する。
async fn dispatch_all(app: &AppHandle, events: &[TimerEvent], snapshot: &TimerState) {
    for ev in events {
        dispatch_event(app, ev, snapshot).await;
    }
}

#[tauri::command]
pub async fn timer_start(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<TimerState> {
    let now = SystemTime::now();
    let (events, snapshot) = run_machine(&state, |m| m.start(now));
    dispatch_all(&app, &events, &snapshot).await;
    Ok(snapshot)
}

#[tauri::command]
pub async fn timer_pause(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<TimerState> {
    let now = SystemTime::now();
    let (events, snapshot) = run_machine(&state, |m| m.pause(now));
    dispatch_all(&app, &events, &snapshot).await;
    Ok(snapshot)
}

#[tauri::command]
pub async fn timer_reset(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<TimerState> {
    let (events, snapshot) = run_machine(&state, |m| m.reset());
    dispatch_all(&app, &events, &snapshot).await;
    Ok(snapshot)
}

#[tauri::command]
pub async fn timer_skip(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<TimerState> {
    let now = SystemTime::now();
    // skip は `Completed` + `StateChanged` を順に返す。両方 dispatch する。
    let (events, snapshot) = run_machine(&state, |m| m.skip(now));
    dispatch_all(&app, &events, &snapshot).await;
    Ok(snapshot)
}

#[tauri::command]
pub async fn timer_get_state(state: State<'_, AppState>) -> AppResult<TimerState> {
    // 状態の読み出しだけ。events は発火しない。
    let machine = match state.machine.lock() {
        Ok(g) => g,
        Err(poisoned) => {
            log::error!("timer_get_state: mutex poisoned, recovering");
            poisoned.into_inner()
        }
    };
    Ok(machine.state())
}
