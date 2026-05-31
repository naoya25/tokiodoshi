import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { EventMap } from '$lib/types';

export async function on<K extends keyof EventMap>(
  name: K,
  handler: (payload: EventMap[K]) => void,
): Promise<UnlistenFn> {
  return listen<EventMap[K]>(name, (e) => handler(e.payload));
}
