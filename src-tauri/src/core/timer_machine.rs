//! TimerMachine: ポモドーロサイクルの純Rustステートマシン。
//! Tauri::* を一切 import せず、`cargo test` で単体テスト可能に保つ。
//!
//! 仕様:
//! - docs/spec/backend/design.md `TimerMachine 設計` 節
//! - docs/spec/backend/tasks.md B2.1 (10 ケーステスト)
//! - docs/requirements.md v0.3 §5.3 原則 7
//!
//! 設計判断:
//! - `TimerEvent` は `crate::models::timer_state::TimerEvent` を使用。
//!   `Tick(u32)` / `StateChanged { phase, count }` / `Completed { kind }` の 3 variant
//!   (内部 enum で Serialize 不要、ticker 側で emit payload に変換)。
//! - `now: SystemTime` を全状態変更 API の引数にし、Instant を使わない
//!   (macOS スリープ復帰でズレないため)。
//! - `pending_config` は `set_config()` で積み、次の `start()` で commit する
//!   (E3: 現セッションは変更前 duration で完走)。
//! - `paused_remaining` / `prev_phase` で pause/resume の戻り先を保持する。
//! - `completed_work_sessions` は long_break 判定のための内部カウンタで、
//!   `TimerState::session_count` はフロントに見える「完了 work セッション数」と同じ値を反映。

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
    prev_phase: Option<Phase>,
    completed_work_sessions: u32,
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
            prev_phase: None,
            completed_work_sessions: 0,
        }
    }

    /// 現在の状態スナップショット。
    pub fn state(&self) -> TimerState {
        self.state.clone()
    }

    /// 次セッションから有効になる config を積む。現セッションには影響しない (E3)。
    pub fn set_config(&mut self, c: TimerConfig) {
        self.pending_config = Some(c);
    }

    /// start():
    /// - Idle → Work で開始 (`pending_config` があれば commit)
    /// - Paused → 直前のフェーズに resume (残り時間を保持)
    /// - その他 (走行中) → 何もしない (空 Vec)
    pub fn start(&mut self, now: SystemTime) -> Vec<TimerEvent> {
        match self.state.phase {
            Phase::Idle => {
                // 次セッションから適用される config を commit
                if let Some(pending) = self.pending_config.take() {
                    self.config = pending;
                }
                self.enter_phase(Phase::Work, now)
            }
            Phase::Paused => {
                // resume: prev_phase に戻り、paused_remaining 分だけ end_at を伸ばす
                let resume_phase = self.prev_phase.unwrap_or(Phase::Work);
                let remaining = self.paused_remaining.unwrap_or_else(|| {
                    Duration::from_secs(self.state.remaining_seconds as u64)
                });
                self.state.phase = resume_phase;
                self.state.remaining_seconds = remaining.as_secs() as u32;
                self.end_at = Some(now + remaining);
                self.paused_remaining = None;
                self.prev_phase = None;
                vec![TimerEvent::StateChanged {
                    phase: resume_phase,
                    count: self.state.session_count,
                }]
            }
            _ => Vec::new(),
        }
    }

    /// pause(): 走行中フェーズ (Work/ShortBreak/LongBreak) でのみ動作。
    /// 残り時間を `paused_remaining` に保存し、Phase::Paused へ。
    pub fn pause(&mut self, now: SystemTime) -> Vec<TimerEvent> {
        match self.state.phase {
            Phase::Work | Phase::ShortBreak | Phase::LongBreak => {
                let remaining = match self.end_at {
                    Some(end) => end.duration_since(now).unwrap_or(Duration::ZERO),
                    None => Duration::from_secs(self.state.remaining_seconds as u64),
                };
                self.prev_phase = Some(self.state.phase);
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

    /// reset(): 全状態を初期化。session_count = 0, Idle, end_at=None。
    pub fn reset(&mut self) -> Vec<TimerEvent> {
        let work_seconds = self.config.work_seconds;
        self.state = TimerState {
            phase: Phase::Idle,
            remaining_seconds: work_seconds,
            session_count: 0,
            current_duration_seconds: work_seconds,
        };
        self.end_at = None;
        self.paused_remaining = None;
        self.prev_phase = None;
        self.completed_work_sessions = 0;
        vec![TimerEvent::StateChanged {
            phase: Phase::Idle,
            count: 0,
        }]
    }

    /// skip(): 現フェーズを即完了扱いし、次フェーズへ遷移。
    /// Idle / Paused では何もしない。
    pub fn skip(&mut self, now: SystemTime) -> Vec<TimerEvent> {
        match self.state.phase {
            Phase::Work | Phase::ShortBreak | Phase::LongBreak => {
                let kind = phase_to_session_kind(self.state.phase);
                let next = self.advance_after_completion(kind);
                let mut events = vec![TimerEvent::Completed { kind }];
                let state_events = self.enter_phase(next, now);
                events.extend(state_events);
                events
            }
            _ => Vec::new(),
        }
    }

    /// poll(now): ticker から定期的に呼ばれる。
    /// - 走行中フェーズで `now >= end_at` なら 1 回だけセッション完了処理を行う
    ///   (Tick(0) + Completed + 次の StateChanged を 1 度に返す)
    /// - それ以外は Tick(remaining) のみ
    /// - Paused / Idle は空 Vec
    pub fn poll(&mut self, now: SystemTime) -> Vec<TimerEvent> {
        match self.state.phase {
            Phase::Work | Phase::ShortBreak | Phase::LongBreak => {
                let end_at = match self.end_at {
                    Some(e) => e,
                    None => return Vec::new(),
                };
                if now >= end_at {
                    // スリープ復帰相当: 大きな時間ジャンプでも 1 回で完了処理 + 次フェーズ遷移
                    self.state.remaining_seconds = 0;
                    let mut events = Vec::with_capacity(3);
                    events.push(TimerEvent::Tick(0));
                    let kind = phase_to_session_kind(self.state.phase);
                    events.push(TimerEvent::Completed { kind });
                    let next = self.advance_after_completion(kind);
                    events.extend(self.enter_phase(next, now));
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

    // ----- 内部ヘルパ -----

    /// 走行フェーズに遷移する共通処理。end_at と state を再計算し、StateChanged を返す。
    fn enter_phase(&mut self, phase: Phase, now: SystemTime) -> Vec<TimerEvent> {
        let duration_seconds = match phase {
            Phase::Work => self.config.work_seconds,
            Phase::ShortBreak => self.config.short_break_seconds,
            Phase::LongBreak => self.config.long_break_seconds,
            Phase::Idle | Phase::Paused => 0,
        };
        self.state.phase = phase;
        self.state.remaining_seconds = duration_seconds;
        self.state.current_duration_seconds = duration_seconds;
        self.end_at = if duration_seconds > 0 {
            Some(now + Duration::from_secs(duration_seconds as u64))
        } else {
            None
        };
        vec![TimerEvent::StateChanged {
            phase,
            count: self.state.session_count,
        }]
    }

    /// 完了したフェーズの kind を受け取り、次フェーズを決定する。
    /// - Work 完了 → completed_work_sessions += 1 → ShortBreak or LongBreak
    /// - ShortBreak / LongBreak 完了 → Work
    fn advance_after_completion(&mut self, just_completed: SessionKind) -> Phase {
        match just_completed {
            SessionKind::Work => {
                self.completed_work_sessions += 1;
                self.state.session_count = self.completed_work_sessions;
                let cycle = self.config.sessions_until_long_break;
                if cycle > 0 && self.completed_work_sessions % cycle == 0 {
                    Phase::LongBreak
                } else {
                    Phase::ShortBreak
                }
            }
            SessionKind::ShortBreak | SessionKind::LongBreak => Phase::Work,
        }
    }
}

fn phase_to_session_kind(phase: Phase) -> SessionKind {
    match phase {
        Phase::Work => SessionKind::Work,
        Phase::ShortBreak => SessionKind::ShortBreak,
        Phase::LongBreak => SessionKind::LongBreak,
        // 呼び出し側で Idle/Paused は除外する前提
        Phase::Idle | Phase::Paused => SessionKind::Work,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用の短い config (work=4, short=2, long=3, cycle=4)。
    /// セッション完了の境界が短時間で出るようにする。
    fn fast_config() -> TimerConfig {
        TimerConfig {
            work_seconds: 4,
            short_break_seconds: 2,
            long_break_seconds: 3,
            sessions_until_long_break: 4,
        }
    }

    fn t0() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }

    // 1. 初期状態は Idle、session_count=0、end_at=None
    #[test]
    fn case01_initial_state_is_idle() {
        let m = TimerMachine::new(fast_config());
        let s = m.state();
        assert_eq!(s.phase, Phase::Idle);
        assert_eq!(s.session_count, 0);
        assert!(m.end_at.is_none());
        assert_eq!(s.current_duration_seconds, 4);
    }

    // 2. start() で Work に遷移、end_at 設定、StateChanged 返却、remaining=work_seconds
    #[test]
    fn case02_start_transitions_to_work() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        let events = m.start(now);
        assert_eq!(events.len(), 1);
        match &events[0] {
            TimerEvent::StateChanged {
                phase,
                count,
            } => {
                assert_eq!(*phase, Phase::Work);
                assert_eq!(*count, 0);
            }
            other => panic!("expected StateChanged, got {:?}", other),
        }
        let s = m.state();
        assert_eq!(s.phase, Phase::Work);
        assert_eq!(s.remaining_seconds, 4);
        assert_eq!(s.current_duration_seconds, 4);
        assert_eq!(m.end_at, Some(now + Duration::from_secs(4)));
    }

    // 3. poll() で時間経過に応じて Tick の remaining_seconds が減る
    #[test]
    fn case03_poll_emits_tick_with_decreasing_remaining() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        let _ = m.start(now);
        let events = m.poll(now + Duration::from_secs(1));
        assert_eq!(events.len(), 1);
        match &events[0] {
            TimerEvent::Tick(remaining_seconds) => {
                assert_eq!(*remaining_seconds, 3);
            }
            other => panic!("expected Tick, got {:?}", other),
        }
        let events = m.poll(now + Duration::from_secs(3));
        match &events[0] {
            TimerEvent::Tick(remaining_seconds) => assert_eq!(*remaining_seconds, 1),
            other => panic!("expected Tick, got {:?}", other),
        }
        assert_eq!(m.state().remaining_seconds, 1);
    }

    // 4. Work 完了 → [Tick(0), Completed{Work}, StateChanged{ShortBreak, 1}]
    #[test]
    fn case04_work_completion_emits_three_events_in_order() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        let _ = m.start(now);
        let events = m.poll(now + Duration::from_secs(4));
        assert_eq!(events.len(), 3);
        match &events[0] {
            TimerEvent::Tick(remaining_seconds) => assert_eq!(*remaining_seconds, 0),
            other => panic!("expected Tick(0), got {:?}", other),
        }
        match &events[1] {
            TimerEvent::Completed { kind } => assert_eq!(*kind, SessionKind::Work),
            other => panic!("expected Completed(Work), got {:?}", other),
        }
        match &events[2] {
            TimerEvent::StateChanged {
                phase,
                count,
            } => {
                assert_eq!(*phase, Phase::ShortBreak);
                assert_eq!(*count, 1);
            }
            other => panic!("expected StateChanged(ShortBreak,1), got {:?}", other),
        }
        let s = m.state();
        assert_eq!(s.phase, Phase::ShortBreak);
        assert_eq!(s.session_count, 1);
        assert_eq!(s.current_duration_seconds, 2);
        assert_eq!(s.remaining_seconds, 2);
    }

    // 5. 4 セッション目の Work 完了で次が LongBreak
    #[test]
    fn case05_fourth_work_completion_goes_to_long_break() {
        let mut m = TimerMachine::new(fast_config());
        let mut now = t0();
        // work1 → short1
        let _ = m.start(now);
        let _ = m.poll(now + Duration::from_secs(4));
        now += Duration::from_secs(4);
        // short1 → work2
        let _ = m.poll(now + Duration::from_secs(2));
        now += Duration::from_secs(2);
        assert_eq!(m.state().phase, Phase::Work);
        // work2 → short2
        let _ = m.poll(now + Duration::from_secs(4));
        now += Duration::from_secs(4);
        assert_eq!(m.state().phase, Phase::ShortBreak);
        // short2 → work3
        let _ = m.poll(now + Duration::from_secs(2));
        now += Duration::from_secs(2);
        // work3 → short3
        let _ = m.poll(now + Duration::from_secs(4));
        now += Duration::from_secs(4);
        assert_eq!(m.state().phase, Phase::ShortBreak);
        // short3 → work4
        let _ = m.poll(now + Duration::from_secs(2));
        now += Duration::from_secs(2);
        assert_eq!(m.state().phase, Phase::Work);
        // work4 完了 → LongBreak
        let events = m.poll(now + Duration::from_secs(4));
        assert_eq!(events.len(), 3);
        match &events[2] {
            TimerEvent::StateChanged {
                phase,
                count,
            } => {
                assert_eq!(*phase, Phase::LongBreak);
                assert_eq!(*count, 4);
            }
            other => panic!("expected LongBreak, got {:?}", other),
        }
        assert_eq!(m.state().phase, Phase::LongBreak);
        assert_eq!(m.state().session_count, 4);
        assert_eq!(m.state().current_duration_seconds, 3);
    }

    // 6. pause() → start() (resume) で残り時間が保持される
    #[test]
    fn case06_pause_then_resume_preserves_remaining() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        let _ = m.start(now);
        // 1 秒経過したところで pause
        let pause_now = now + Duration::from_secs(1);
        let events = m.pause(pause_now);
        assert_eq!(events.len(), 1);
        match &events[0] {
            TimerEvent::StateChanged { phase, .. } => assert_eq!(*phase, Phase::Paused),
            other => panic!("expected Paused, got {:?}", other),
        }
        assert_eq!(m.state().phase, Phase::Paused);
        assert_eq!(m.state().remaining_seconds, 3);

        // 大量の wall-clock が経過しても resume 時の残りは保持される
        let resume_now = pause_now + Duration::from_secs(60);
        let events = m.start(resume_now);
        assert_eq!(events.len(), 1);
        match &events[0] {
            TimerEvent::StateChanged { phase, .. } => assert_eq!(*phase, Phase::Work),
            other => panic!("expected Work resume, got {:?}", other),
        }
        assert_eq!(m.state().phase, Phase::Work);
        assert_eq!(m.state().remaining_seconds, 3);
        // resume 後の poll で残りが減る
        let events = m.poll(resume_now + Duration::from_secs(1));
        match &events[0] {
            TimerEvent::Tick(remaining_seconds) => assert_eq!(*remaining_seconds, 2),
            other => panic!("expected Tick(2), got {:?}", other),
        }
    }

    // 7. skip() で現フェーズ即完了 → 次フェーズへ (Completed + StateChanged)
    #[test]
    fn case07_skip_completes_current_and_advances() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        let _ = m.start(now);
        // すぐ skip
        let events = m.skip(now + Duration::from_secs(1));
        assert_eq!(events.len(), 2);
        match &events[0] {
            TimerEvent::Completed { kind } => assert_eq!(*kind, SessionKind::Work),
            other => panic!("expected Completed(Work), got {:?}", other),
        }
        match &events[1] {
            TimerEvent::StateChanged {
                phase,
                count,
            } => {
                assert_eq!(*phase, Phase::ShortBreak);
                assert_eq!(*count, 1);
            }
            other => panic!("expected StateChanged(ShortBreak,1), got {:?}", other),
        }
        assert_eq!(m.state().phase, Phase::ShortBreak);
        assert_eq!(m.state().session_count, 1);
    }

    // 8. reset() で全状態が初期化される (Idle, count=0)
    #[test]
    fn case08_reset_returns_to_idle() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        let _ = m.start(now);
        let _ = m.poll(now + Duration::from_secs(4)); // work1 完了 → ShortBreak, count=1
        assert_eq!(m.state().session_count, 1);
        let events = m.reset();
        assert_eq!(events.len(), 1);
        match &events[0] {
            TimerEvent::StateChanged {
                phase,
                count,
            } => {
                assert_eq!(*phase, Phase::Idle);
                assert_eq!(*count, 0);
            }
            other => panic!("expected StateChanged(Idle,0), got {:?}", other),
        }
        let s = m.state();
        assert_eq!(s.phase, Phase::Idle);
        assert_eq!(s.session_count, 0);
        assert_eq!(s.remaining_seconds, 4);
        assert!(m.end_at.is_none());
        assert_eq!(m.completed_work_sessions, 0);
    }

    // 9. set_config() は現セッションに影響しない、次の start() から有効
    #[test]
    fn case09_set_config_applies_on_next_session() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        let _ = m.start(now);
        // 走行中に新 config を積む (work=10, short=5)
        let new_config = TimerConfig {
            work_seconds: 10,
            short_break_seconds: 5,
            long_break_seconds: 8,
            sessions_until_long_break: 4,
        };
        m.set_config(new_config);
        // 現セッションは変更前 (4 秒) で完走する
        assert_eq!(m.state().current_duration_seconds, 4);
        let events = m.poll(now + Duration::from_secs(4));
        // ShortBreak に入るが、現 config (short=2) のまま (pending は work 完了時にはまだ commit されない)
        // 次の Work start で commit される
        match &events[2] {
            TimerEvent::StateChanged { phase, .. } => assert_eq!(*phase, Phase::ShortBreak),
            other => panic!("expected ShortBreak, got {:?}", other),
        }
        assert_eq!(m.state().current_duration_seconds, 2);

        // Idle に戻し、改めて start() すると pending が commit される
        let _ = m.reset();
        let _ = m.start(now + Duration::from_secs(100));
        assert_eq!(m.state().current_duration_seconds, 10);
    }

    // 10. スリープ復帰相当: end_at を 30 秒過ぎた poll で 1 回で [Tick(0), Completed, StateChanged]
    #[test]
    fn case10_sleep_resume_emits_completion_in_single_poll() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        let _ = m.start(now);
        // 4 秒経過予定だったが、スリープから戻って 34 秒後に初めて poll
        let resume_now = now + Duration::from_secs(34);
        let events = m.poll(resume_now);
        assert_eq!(events.len(), 3);
        match &events[0] {
            TimerEvent::Tick(remaining_seconds) => assert_eq!(*remaining_seconds, 0),
            other => panic!("expected Tick(0), got {:?}", other),
        }
        match &events[1] {
            TimerEvent::Completed { kind } => assert_eq!(*kind, SessionKind::Work),
            other => panic!("expected Completed(Work), got {:?}", other),
        }
        match &events[2] {
            TimerEvent::StateChanged {
                phase,
                count,
            } => {
                assert_eq!(*phase, Phase::ShortBreak);
                assert_eq!(*count, 1);
            }
            other => panic!("expected ShortBreak, got {:?}", other),
        }
        // 次の end_at は resume_now を基準に計算されている
        assert_eq!(m.end_at, Some(resume_now + Duration::from_secs(2)));
        // 連続完了の防止: 同じ poll で 1 セッションだけ完了する
        // (resume_now から 2 秒経つ前に再度 poll を呼んでも完了しない)
        let events = m.poll(resume_now + Duration::from_secs(1));
        assert_eq!(events.len(), 1);
        match &events[0] {
            TimerEvent::Tick(remaining_seconds) => assert_eq!(*remaining_seconds, 1),
            other => panic!("expected Tick(1), got {:?}", other),
        }
    }

    // 11. フルサイクル統合: Idle → Work → Short → Work → Short → Work → Short → Work → Long → (Work)
    #[test]
    fn case11_full_cycle_integration() {
        let mut m = TimerMachine::new(fast_config());
        let mut now = t0();

        let phases_sequence = [
            (Phase::Work, 4, 1),
            (Phase::ShortBreak, 2, 1),
            (Phase::Work, 4, 2),
            (Phase::ShortBreak, 2, 2),
            (Phase::Work, 4, 3),
            (Phase::ShortBreak, 2, 3),
            (Phase::Work, 4, 4),
            (Phase::LongBreak, 3, 4),
        ];

        // start で Work に入る
        let _ = m.start(now);
        assert_eq!(m.state().phase, Phase::Work);

        for (i, (expected_phase, duration, expected_count_after)) in
            phases_sequence.iter().enumerate()
        {
            // 現フェーズが期待通りであることを確認
            assert_eq!(m.state().phase, *expected_phase, "step {}", i);
            // 期間ぶんを進めて完了させる
            now += Duration::from_secs(*duration as u64);
            let events = m.poll(now);
            assert_eq!(events.len(), 3, "step {}: events", i);
            // work 完了時のみ session_count が増える
            if matches!(*expected_phase, Phase::Work) {
                assert_eq!(m.state().session_count, *expected_count_after, "step {}", i);
            }
        }
        // LongBreak 完了後は Work に戻る
        assert_eq!(m.state().phase, Phase::Work);
        // session_count は 4 のまま (work4 完了時の値)
        assert_eq!(m.state().session_count, 4);
    }

    // 12. pause/resume が break 中でも動く
    #[test]
    fn case12_pause_during_break_then_resume() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        let _ = m.start(now);
        let _ = m.poll(now + Duration::from_secs(4)); // Work → ShortBreak
        let break_start = now + Duration::from_secs(4);
        // 1 秒経過したところで pause
        let _ = m.pause(break_start + Duration::from_secs(1));
        assert_eq!(m.state().phase, Phase::Paused);
        assert_eq!(m.state().remaining_seconds, 1);
        // resume
        let resume_now = break_start + Duration::from_secs(60);
        let events = m.start(resume_now);
        match &events[0] {
            TimerEvent::StateChanged { phase, .. } => assert_eq!(*phase, Phase::ShortBreak),
            other => panic!("expected ShortBreak resume, got {:?}", other),
        }
        assert_eq!(m.state().remaining_seconds, 1);
    }
}
