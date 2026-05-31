//! TimerMachine: シンプルな作業セッションのステートマシン。
//! Tauri::* を一切 import せず、`cargo test` で単体テスト可能に保つ。
//!
//! MVP 設計:
//! - フェーズは Idle / Work / Paused の 3 つだけ
//! - 休憩フェーズは廃止 (Work 完了 → Idle に戻る)
//! - `set_config()` は次の `start()` / `reset()` 時に commit (E3: 実行中は据え置き)
//! - **カコン先行発火**: `Completed` イベントは `end_at - KAKON_LEAD_MS` の時点で
//!   発火する。フロントが受信してから倒れアニメ開始 → ちょうど `end_at` 時刻に
//!   筒が石にぶつかる視覚的タイミング。
//!
//! 仕様:
//! - docs/spec/backend/design.md `TimerMachine 設計` 節
//! - docs/spec/backend/tasks.md B2.1

#![allow(dead_code)]

use std::time::{Duration, SystemTime};

use crate::models::timer_state::TimerEvent;
use crate::models::{Phase, SessionKind, TimerConfig, TimerState};

/// フロントのカコン演出時間 (開始 → 倒れ → 戻り終わり)。
///
/// 実物のししおどしは「水が出て軽くなった後、反動で逆へ振れて石を打つ」瞬間にカコンが鳴る。
/// フロントの `playKakon` シーケンス: `-12° → +12°` (280ms) + `+12° → -12°` (780ms)
/// = 戻り終わりまで 1060ms。これに WebView の event 受信遅延と
/// requestAnimationFrame のフレーム境界誤差 (40ms 程度) を加えて 1100ms とする。
///
/// バック側はこの分だけ `end_at` より前に `Completed` を発火することで、
/// 戻り終わりがちょうど `end_at` (= タイマー 00:00) に一致する。
const KAKON_LEAD_MS: u64 = 1100;

