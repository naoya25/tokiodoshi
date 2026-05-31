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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DurationsSettings {
    pub work_seconds: u32,
    pub short_break_seconds: u32,
    pub long_break_seconds: u32,
    pub sessions_until_long_break: u32,
}

impl Default for DurationsSettings {
    fn default() -> Self {
        // docs/requirements.md §6.1 のデフォルト値
        Self {
            work_seconds: 1500,
            short_break_seconds: 300,
            long_break_seconds: 900,
            sessions_until_long_break: 4,
        }
    }
}

impl From<&DurationsSettings> for TimerConfig {
    fn from(d: &DurationsSettings) -> Self {
        TimerConfig {
            work_seconds: d.work_seconds,
            short_break_seconds: d.short_break_seconds,
            long_break_seconds: d.long_break_seconds,
            sessions_until_long_break: d.sessions_until_long_break,
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
}

impl Default for BehaviorSettings {
    fn default() -> Self {
        Self {
            launch_at_login: false,
            hide_dock_icon: false,
            auto_show_window_on_start: true,
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
        // docs/requirements.md §6.1 と DEFAULT_SETTINGS (TS) の一致を担保
        let s = Settings::default();
        assert_eq!(s.durations.work_seconds, 1500);
        assert_eq!(s.durations.short_break_seconds, 300);
        assert_eq!(s.durations.long_break_seconds, 900);
        assert_eq!(s.durations.sessions_until_long_break, 4);

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
        // フロント TS の DEFAULT_SETTINGS と互換のキー名で出ること
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();

        // durations
        assert!(json.contains("\"work_seconds\":1500"));
        assert!(json.contains("\"short_break_seconds\":300"));
        assert!(json.contains("\"long_break_seconds\":900"));
        assert!(json.contains("\"sessions_until_long_break\":4"));

        // audio (muted は廃止済み)
        assert!(json.contains("\"mode\":\"full\""));
        assert!(json.contains("\"master_volume\":0.7"));
        assert!(json.contains("\"water_volume\":0.3"));
        assert!(json.contains("\"kakon_volume\":0.6"));
        assert!(!json.contains("\"muted\""));

        // behavior
        assert!(json.contains("\"launch_at_login\":false"));
        assert!(json.contains("\"hide_dock_icon\":false"));
        assert!(json.contains("\"auto_show_window_on_start\":true"));

        // appearance
        assert!(json.contains("\"theme\":\"system\""));

        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn settings_partial_update_replaces_field() {
        // settings_set で部分的に変えても、他フィールドが温存できる流れの確認。
        let mut s = Settings::default();
        s.audio.mode = AudioMode::Silent;
        s.audio.master_volume = 0.1;
        s.durations.work_seconds = 60;

        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();

        assert_eq!(back.audio.mode, AudioMode::Silent);
        assert!((back.audio.master_volume - 0.1).abs() < f32::EPSILON);
        assert_eq!(back.durations.work_seconds, 60);
        // 触っていない箇所はデフォルトのまま
        assert_eq!(back.appearance.theme, Theme::System);
        assert!(back.behavior.auto_show_window_on_start);
        assert_eq!(back.durations.long_break_seconds, 900);
    }
}
