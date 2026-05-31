import { invoke } from '@tauri-apps/api/core';
import type { AudioMode, VolumeKind } from '$lib/types';

export const audioSetMode = (mode: AudioMode) =>
  invoke<void>('audio_set_mode', { mode });

export const audioSetVolume = (kind: VolumeKind, value: number) =>
  invoke<void>('audio_set_volume', { kind, value });
