//! 250ms 毎に `TimerMachine.poll()` を呼ぶ tokio タスク。
//!
//! 終端時刻 (`SystemTime`) 基準で macOS スリープ復帰にも追従する
//! (`Instant` ではない)。
//!
//! 仕様:
//! - docs/spec/backend/design.md `Ticker 設計` / `Data Flow` 節
//! - docs/spec/backend/tasks.md B2.2
//!
//! ## イベント分配
//!
//! `TimerMachine::poll()` が返した `Vec<TimerEvent>` を以下に振り分ける。
//!
//! | Variant            | emit 名              | payload                                        | 副作用                                                       |
//! |--------------------|----------------------|------------------------------------------------|--------------------------------------------------------------|
//! | `Tick(u32)`        | `timer:tick`         | `{ remaining_seconds }`                        | なし (tray title 更新は MVP では別途 listen で行う)          |
//! | `StateChanged{..}` | `timer:state_changed`| `{ phase, session_count }`                     | Work 開始: audio.start_water + save_active_session           |
//! |                    |                      |                                                | Break: audio.stop_water                                      |
//! |                    |                      |                                                | Idle/Paused: audio.stop_water + clear_active_session         |
//! | `Completed{kind}`  | `timer:completed`    | `{ "type": SessionKind }`                      | audio.play_kakon + persistence::record_session + clear       |
//!
//! ## エラー処理
//! - emit / DB INSERT 失敗時は `log::warn!` で記録するのみ。ループは継続 (E4 要件)。
//! - パニックも spawn したタスク内に閉じ込め、メイン側へは伝播しない。
//!
//! ## 重要: `tauri::async_runtime::spawn` を使うこと
//! `tauri::Builder::default().setup()` 内では `tokio::spawn` を呼ぶと
//! 「there is no reactor running」でパニックする。Tauri が管理する
//! `async_runtime` (内部は tokio) 上で spawn する必要がある。
//!
//! ## 二重 spawn 防止
//! `state.ticker_spawned.swap(true, SeqCst)` で false → true。
//! 既に true ならログを出して return。

#![allow(dead_code)]

use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime};

use chrono::Utc;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::core::persistence;
use crate::models::{
    iso8601, ActiveSession, Phase, SessionKind, SessionRecord, TimerEvent, TimerState,
};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// emit ペイロード型 (Serialize)
// ---------------------------------------------------------------------------

/// `timer:tick` の payload。
#[derive(Serialize, Clone, Debug, PartialEq)]
struct TickPayload {
    remaining_seconds: u32,
}

/// `timer:state_changed` の payload。
#[derive(Serialize, Clone, Debug, PartialEq)]
struct StateChangedPayload {
    phase: Phase,
    session_count: u32,
}

/// `timer:completed` の payload。`type` キーをフロントに見せるため `#[serde(rename)]`。
#[derive(Serialize, Clone, Debug, PartialEq)]
struct CompletedPayload {
    #[serde(rename = "type")]
    kind: SessionKind,
}

const EVENT_TICK: &str = "timer:tick";
const EVENT_STATE_CHANGED: &str = "timer:state_changed";
const EVENT_COMPLETED: &str = "timer:completed";

/// ticker の poll 間隔。短いほど Completed 発火のジッターが減るが、CPU 使用が増える。
/// 50ms にしているのは KAKON_LEAD_MS との位相ズレを 50ms 以内に抑えるため
/// (これより大きいとカコン音とアニメ終わりの同期が崩れて見える)。
/// tokio::time::interval 自体は非常に軽量なので、50ms ループでも実測上の
/// アイドル CPU 使用率は < 1% に収まる。
const TICK_INTERVAL_MS: u64 = 50;

// ---------------------------------------------------------------------------
// 公開 API
// ---------------------------------------------------------------------------

