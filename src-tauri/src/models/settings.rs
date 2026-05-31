use serde::{Deserialize, Serialize};

use super::timer_state::TimerConfig;

/// 音再生のモード。
/// フロント `AudioMode` と同期 (`"silent"`, `"kakon_only"`, `"full"`)。
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AudioMode {
    Silent,
    KakonOnly,
    Full,
}

impl Default for AudioMode {
    fn default() -> Self {
        AudioMode::Full
    }
}

/// テーマ設定。`appearance.theme` に対応。
/// フロント `Theme` と同期 (`"system"`, `"light"`, `"dark"`)。
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::System
    }
}

/// `audio_set_volume` コマンドで指定する対象。
/// フロント `VolumeKind` と同期 (`"master"`, `"water"`, `"kakon"`)。
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum VolumeKind {
    Master,
    Water,
    Kakon,
}

/// 作業セッションの長さ (秒)。MVP では `work_seconds` のみ。
///
/// 旧フィールド `short_break_seconds` / `long_break_seconds` /
/// `sessions_until_long_break` は休憩フェーズ廃止に伴い削除済み。
/// 既存の settings.json に古いキーが残っていても serde が無視する。
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct DurationsSettings {
    pub work_seconds: u32,
}

impl Default for DurationsSettings {
    fn default() -> Self {
        Self { work_seconds: 1500 }
    }
}

impl From<&DurationsSettings> for TimerConfig {
    fn from(d: &DurationsSettings) -> Self {
        TimerConfig {
            work_seconds: d.work_seconds,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AudioSettings {
    pub mode: AudioMode,
    pub master_volume: f32,
    pub water_volume: f32,
    pub kakon_volume: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        // docs/requirements.md §6.1 のデフォルト値
        Self {
            mode: AudioMode::Full,
            master_volume: 0.7,
            water_volume: 0.3,
            kakon_volume: 0.6,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BehaviorSettings {
    pub launch_at_login: bool,
    pub hide_dock_icon: bool,
    pub auto_show_window_on_start: bool,
    /// Work セッション完了後に自動で次のセッションを開始するか。
    /// false の場合は Idle に戻って手動で start を待つ。
    #[serde(default)]
    pub loop_sessions: bool,
}

impl Default for BehaviorSettings {
    fn default() -> Self {
        Self {
            launch_at_login: false,
            hide_dock_icon: false,
            auto_show_window_on_start: true,
            loop_sessions: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppearanceSettings {
    pub theme: Theme,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
        }
    }
}

/// アプリ全体の設定。`tauri-plugin-store` に JSON で永続化される。
/// フロント `Settings` と同期。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Settings {
    pub durations: DurationsSettings,
    pub audio: AudioSettings,
    pub behavior: BehaviorSettings,
    pub appearance: AppearanceSettings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_matches_spec() {
        let s = Settings::default();
        assert_eq!(s.durations.work_seconds, 1500);

        assert_eq!(s.audio.mode, AudioMode::Full);
        assert!((s.audio.master_volume - 0.7).abs() < f32::EPSILON);
        assert!((s.audio.water_volume - 0.3).abs() < f32::EPSILON);
        assert!((s.audio.kakon_volume - 0.6).abs() < f32::EPSILON);

        assert!(!s.behavior.launch_at_login);
        assert!(!s.behavior.hide_dock_icon);
        assert!(s.behavior.auto_show_window_on_start);

        assert_eq!(s.appearance.theme, Theme::System);
    }

    #[test]
    fn settings_full_round_trip_matches_ts_keys() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();

        assert!(json.contains("\"work_seconds\":1500"));
        // 旧フィールドは含まれない
        assert!(!json.contains("\"short_break_seconds\""));
        assert!(!json.contains("\"long_break_seconds\""));
        assert!(!json.contains("\"sessions_until_long_break\""));

        assert!(json.contains("\"mode\":\"full\""));
        assert!(json.contains("\"master_volume\":0.7"));
        assert!(json.contains("\"water_volume\":0.3"));
        assert!(json.contains("\"kakon_volume\":0.6"));

        assert!(json.contains("\"launch_at_login\":false"));
        assert!(json.contains("\"hide_dock_icon\":false"));
        assert!(json.contains("\"auto_show_window_on_start\":true"));

        assert!(json.contains("\"theme\":\"system\""));

        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn settings_partial_update_replaces_field() {
        let mut s = Settings::default();
        s.audio.mode = AudioMode::Silent;
        s.audio.master_volume = 0.1;
        s.durations.work_seconds = 60;

        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();

        assert_eq!(back.audio.mode, AudioMode::Silent);
        assert!((back.audio.master_volume - 0.1).abs() < f32::EPSILON);
        assert_eq!(back.durations.work_seconds, 60);
        assert_eq!(back.appearance.theme, Theme::System);
        assert!(back.behavior.auto_show_window_on_start);
    }
}
