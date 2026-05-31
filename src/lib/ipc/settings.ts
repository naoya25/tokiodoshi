import { invoke } from '@tauri-apps/api/core';
import type { Settings } from '$lib/types';

export const settingsGet = () => invoke<Settings>('settings_get');
export const settingsSet = (settings: Settings) =>
  invoke<void>('settings_set', { settings });
