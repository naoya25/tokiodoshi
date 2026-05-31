use serde::{Deserialize, Serialize};

/// TimerMachine の現フェーズ。
/// フロント `src/lib/types/index.ts::Phase` と同期。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    Work,
    ShortBreak,
    LongBreak,
    Paused,
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Idle
    }
}

/// 履歴に保存される種類。Phase の `Paused` / `Idle` は含まない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    Work,
    ShortBreak,
    LongBreak,
}

/// フロントに返す現在のタイマー状態のスナップショット。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerState {
    pub phase: Phase,
    pub remaining_seconds: u32,
    pub session_count: u32,
    pub current_duration_seconds: u32,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            remaining_seconds: 1500,
            session_count: 0,
            current_duration_seconds: 1500,
        }
    }
}

/// タイマー長設定 (秒単位)。Settings から派生する。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TimerConfig {
    pub work_seconds: u32,
    pub short_break_seconds: u32,
    pub long_break_seconds: u32,
    pub sessions_until_long_break: u32,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            work_seconds: 1500,
            short_break_seconds: 300,
            long_break_seconds: 900,
            sessions_until_long_break: 4,
        }
    }
}
