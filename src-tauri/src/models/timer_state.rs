use serde::{Deserialize, Serialize};

/// TimerMachine の現フェーズ。
/// フロント `src/lib/types/index.ts::Phase` と同期 (`"idle"`, `"work"`, `"paused"`)。
///
/// MVP では休憩フェーズを廃止し、単純な作業タイマーに簡略化している
/// (Work 完了 → Idle へ戻るだけ)。将来再導入する場合はこの enum と
/// `TimerMachine::poll()` の遷移ロジックを同時に拡張する。
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    Work,
    Paused,
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Idle
    }
}

/// 履歴に保存されるセッションの種類。MVP では `Work` のみ。
/// 将来の拡張余地として enum で残す (variant を増やすだけで OK)。
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SessionKind {
    Work,
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
/// MVP では `work_seconds` のみ。
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct TimerConfig {
    pub work_seconds: u32,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self { work_seconds: 1500 }
    }
}

/// TimerMachine が `core/ticker.rs` に返す内部イベント。
///
/// - `Tick(remaining_seconds)` は走行中 1 秒ごと
/// - `StateChanged { phase, count }` はフェーズ遷移時
/// - `Completed { kind }` はセッション完了時 (kind は MVP では `Work` のみ)
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
        assert_eq!(serde_json::to_string(&Phase::Idle).unwrap(), "\"idle\"");
        assert_eq!(serde_json::to_string(&Phase::Work).unwrap(), "\"work\"");
        assert_eq!(serde_json::to_string(&Phase::Paused).unwrap(), "\"paused\"");
    }

    #[test]
    fn session_kind_serialize_snake_case() {
        assert_eq!(serde_json::to_string(&SessionKind::Work).unwrap(), "\"work\"");
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
        assert!(json.contains("\"phase\":\"work\""));
        assert!(json.contains("\"remaining_seconds\":1234"));
        assert!(json.contains("\"session_count\":2"));
        assert!(json.contains("\"current_duration_seconds\":1500"));
        let back: TimerState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn timer_config_default_is_25min() {
        assert_eq!(TimerConfig::default().work_seconds, 1500);
    }
}
