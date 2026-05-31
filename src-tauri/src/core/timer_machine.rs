//! TimerMachine: ポモドーロサイクルの純Rustステートマシン。
//! Tauri::* を一切 import せず、`cargo test` で単体テスト可能に保つ。
//!
//! TODO(backend): docs/spec/backend/design.md `TimerMachine 設計` 節 と
//! docs/spec/backend/tasks.md B2.1 の 10 ケーステストを満たすこと

#![allow(dead_code)]

use std::time::SystemTime;

use crate::models::{Phase, SessionKind, TimerConfig, TimerState};

pub enum TimerEvent {
    Tick(u32),
    StateChanged { phase: Phase, session_count: u32 },
    Completed { kind: SessionKind },
}

pub struct TimerMachine {
    state: TimerState,
    config: TimerConfig,
    end_at: Option<SystemTime>,
}

impl TimerMachine {
    pub fn new(config: TimerConfig) -> Self {
        Self {
            state: TimerState::default(),
            config,
            end_at: None,
        }
    }

    pub fn state(&self) -> &TimerState {
        &self.state
    }

    pub fn set_config(&mut self, c: TimerConfig) {
        self.config = c;
        // 現セッションは継続、次セッションから適用 (E3)
    }

    pub fn start(&mut self) -> TimerEvent {
        // TODO(backend): Idle → Work 遷移、end_at 計算、StateChanged を返す
        unimplemented!("backend担当者が実装")
    }

    pub fn pause(&mut self) -> TimerEvent {
        unimplemented!("backend担当者が実装")
    }

    pub fn reset(&mut self) -> TimerEvent {
        unimplemented!("backend担当者が実装")
    }

    pub fn skip(&mut self) -> Vec<TimerEvent> {
        unimplemented!("backend担当者が実装")
    }

    pub fn poll(&mut self, _now: SystemTime) -> Vec<TimerEvent> {
        // TODO(backend): end_at と now から残り時間を計算し、Tick と Completed を返す
        unimplemented!("backend担当者が実装")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_idle() {
        let m = TimerMachine::new(TimerConfig::default());
        assert_eq!(m.state().phase, Phase::Idle);
        assert_eq!(m.state().session_count, 0);
    }

    // TODO(backend): tasks.md B2.1 の残り 9 ケースを追加
}