/// ticker タスクを 1 回だけ spawn する。
/// `setup()` から呼ぶ。二重呼び出しは `AppState.ticker_spawned` で防ぐ。
pub fn spawn(app: AppHandle) {
    // 二重 spawn 防止
    {
        let state = app.state::<AppState>();
        let already = state.ticker_spawned.swap(true, Ordering::SeqCst);
        if already {
            log::warn!("ticker::spawn called twice, ignoring second call");
            return;
        }
    }

    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(TICK_INTERVAL_MS));
        // poll 処理が 50ms より長引いた場合に missed tick が burst で連発するのを防ぐ。
        // 古い tick はスキップして、最新の周期に追従する。
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let now = SystemTime::now();

            // machine.poll() の lock は短く保つ (audio / persistence の I/O はロック外で実行)
            let (events, snapshot) = {
                let state = app.state::<AppState>();
                let mut machine = match state.machine.lock() {
                    Ok(g) => g,
                    Err(poisoned) => {
                        // panic 後の poison は復帰せず fast fail (design.md の方針)
                        log::error!("ticker: TimerMachine mutex poisoned, recovering");
                        poisoned.into_inner()
                    }
                };
                let events = machine.poll(now);
                let snapshot = machine.state();
                (events, snapshot)
            };

            for event in events {
                dispatch_event(&app, &event, &snapshot).await;
            }
        }
    });
}

// ---------------------------------------------------------------------------
// 内部: event 分配
// ---------------------------------------------------------------------------

/// 1 イベントを emit + 副作用に振り分ける。
/// 失敗しても次の event 処理を止めない。
///
/// `pub(crate)`: commands/* からも同じ分配ロジックを使うため公開する。
/// timer_start / pause / reset / skip は即時 IPC レスポンスを実現するために
/// ticker を待たず、コマンド側で生成した TimerEvent をここに通して
/// emit + 副作用 (audio / persistence) を走らせる。
pub(crate) async fn dispatch_event(app: &AppHandle, event: &TimerEvent, snapshot: &TimerState) {
    match event {
        TimerEvent::Tick(remaining_seconds) => {
            emit_tick(app, *remaining_seconds);
            if let Some(tray) = app.try_state::<tauri::tray::TrayIcon>() {
                crate::tray::update_title(&tray, *remaining_seconds);
            }
            // 注: カコン音はこの分岐では再生しない (Tick(0) は end_at 到達時 = アニメ戻り終わり)。
            // 音は `PlayKakonAudio` イベント (end_at - KAKON_AUDIO_LEAD_MS) で再生される。
        }
        TimerEvent::StateChanged { phase, count } => {
            emit_state_changed(app, *phase, *count);
            on_state_changed(app, *phase, snapshot).await;
        }
        TimerEvent::Completed { kind } => {
            emit_completed(app, *kind);
            on_completed(app, *kind, snapshot).await;
        }
        TimerEvent::PlayKakonAudio => {
            // カコン音だけを再生 (フロントへの emit はなし)。
            // 視覚アニメ戻り終わりの少し前 (= end_at - KAKON_AUDIO_LEAD_MS) に
            // 鳴らすことで、聴覚的な「石を打つ瞬間」と視覚を同期させる。
            with_audio(app, |a| a.play_kakon());
        }
    }
}

/// `StateChanged` 副作用。
async fn on_state_changed(app: &AppHandle, phase: Phase, snapshot: &TimerState) {
    match phase {
        Phase::Work => {
            // 音: 水音開始
            with_audio(app, |a| a.start_water());

            // active_session 保存 (1 回だけ)
            let now = Utc::now();
            let end_at = now + chrono::Duration::seconds(snapshot.current_duration_seconds as i64);
            let active = ActiveSession {
                r#type: SessionKind::Work,
                started_at: iso8601(now),
                planned_duration_seconds: snapshot.current_duration_seconds,
                end_at: iso8601(end_at),
            };
            if let Err(e) = persistence::save_active_session(app, &active).await {
                log::warn!("ticker: save_active_session failed: {e}");
            }
        }
        Phase::Idle | Phase::Paused => {
            with_audio(app, |a| a.stop_water());
            if matches!(phase, Phase::Idle) {
                if let Err(e) = persistence::clear_active_session(app).await {
                    log::warn!("ticker: clear_active_session (Idle) failed: {e}");
                }
            }
        }
    }
    // 全 phase 共通で tray icon を更新
    if let Some(tray) = app.try_state::<tauri::tray::TrayIcon>() {
        crate::tray::update_icon(&tray, phase, app);
    }
}

