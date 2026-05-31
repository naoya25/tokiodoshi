use serde::{Deserialize, Serialize};

use super::timer_state::TimerConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurationsSettings {
    pub work_seconds: u32,
    pub short_break_seconds: u32,
    pub long_break_seconds: u32,
    pub sessions_until_long_break: u32,
}

impl Default for DurationsSettings {
    fn default() -> Self {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    pub mode: AudioMode,
    pub master_volume: f32,
    pub water_volume: f32,
    pub kakon_volume: f32,
    pub muted: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            mode: AudioMode::Full,
            master_volume: 0.7,
            water_volume: 0.3,
            kakon_volume: 0.6,
            muted: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceSettings {
    pub theme: Theme,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self { theme: Theme::System }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub durations: DurationsSettings,
    pub audio: AudioSettings,
    pub behavior: BehaviorSettings,
    pub appearance: AppearanceSettings,
}
