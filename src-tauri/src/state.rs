//! アプリ全体の共有状態。
//! Tauri の `.manage()` に渡し、各コマンドから `State<'_, AppState>` で取得する。
//!
//! TODO(backend): TimerMachine / AudioService を実装したらここに統合する

use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use crate::models::TimerConfig;

pub struct AppState {
    // TODO(backend): pub machine: Mutex<TimerMachine>,
    // TODO(backend): pub audio: Mutex<AudioService>,
    pub ticker_spawned: AtomicBool,
    pub config: Mutex<TimerConfig>,
}

impl AppState {
    pub fn new(config: TimerConfig) -> Self {
        Self {
            ticker_spawned: AtomicBool::new(false),
            config: Mutex::new(config),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(TimerConfig::default())
    }
}
