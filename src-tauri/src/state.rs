//! アプリ全体の共有状態。
//! Tauri の `.manage()` に渡し、各コマンドから `State<'_, AppState>` で取得する。
//!
//! 設計判断:
//! - 同期 Mutex (`std::sync::Mutex`) を使う: `#[tauri::command]` 本体は同期で
//!   `lock().unwrap()` で十分。`tokio::sync::Mutex` を持ち込まないことで
//!   core/* の Tauri 非依存ポリシーを保つ (docs/spec/backend/design.md)。
//! - `ticker_spawned` は `AtomicBool`: ticker は最初の `timer_start` で 1 回だけ
//!   spawn し、以後は compare_exchange で再 spawn を防ぐ。
//! - `AudioService::new` は失敗しない (silent fallback) ので `AppState::new` も
//!   Result を返さず `Self` を返す (Wave1 で確定した共有契約)。
//! - 現 TimerConfig は `machine.lock().set_config(...)` で書き換える方針なので
//!   AppState 側に `Mutex<TimerConfig>` を別途持たない (二重管理を避ける)。
//!
//! Send/Sync:
//! - `TimerMachine` はプリミティブ + `Option<SystemTime>` のみで Send + Sync 自動派生。
//! - `AudioService` は `rodio::OutputStream` を持つが `Option<OutputStream>` で
//!   包んでいるだけ。`Mutex<T: Send>` は `Send + Sync` になるため、Tauri State の
//!   要求 (`Send + Sync`) は `OutputStream: Send` だけ満たせば OK。
//!   現状 `cargo check` がエラーにならないことで担保する。

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use crate::core::audio_service::AudioService;
use crate::core::timer_machine::TimerMachine;
use crate::models::TimerConfig;

pub struct AppState {
    pub machine: Mutex<TimerMachine>,
    pub audio: Mutex<AudioService>,
    pub ticker_spawned: AtomicBool,
}

impl AppState {
    /// 起動時に `TimerConfig` (= 永続化済み Settings から派生) と
    /// 音アセットのディレクトリを渡して構築する。
    ///
    /// `AudioService::new` は出力デバイス取得失敗等を silent fallback で
    /// 吸収するので、ここでは Result を返さず `Self` を返す。
    pub fn new(config: TimerConfig, assets_dir: PathBuf) -> Self {
        Self {
            machine: Mutex::new(TimerMachine::new(config)),
            audio: Mutex::new(AudioService::new(assets_dir)),
            ticker_spawned: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Phase;
    use std::sync::atomic::Ordering;

    /// DoD: Default Settings から AppState を構築できる。
    /// - `AudioService::new` は silent fallback で必ず成功する想定
    /// - `ticker_spawned` は false で起動
    /// - `machine.lock()` で初期 `TimerState` (Idle) を取得できる
    #[test]
    fn default_settings_can_build_app_state() {
        let state = AppState::new(TimerConfig::default(), std::env::temp_dir());

        assert!(!state.ticker_spawned.load(Ordering::SeqCst));

        let machine = state.machine.lock().expect("machine lock poisoned");
        let snapshot = machine.state();
        assert_eq!(snapshot.phase, Phase::Idle);
        assert_eq!(snapshot.session_count, 0);
        // Default の work_seconds (1500) が反映されていること
        assert_eq!(snapshot.current_duration_seconds, 1500);
    }
}
