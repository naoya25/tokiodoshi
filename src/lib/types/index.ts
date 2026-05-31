// Rust 側 `src-tauri/src/models/` と手書きで同期する TypeScript 型定義。
// Rust 側を変更したら、このファイルも必ず合わせて更新すること。
//
// MVP では休憩フェーズを廃止し、Phase は idle / work / paused のみ。

export type Phase = 'idle' | 'work' | 'paused';

export type SessionKind = 'work';

export interface TimerState {
  phase: Phase;
  remaining_seconds: number;
  session_count: number;
  current_duration_seconds: number;
}

export type AudioMode = 'silent' | 'kakon_only' | 'full';

export type Theme = 'system' | 'light' | 'dark';

export interface DurationsSettings {
  work_seconds: number;
}

export interface AudioSettings {
  mode: AudioMode;
  master_volume: number;
  water_volume: number;
  kakon_volume: number;
}

export interface BehaviorSettings {
  launch_at_login: boolean;
  hide_dock_icon: boolean;
  auto_show_window_on_start: boolean;
}

export interface AppearanceSettings {
  theme: Theme;
}

export interface Settings {
  durations: DurationsSettings;
  audio: AudioSettings;
  behavior: BehaviorSettings;
  appearance: AppearanceSettings;
}

export interface SessionRecord {
  id: number;
  type: SessionKind;
  started_at: string;
  completed_at: string | null;
  was_completed: boolean;
  planned_duration_seconds: number;
}

export type VolumeKind = 'master' | 'water' | 'kakon';

// イベントペイロード
export interface TickPayload {
  remaining_seconds: number;
}

export interface StateChangedPayload {
  phase: Phase;
  session_count: number;
}

export interface CompletedPayload {
  type: SessionKind;
}

export type EventMap = {
  'timer:tick': TickPayload;
  'timer:state_changed': StateChangedPayload;
  'timer:completed': CompletedPayload;
};

export const DEFAULT_SETTINGS: Settings = {
  durations: {
    work_seconds: 1500,
  },
  audio: {
    mode: 'full',
    master_volume: 0.7,
    water_volume: 0.3,
    kakon_volume: 0.6,
  },
  behavior: {
    launch_at_login: false,
    hide_dock_icon: false,
    auto_show_window_on_start: true,
  },
  appearance: {
    theme: 'system',
  },
};
