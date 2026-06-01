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

/// フロントの **アニメ開始** を `end_at` の何 ms 前に発火するか。
/// フロントの `playKakon` は `-12° → +12°` (1000ms) + `+12° → -12°` (500ms) = 1500ms。
///
/// 数値を `1500` より小さくすると、アニメ開始が遅れて戻り終わりが `end_at` より
/// 後ろにずれる (= カコン音の後で筒の動きが続く見え方になる)。
/// 現状 1200ms で「カコン音 → 300ms 後にアニメ終了」のタイミングを意図している。
const KAKON_LEAD_MS: u64 = 1200;

/// **カコン音** を `end_at` の何 ms 前に発火するか。
/// アニメ戻り終わり (= end_at) より少し早めに音を鳴らすことで、
/// rodio の再生開始遅延や OS audio buffer を吸収し、聴覚的に石を打つ瞬間と
/// 視覚的な戻り終わりが揃って感じられるようにする。
const KAKON_AUDIO_LEAD_MS: u64 = 100;

pub struct TimerMachine {
    state: TimerState,
    config: TimerConfig,
    pending_config: Option<TimerConfig>,
    end_at: Option<SystemTime>,
    paused_remaining: Option<Duration>,
    /// 現セッションで `Completed` をすでに発火したか。先行発火を 1 回だけにするためのフラグ。
    /// `start()` / `reset()` / セッション遷移時にリセットする。
    completed_emitted: bool,
    /// 現セッションで `PlayKakonAudio` をすでに発火したか。1 回だけ鳴らすためのフラグ。
    audio_emitted: bool,
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
            audio_emitted: false,
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
                self.audio_emitted = false;
                let work_seconds = self.config.work_seconds;
                self.state.current_duration_seconds = work_seconds;
                self.state.remaining_seconds = work_seconds;
                self.state.phase = Phase::Work;
                self.end_at = if work_seconds > 0 {
                    Some(now + Duration::from_secs(work_seconds as u64))
                } else {
                    None
                };
                // StateChanged + 初期 Tick(work_seconds) を同時発火。
                // 特に loop_mode 経路では `timer_start` コマンド戻り値の applyState を
                // 経由しないので、Tick を出さないと frontend の remaining_seconds が
                // 古い値 (= 0) のまま新セッションが「00:00」表示で始まってしまう。
                vec![
                    TimerEvent::StateChanged {
                        phase: Phase::Work,
                        count: self.state.session_count,
                    },
                    TimerEvent::Tick(work_seconds),
                ]
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
        self.audio_emitted = false;
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
                let audio_lead = Duration::from_millis(KAKON_AUDIO_LEAD_MS);
                let pre_audio = end_at.checked_sub(audio_lead).unwrap_or(end_at);

