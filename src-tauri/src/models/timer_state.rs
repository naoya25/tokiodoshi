use serde::{Deserialize, Serialize};

/// TimerMachine の現フェーズ。
/// フロント `src/lib/types/index.ts::Phase` と同期 (`"idle"`, `"work"`, `"short_break"`, `"long_break"`, `"paused"`)。
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
/// フロント `SessionKind` と同期 (`"work"`, `"short_break"`, `"long_break"`)。
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    Work,
    ShortBreak,
    LongBreak,
}

/// フロントに返す現在のタイマー状態のスナップショット。
/// `timer_*` コマンドの戻り値に使う。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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

/// タイマー長設定 (秒単位)。`Settings::durations` から派生する。
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
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

/// TimerMachine が `core/ticker.rs` に返す内部イベント。
///
/// - `Tick(remaining_seconds)` は走行中 1 秒ごと
/// - `StateChanged { phase, count }` はフェーズ遷移時
/// - `Completed { kind }` はセッション完了時 (kind は `Paused` / `Idle` を含まない)
///
/// `core/ticker.rs` がこれらを `timer:tick` / `timer:state_changed` / `timer:completed`
/// イベントの emit payload に変換する。Serialize は不要 (内部 enum)。
#[derive(Debug, Clone, PartialEq)]
pub enum TimerEvent {
    Tick(u32),
    StateChanged { phase: Phase, count: u32 },
    Completed { kind: SessionKind },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_serialize_snake_case() {
        // フロント TS との整合性 (`"short_break"`, `"long_break"` 等)
        assert_eq!(serde_json::to_string(&Phase::Idle).unwrap(), "\"idle\"");
        assert_eq!(serde_json::to_string(&Phase::Work).unwrap(), "\"work\"");
        assert_eq!(
            serde_json::to_string(&Phase::ShortBreak).unwrap(),
            "\"short_break\""
        );
        assert_eq!(
            serde_json::to_string(&Phase::LongBreak).unwrap(),
            "\"long_break\""
        );
        assert_eq!(serde_json::to_string(&Phase::Paused).unwrap(), "\"paused\"");
    }

    #[test]
    fn timer_state_serialize_round_trip() {
        let s = TimerState {
            phase: Phase::Work,
            remaining_seconds: 1234,
            session_count: 2,
            current_duration_seconds: 1500,
        };
        let json = serde_json::to_string(&s).unwrap();
        // フロント TS が期待するキー名がそのまま出ること
        assert!(json.contains("\"phase\":\"work\""));
        assert!(json.contains("\"remaining_seconds\":1234"));
        assert!(json.contains("\"session_count\":2"));
        assert!(json.contains("\"current_duration_seconds\":1500"));
        let back: TimerState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }
}
