//! TimerMachine: シンプルな作業セッションのステートマシン。
//! Tauri::* を一切 import せず、`cargo test` で単体テスト可能に保つ。
//!
//! MVP 設計:
//! - フェーズは Idle / Work / Paused の 3 つだけ
//! - 休憩フェーズは廃止 (Work 完了 → Idle に戻る)
//! - `set_config()` は次の `start()` / `reset()` 時に commit (E3: 実行中は据え置き)
//!
//! 仕様:
//! - docs/spec/backend/design.md `TimerMachine 設計` 節
//! - docs/spec/backend/tasks.md B2.1

#![allow(dead_code)]

use std::time::{Duration, SystemTime};

use crate::models::timer_state::TimerEvent;
use crate::models::{Phase, SessionKind, TimerConfig, TimerState};

pub struct TimerMachine {
    state: TimerState,
    config: TimerConfig,
    pending_config: Option<TimerConfig>,
    end_at: Option<SystemTime>,
    paused_remaining: Option<Duration>,
}

impl TimerMachine {
    pub fn new(config: TimerConfig) -> Self {
        let work_seconds = config.work_seconds;
        Self {
            state: TimerState {
                phase: Phase::Idle,
                remaining_seconds: work_seconds,
                session_count: 0,
                current_duration_seconds: work_seconds,
            },
            config,
            pending_config: None,
            end_at: None,
            paused_remaining: None,
        }
    }

    /// 現在の状態スナップショット。
    pub fn state(&self) -> TimerState {
        self.state.clone()
    }

    /// 次の Idle → Work 遷移 (start) または reset 時に有効になる config を積む。
    /// 現セッションには影響しない (E3)。
    pub fn set_config(&mut self, c: TimerConfig) {
        self.pending_config = Some(c);
    }

    /// 内部: pending_config があれば commit する。
    fn commit_pending_config(&mut self) {
        if let Some(p) = self.pending_config.take() {
            self.config = p;
        }
    }

    /// start():
    /// - Idle → Work (pending_config を commit してから新セッション開始)
    /// - Paused → 直前 (= Work) に resume、残り時間を保持
    /// - 走行中 (Work) は何もしない
    pub fn start(&mut self, now: SystemTime) -> Vec<TimerEvent> {
        match self.state.phase {
            Phase::Idle => {
                self.commit_pending_config();
                let work_seconds = self.config.work_seconds;
                self.state.current_duration_seconds = work_seconds;
                self.state.remaining_seconds = work_seconds;
                self.state.phase = Phase::Work;
                self.end_at = if work_seconds > 0 {
                    Some(now + Duration::from_secs(work_seconds as u64))
                } else {
                    None
                };
                vec![TimerEvent::StateChanged {
                    phase: Phase::Work,
                    count: self.state.session_count,
                }]
            }
            Phase::Paused => {
                let remaining = self.paused_remaining.unwrap_or_else(|| {
                    Duration::from_secs(self.state.remaining_seconds as u64)
                });
                self.state.phase = Phase::Work;
                self.state.remaining_seconds = remaining.as_secs() as u32;
                self.end_at = Some(now + remaining);
                self.paused_remaining = None;
                vec![TimerEvent::StateChanged {
                    phase: Phase::Work,
                    count: self.state.session_count,
                }]
            }
            Phase::Work => Vec::new(),
        }
    }

    /// pause(): Work 走行中のみ動作。残り時間を保存し Paused へ。
    pub fn pause(&mut self, now: SystemTime) -> Vec<TimerEvent> {
        match self.state.phase {
            Phase::Work => {
                let remaining = match self.end_at {
                    Some(end) => end.duration_since(now).unwrap_or(Duration::ZERO),
                    None => Duration::from_secs(self.state.remaining_seconds as u64),
                };
                self.paused_remaining = Some(remaining);
                self.state.phase = Phase::Paused;
                self.state.remaining_seconds = remaining.as_secs() as u32;
                self.end_at = None;
                vec![TimerEvent::StateChanged {
                    phase: Phase::Paused,
                    count: self.state.session_count,
                }]
            }
            _ => Vec::new(),
        }
    }