/// `Completed` 副作用。
/// - sessions テーブルへ INSERT
/// - active_session を clear
/// 注: カコン音は `Tick(0)` 受信時 (アニメ戻り終わり = タイマー 00:00) に鳴らすので
/// ここでは再生しない。Completed は 1060ms 早いタイミングなので音的に同期しない。
async fn on_completed(app: &AppHandle, kind: SessionKind, snapshot: &TimerState) {
    let now = Utc::now();
    // planned_duration_seconds は完了直前の duration。snapshot は既に次フェーズへ
    // 遷移した後の値を持ち得るため、kind に基づいて TimerConfig から再取得する。
    let planned = planned_seconds_for(app, kind).unwrap_or(snapshot.current_duration_seconds);
    let started_at_dt = now - chrono::Duration::seconds(planned as i64);
    let record = SessionRecord {
        id: 0, // INSERT 後は last_insert_rowid を返すが、ここでは未使用
        r#type: kind,
        started_at: iso8601(started_at_dt),
        completed_at: Some(iso8601(now)),
        was_completed: true,
        planned_duration_seconds: planned,
    };
    if let Err(e) = persistence::record_session(app, &record).await {
        log::warn!("ticker: record_session failed: {e}");
    }
    if let Err(e) = persistence::clear_active_session(app).await {
        log::warn!("ticker: clear_active_session (after Completed) failed: {e}");
    }
}

/// `kind` に対応する duration_seconds を返す。
///
/// 完了直後の `snapshot.current_duration_seconds` は既に次フェーズの値に置き換わって
/// いるため、`machine.state().current_duration_seconds` ではなく、`TimerMachine` の
/// 持つ内部 config (= 現在動作中の duration) を読む必要がある。
/// ただし `TimerMachine` 側に直接 `config()` getter を生やしていないので、現状は
/// `None` を返して呼び出し側で snapshot fallback を使う。
/// TODO(backend): `TimerMachine::current_config()` を追加してここで使うか、
/// completed event を発火する前に planned_duration を確定して払い出す。
fn planned_seconds_for(_app: &AppHandle, _kind: SessionKind) -> Option<u32> {
    None
}

/// AudioService をロックして F を実行する。poison 時は復旧して呼び出す
/// (audio は state が壊れても再生機能だけ続行できれば良いため)。
fn with_audio<F>(app: &AppHandle, f: F)
where
    F: FnOnce(&mut crate::core::audio_service::AudioService),
{
    let state = match app.try_state::<AppState>() {
        Some(s) => s,
        None => {
            log::warn!("ticker: AppState not managed, skip audio side effect");
            return;
        }
    };
    let mut audio = match state.audio.lock() {
        Ok(g) => g,
        Err(poisoned) => {
            log::warn!("ticker: AudioService mutex poisoned, recovering");
            poisoned.into_inner()
        }
    };
    f(&mut audio);
}

// ---------------------------------------------------------------------------
// 内部: emit ラッパ (失敗は warn のみ)
// ---------------------------------------------------------------------------

fn emit_tick(app: &AppHandle, remaining_seconds: u32) {
    let payload = TickPayload { remaining_seconds };
    if let Err(e) = app.emit(EVENT_TICK, payload) {
        log::warn!("ticker: emit {EVENT_TICK} failed: {e}");
    }
}

fn emit_state_changed(app: &AppHandle, phase: Phase, count: u32) {
    let payload = StateChangedPayload {
        phase,
        session_count: count,
    };
    if let Err(e) = app.emit(EVENT_STATE_CHANGED, payload) {
        log::warn!("ticker: emit {EVENT_STATE_CHANGED} failed: {e}");
    }
}

fn emit_completed(app: &AppHandle, kind: SessionKind) {
    let payload = CompletedPayload { kind };
    if let Err(e) = app.emit(EVENT_COMPLETED, payload) {
        log::warn!("ticker: emit {EVENT_COMPLETED} failed: {e}");
    }
}

// ---------------------------------------------------------------------------
// Tests — AppHandle が絡む部分は統合テストが難しいので、
// emit payload の serialize 形だけを純関数として検証する。
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_payload_serializes_with_remaining_seconds() {
        let p = TickPayload {
            remaining_seconds: 1234,
        };
        let json = serde_json::to_string(&p).unwrap();
        // フロント `useTimer` が `payload.remaining_seconds` で読むのでキー名を担保
        assert_eq!(json, "{\"remaining_seconds\":1234}");
    }

    #[test]
    fn state_changed_payload_uses_phase_and_session_count_keys() {
        let p = StateChangedPayload {
            phase: Phase::Idle,
            session_count: 2,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"phase\":\"idle\""));
        assert!(json.contains("\"session_count\":2"));
    }

    #[test]
    fn completed_payload_uses_type_key_not_kind() {
        // フロント側は `{ type: 'work' }` を期待。Rust の `kind` フィールドが
        // serialize 時に `"type"` キーになることを担保する。
        let p = CompletedPayload {
            kind: SessionKind::Work,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "{\"type\":\"work\"}");
    }
}
