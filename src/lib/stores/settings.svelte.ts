import * as settingsIpc from '$lib/ipc/settings';
import { DEFAULT_SETTINGS, type Settings } from '$lib/types';

class SettingsStore {
  settings = $state<Settings>(structuredClone(DEFAULT_SETTINGS));

  private saveTimer: ReturnType<typeof setTimeout> | null = null;

  async init(): Promise<void> {
    try {
      this.settings = await settingsIpc.settingsGet();
    } catch (e) {
      console.warn('[settings] init failed, using defaults:', e);
    }
  }

  /** 部分更新（debounce 200ms で save） */
  update(partial: Partial<Settings>): void {
    this.settings = { ...this.settings, ...partial } as Settings;
    this.scheduleSave();
  }

  /** ネストしたフィールドの更新 */
  updateNested<K extends keyof Settings>(key: K, partial: Partial<Settings[K]>): void {
    this.settings = {
      ...this.settings,
      [key]: { ...this.settings[key], ...partial },
    };
    this.scheduleSave();
  }

  private scheduleSave(): void {
    if (this.saveTimer !== null) clearTimeout(this.saveTimer);
    this.saveTimer = setTimeout(() => {
      void this.flush();
    }, 200);
  }

  private async flush(): Promise<void> {
    try {
      await settingsIpc.settingsSet(this.settings);
    } catch (e) {
      console.warn('[settings] save failed:', e);
    }
  }
}

export const settingsStore = new SettingsStore();