                if now >= end_at {
                    // 完了確定
                    self.state.remaining_seconds = 0;
                    self.state.session_count = self.state.session_count.saturating_add(1);
                    let mut events = Vec::with_capacity(5);
                    events.push(TimerEvent::Tick(0));
                    if !self.completed_emitted {
                        // poll が KAKON_LEAD_MS より遅れた場合 (例: スリープ復帰直後)
                        // ここでまとめて Completed も発火する
                        events.push(TimerEvent::Completed {
                            kind: SessionKind::Work,
                        });
                    }
                    if !self.audio_emitted {
                        // ジャンプ経路でカコン音も未再生なら、ここで一緒に発火
                        events.push(TimerEvent::PlayKakonAudio);
                    }
                    self.transition_to_idle();
                    events.push(TimerEvent::StateChanged {
                        phase: Phase::Idle,
                        count: self.state.session_count,
                    });
                    self.completed_emitted = false;
                    self.audio_emitted = false;

                    // ループモードなら次の Work セッションを即時開始
                    // (自然完了経由のみ。skip/reset はループしない)
                    if self.loop_mode {
                        events.extend(self.start(now));
                    }
                    events
                } else if !self.audio_emitted && now >= pre_audio {
                    // カコン音タイミング: end_at の KAKON_AUDIO_LEAD_MS (100ms) 前。
                    // 視覚 (アニメ戻り終わり) より少し早めに音を鳴らして同期感を出す。
                    self.audio_emitted = true;
                    let remaining = end_at.duration_since(now).unwrap_or(Duration::ZERO);
                    let secs = remaining.as_secs() as u32;
                    self.state.remaining_seconds = secs;
                    let mut events = Vec::with_capacity(2);
                    if secs > 0 {
                        events.push(TimerEvent::Tick(secs));
                    }
                    events.push(TimerEvent::PlayKakonAudio);
                    events
                } else if !self.completed_emitted && now >= pre_completion {
                    // 倒れアニメ開始タイミング: end_at の KAKON_LEAD_MS 前
                    let remaining = end_at.duration_since(now).unwrap_or(Duration::ZERO);
                    let secs = remaining.as_secs() as u32;
                    let remaining_ms = remaining.as_millis();
                    log::info!(
                        "[machine] pre-completion fired: remaining={}ms (target lead={}ms), tick_secs={}",
                        remaining_ms, KAKON_LEAD_MS, secs
                    );
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
                    // `Tick(0)` は `now >= end_at` 経路の役割 (1 回だけ発火)。
                    // pre_completion 経路 (Completed 先行発火) の後、end_at 到達までの
                    // 1 秒間は `remaining < 1s` → `secs = 0` になるが、ここで
                    // `Tick(0)` を返すと 50ms poll 毎に大量発火してカコン音が
                    // debounce すり抜けで連発する。secs == 0 のときは何も発火しない。
                    if secs == 0 {
                        Vec::new()
                    } else {
                        vec![TimerEvent::Tick(secs)]
                    }
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
        self.audio_emitted = false;
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
        // StateChanged(Work) + 初期 Tick(work_seconds) の 2 件
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], TimerEvent::StateChanged { phase: Phase::Work, count: 0 }));
        assert!(matches!(events[1], TimerEvent::Tick(4)));
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
        // end_at に到達したら Tick(0) + PlayKakonAudio + StateChanged(Idle)、Completed は出ない
        // (PlayKakonAudio は end_at - 100ms 経路で出るが、pre と end の間で poll してないので
        // ここでまとめて発火する)
        let end = now + Duration::from_secs(4);
        let events = m.poll(end);
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], TimerEvent::Tick(0)));
        assert!(matches!(events[1], TimerEvent::PlayKakonAudio));
        assert!(matches!(events[2], TimerEvent::StateChanged { phase: Phase::Idle, count: 1 }));
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
        // Tick(0) + PlayKakonAudio + StateChanged(Idle) + StateChanged(Work) + Tick(4) の 5 件
        // Completed は先行で出ているのでここには出ない
        // 最後の Tick(4) は loop_mode の自動 start() で発火される初期 Tick
        assert_eq!(events.len(), 5);
        assert!(matches!(events[0], TimerEvent::Tick(0)));
        assert!(matches!(events[1], TimerEvent::PlayKakonAudio));
        assert!(matches!(events[2], TimerEvent::StateChanged { phase: Phase::Idle, count: 1 }));
        assert!(matches!(events[3], TimerEvent::StateChanged { phase: Phase::Work, count: 1 }));
        assert!(matches!(events[4], TimerEvent::Tick(4)));
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
        // Tick(0) + Completed + PlayKakonAudio + StateChanged(Idle) の 4 件
        assert_eq!(events.len(), 4);
        assert!(matches!(events[0], TimerEvent::Tick(0)));
        assert!(matches!(events[1], TimerEvent::Completed { kind: SessionKind::Work }));
        assert!(matches!(events[2], TimerEvent::PlayKakonAudio));
        assert!(matches!(events[3], TimerEvent::StateChanged { phase: Phase::Idle, count: 1 }));
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
