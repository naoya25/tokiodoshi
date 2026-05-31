import { invoke } from '@tauri-apps/api/core';
import type { SessionRecord } from '$lib/types';

export const historyList = (from: string, to: string) =>
  invoke<SessionRecord[]>('history_list', { from, to });