    /// reset(): 全状態を初期化し、pending_config も commit する。
    /// session_count もゼロに戻す。
    pub fn reset(&mut self) -> Vec<TimerEvent> {
        self.commit_pending_config();
        let work_seconds = self.config.work_seconds;
        self.state = TimerState {
            phase: Phase::Idle,
            remaining_seconds: work_seconds,
            session_count: 0,
            current_duration_seconds: work_seconds,
        };
        self.end_at = None;
        self.paused_remaining = None;
        vec![TimerEvent::StateChanged {
            phase: Phase::Idle,
            count: 0,
        }]
    }

    /// skip(): Work 走行中のみ、現セッションを即完了扱いにして Idle へ。
    /// Completed と Idle への StateChanged を返す。
    pub fn skip(&mut self, _now: SystemTime) -> Vec<TimerEvent> {
        match self.state.phase {
            Phase::Work => {
                self.state.session_count = self.state.session_count.saturating_add(1);
                self.transition_to_idle();
                vec![
                    TimerEvent::Completed {
                        kind: SessionKind::Work,
                    },
                    TimerEvent::StateChanged {
                        phase: Phase::Idle,
                        count: self.state.session_count,
                    },
                ]
            }
            _ => Vec::new(),
        }
    }

    /// poll(now): ticker から定期的に呼ばれる。
    /// - Work で end_at に達したら Tick(0) + Completed + StateChanged(Idle) を返す
    /// - 走行中なら Tick(remaining) のみ
    /// - Idle / Paused は空 Vec
    pub fn poll(&mut self, now: SystemTime) -> Vec<TimerEvent> {
        match self.state.phase {
            Phase::Work => {
                let end_at = match self.end_at {
                    Some(e) => e,
                    None => return Vec::new(),
                };
                if now >= end_at {
                    self.state.remaining_seconds = 0;
                    self.state.session_count = self.state.session_count.saturating_add(1);
                    let mut events = Vec::with_capacity(3);
                    events.push(TimerEvent::Tick(0));
                    events.push(TimerEvent::Completed {
                        kind: SessionKind::Work,
                    });
                    self.transition_to_idle();
                    events.push(TimerEvent::StateChanged {
                        phase: Phase::Idle,
                        count: self.state.session_count,
                    });
                    events
                } else {
                    let remaining = end_at.duration_since(now).unwrap_or(Duration::ZERO);
                    let secs = remaining.as_secs() as u32;
                    self.state.remaining_seconds = secs;
                    vec![TimerEvent::Tick(secs)]
                }
            }
            _ => Vec::new(),
        }
    }

    /// 内部ヘルパ: Work 完了後に Idle 状態にリセット (session_count は維持)。
    /// pending_config はここでは commit せず、次の start() / reset() に持ち越す。
    fn transition_to_idle(&mut self) {
        let work_seconds = self.config.work_seconds;
        self.state.phase = Phase::Idle;
        self.state.remaining_seconds = work_seconds;
        self.state.current_duration_seconds = work_seconds;
        self.end_at = None;
        self.paused_remaining = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fast_config() -> TimerConfig {
        TimerConfig { work_seconds: 4 }
    }

    fn t0() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }

    #[test]
    fn initial_state_is_idle() {
        let m = TimerMachine::new(fast_config());
        let s = m.state();
        assert_eq!(s.phase, Phase::Idle);
        assert_eq!(s.session_count, 0);
        assert!(m.end_at.is_none());
        assert_eq!(s.current_duration_seconds, 4);
    }