pub struct TimerMachine {
    state: TimerState,
    config: TimerConfig,
    pending_config: Option<TimerConfig>,
    end_at: Option<SystemTime>,
    paused_remaining: Option<Duration>,
    /// 現セッションで `Completed` をすでに発火したか。先行発火を 1 回だけにするためのフラグ。
    /// `start()` / `reset()` / セッション遷移時にリセットする。
    completed_emitted: bool,
    /// Work 完了後に自動で次のセッションを開始するか (設定からの反映)。
    /// 自然完了 (poll() で end_at 到達) のときだけ働く。
    /// skip / reset / pause→start 経由では発動しない。
    loop_mode: bool,
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
            completed_emitted: false,
            loop_mode: false,
        }
    }

    /// セッション完了後に自動で次のセッションを開始するモードを切り替える。
    /// 設定変更時に commands/settings から呼ぶ想定。即時反映する。
    pub fn set_loop_mode(&mut self, enabled: bool) {
        self.loop_mode = enabled;
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
                self.completed_emitted = false;
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
                // resume 時に completed_emitted はリセットしない。
                // pause 前に既に発火していたなら次の end_at 到達でも 2 重発火しない。
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
        self.completed_emitted = false;
        vec![TimerEvent::StateChanged {
            phase: Phase::Idle,
            count: 0,
        }]
    }

    /// skip(): Work 走行中のみ、現セッションを即完了扱いにして Idle へ。
    /// Completed (まだ未発火なら) と Idle への StateChanged を返す。
    pub fn skip(&mut self, _now: SystemTime) -> Vec<TimerEvent> {
        match self.state.phase {
            Phase::Work => {
                self.state.session_count = self.state.session_count.saturating_add(1);
                let already_completed = self.completed_emitted;
                self.transition_to_idle();
                let mut events = Vec::with_capacity(2);
                if !already_completed {
                    events.push(TimerEvent::Completed {
                        kind: SessionKind::Work,
                    });
                }
                events.push(TimerEvent::StateChanged {
                    phase: Phase::Idle,
                    count: self.state.session_count,
                });
                events
            }
            _ => Vec::new(),
        }
    }

    /// poll(now): ticker から定期的に呼ばれる。
    ///
    /// 発火順:
    /// 1. `end_at - KAKON_LEAD_MS` を過ぎたら `Completed` を 1 回だけ発火 (倒れ開始)
    /// 2. `end_at` に達したら `Tick(0)` + `StateChanged(Idle)` を発火 (= タイマー 00:00)
    /// 3. それ以外は通常の `Tick(remaining)` のみ
    ///
    /// `Completed` と最終遷移が同じ poll で起きる場合 (大きな時間ジャンプ等) は
    /// 同じ呼び出しで両方を返す。
    pub fn poll(&mut self, now: SystemTime) -> Vec<TimerEvent> {
        match self.state.phase {
            Phase::Work => {
                let end_at = match self.end_at {
                    Some(e) => e,
                    None => return Vec::new(),
                };
                let lead = Duration::from_millis(KAKON_LEAD_MS);
                let pre_completion = end_at.checked_sub(lead).unwrap_or(end_at);

                if now >= end_at {
                    // 完了確定
                    self.state.remaining_seconds = 0;
                    self.state.session_count = self.state.session_count.saturating_add(1);
                    let mut events = Vec::with_capacity(4);
                    events.push(TimerEvent::Tick(0));
                    if !self.completed_emitted {
                        // poll が KAKON_LEAD_MS より遅れた場合 (例: スリープ復帰直後)
                        // ここでまとめて Completed も発火する
                        events.push(TimerEvent::Completed {
                            kind: SessionKind::Work,
                        });
                    }
                    self.transition_to_idle();
                    events.push(TimerEvent::StateChanged {
                        phase: Phase::Idle,
                        count: self.state.session_count,
                    });
                    self.completed_emitted = false;

                    // ループモードなら次の Work セッションを即時開始
                    // (自然完了経由のみ。skip/reset はループしない)
                    if self.loop_mode {
                        events.extend(self.start(now));
                    }
                    events
                } else if !self.completed_emitted && now >= pre_completion {
                    // 倒れアニメ開始タイミング: end_at の 280ms 前
                    let remaining = end_at.duration_since(now).unwrap_or(Duration::ZERO);
                    let secs = remaining.as_secs() as u32;
                    self.state.remaining_seconds = secs;
                    self.completed_emitted = true;
                    vec![
                        TimerEvent::Tick(secs),
                        TimerEvent::Completed {
                            kind: SessionKind::Work,
                        },
                    ]
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
        self.completed_emitted = false;
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
    fn completed_fires_before_end_at_by_kakon_lead_ms() {
        // end_at - 280ms の時点で Completed が発火する (Tick とセットで)
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        m.start(now);
        // 280ms 前ぴったり
        let pre = now + Duration::from_secs(4) - Duration::from_millis(KAKON_LEAD_MS);
        let events = m.poll(pre);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], TimerEvent::Tick(_)));
        assert!(matches!(events[1], TimerEvent::Completed { kind: SessionKind::Work }));
        // 状態は Work のまま (end_at にはまだ到達していない)
        assert_eq!(m.state().phase, Phase::Work);
    }

    #[test]
    fn completed_is_emitted_only_once_then_state_changed_at_end_at() {
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        m.start(now);
        // 280ms 前で Completed 発火
        let pre = now + Duration::from_secs(4) - Duration::from_millis(KAKON_LEAD_MS);
        let _ = m.poll(pre);
        // 100ms 後 (まだ end_at 未到達) は Tick のみ、Completed は再発火しない
        let mid = pre + Duration::from_millis(100);
        let events = m.poll(mid);
        assert!(events.iter().all(|e| !matches!(e, TimerEvent::Completed { .. })));
        // end_at に到達したら Tick(0) + StateChanged(Idle)、Completed は出ない
        let end = now + Duration::from_secs(4);
        let events = m.poll(end);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], TimerEvent::Tick(0)));
        assert!(matches!(events[1], TimerEvent::StateChanged { phase: Phase::Idle, count: 1 }));
        let s = m.state();
        assert_eq!(s.phase, Phase::Idle);
        assert_eq!(s.session_count, 1);
        assert_eq!(s.remaining_seconds, 4);
    }

    #[test]
    fn loop_mode_auto_starts_next_session_after_natural_completion() {
        // loop_mode=true なら end_at 到達時に Idle 遷移直後 → Work 再開する
        let mut m = TimerMachine::new(fast_config());
        m.set_loop_mode(true);
        let now = t0();
        m.start(now);
        // 先行 Completed を 280ms 前に発火させてから終端へ進める (実運用と同じ流れ)
        let pre = now + Duration::from_secs(4) - Duration::from_millis(KAKON_LEAD_MS);
        m.poll(pre);
        let events = m.poll(now + Duration::from_secs(4));
        // Tick(0) + StateChanged(Idle) + StateChanged(Work) の 3 件
        // Completed は先行で出ているのでここには出ない
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], TimerEvent::Tick(0)));
        assert!(matches!(events[1], TimerEvent::StateChanged { phase: Phase::Idle, count: 1 }));
        assert!(matches!(events[2], TimerEvent::StateChanged { phase: Phase::Work, count: 1 }));
        // 次セッションが Work で走行中
        assert_eq!(m.state().phase, Phase::Work);
        assert_eq!(m.state().remaining_seconds, 4);
    }

    #[test]
    fn loop_mode_does_not_auto_start_after_skip() {
        // skip 経由は loop_mode でも自動再開しない (ユーザー意図の中断)
        let mut m = TimerMachine::new(fast_config());
        m.set_loop_mode(true);
        let now = t0();
        m.start(now);
        let events = m.skip(now + Duration::from_secs(1));
        // Completed + StateChanged(Idle) のみ、Work 再開なし
        assert!(events.iter().all(|e| !matches!(e, TimerEvent::StateChanged { phase: Phase::Work, .. })));
        assert_eq!(m.state().phase, Phase::Idle);
    }

    #[test]
    fn loop_mode_off_returns_to_idle_after_completion() {
        // loop_mode=false (デフォルト) は従来通り Idle に戻って止まる
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        m.start(now);
        let events = m.poll(now + Duration::from_secs(4));
        assert!(events.iter().all(|e| !matches!(e, TimerEvent::StateChanged { phase: Phase::Work, .. })));
        assert_eq!(m.state().phase, Phase::Idle);
    }

    #[test]
    fn large_time_jump_emits_completed_and_state_changed_in_one_poll() {
        // Completed をまだ出していない状態で end_at を大きく超えた場合
        // (スリープ復帰相当)、同じ poll で Tick(0) + Completed + StateChanged を返す
        let mut m = TimerMachine::new(fast_config());
        let now = t0();
        m.start(now);
        let events = m.poll(now + Duration::from_secs(3600));
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], TimerEvent::Tick(0)));
        assert!(matches!(events[1], TimerEvent::Completed { kind: SessionKind::Work }));
        assert!(matches!(events[2], TimerEvent::StateChanged { phase: Phase::Idle, count: 1 }));
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

}