    #[test]
    fn start_transitions_to_work() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        let events = m.start(now);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], TimerEvent::StateChanged { phase: Phase::Work, count: 0 }));
        let s = m.state();
        assert_eq!(s.phase, Phase::Work);
        assert_eq!(s.remaining_seconds, 4);
        assert_eq!(m.end_at, Some(now + Duration::from_secs(4)));
    }

    #[test]
    fn poll_emits_tick_with_decreasing_remaining() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        m.start(now);
        let events = m.poll(now + Duration::from_secs(1));
        assert!(matches!(events[0], TimerEvent::Tick(3)));
        let events = m.poll(now + Duration::from_secs(3));
        assert!(matches!(events[0], TimerEvent::Tick(1)));
        assert_eq!(m.state().remaining_seconds, 1);
    }

    #[test]
    fn work_completion_emits_tick0_completed_and_state_changed_idle() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        m.start(now);
        let events = m.poll(now + Duration::from_secs(4));
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], TimerEvent::Tick(0)));
        assert!(matches!(events[1], TimerEvent::Completed { kind: SessionKind::Work }));
        assert!(matches!(events[2], TimerEvent::StateChanged { phase: Phase::Idle, count: 1 }));
        let s = m.state();
        assert_eq!(s.phase, Phase::Idle);
        assert_eq!(s.session_count, 1);
        assert_eq!(s.remaining_seconds, 4);
    }

    #[test]
    fn pause_then_resume_preserves_remaining() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        m.start(now);
        let pause_now = now + Duration::from_secs(1);
        m.pause(pause_now);
        assert_eq!(m.state().phase, Phase::Paused);
        assert_eq!(m.state().remaining_seconds, 3);

        // 大量の wall-clock 経過後でも残りは保持
        let resume_now = pause_now + Duration::from_secs(60);
        let events = m.start(resume_now);
        assert!(matches!(events[0], TimerEvent::StateChanged { phase: Phase::Work, .. }));
        assert_eq!(m.state().remaining_seconds, 3);

        let events = m.poll(resume_now + Duration::from_secs(1));
        assert!(matches!(events[0], TimerEvent::Tick(2)));
    }

    #[test]
    fn skip_completes_current_work_and_returns_to_idle() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        m.start(now);
        let events = m.skip(now + Duration::from_secs(2));
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], TimerEvent::Completed { kind: SessionKind::Work }));
        assert!(matches!(events[1], TimerEvent::StateChanged { phase: Phase::Idle, count: 1 }));
        assert_eq!(m.state().phase, Phase::Idle);
        assert_eq!(m.state().session_count, 1);
    }

    #[test]
    fn reset_clears_everything() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        m.start(now);
        m.poll(now + Duration::from_secs(2));
        m.reset();
        assert_eq!(m.state().phase, Phase::Idle);
        assert_eq!(m.state().session_count, 0);
        assert_eq!(m.state().remaining_seconds, 4);
        assert!(m.end_at.is_none());
    }

    #[test]
    fn set_config_applies_at_next_start_not_current_session() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        m.start(now);
        // Work 走行中に config 変更
        m.set_config(TimerConfig { work_seconds: 10 });
        // 現セッションは旧 duration のまま
        assert_eq!(m.state().current_duration_seconds, 4);

        // 完了 → Idle
        m.poll(now + Duration::from_secs(4));
        // Idle 状態 (current_duration_seconds は旧 4 のまま、まだ commit されない)
        assert_eq!(m.state().phase, Phase::Idle);
        assert_eq!(m.state().current_duration_seconds, 4);

        // 次の start で commit される
        let events = m.start(now + Duration::from_secs(10));
        assert!(matches!(events[0], TimerEvent::StateChanged { phase: Phase::Work, .. }));
        assert_eq!(m.state().current_duration_seconds, 10);
        assert_eq!(m.state().remaining_seconds, 10);
    }

    #[test]
    fn reset_commits_pending_config() {
        let mut m = TimerMachine::new(fast_config());
        m.set_config(TimerConfig { work_seconds: 30 });
        m.reset();
        assert_eq!(m.state().current_duration_seconds, 30);
        assert_eq!(m.state().remaining_seconds, 30);
    }

    #[test]
    fn poll_handles_large_time_jump_as_single_completion() {
        // macOS スリープ復帰相当: 大きな時間ジャンプでも 1 回で完了処理
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        m.start(now);
        // 1 時間後を poll しても 1 セッション分しか完了扱いにならない
        let events = m.poll(now + Duration::from_secs(3600));
        assert_eq!(events.len(), 3);
        assert!(matches!(events[1], TimerEvent::Completed { kind: SessionKind::Work }));
        assert_eq!(m.state().session_count, 1);
        assert_eq!(m.state().phase, Phase::Idle);
    }
}
