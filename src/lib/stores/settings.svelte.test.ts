import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { settingsStore } from './settings.svelte';
import { DEFAULT_SETTINGS } from '$lib/types';

beforeEach(() => {
  invokeMock.mockReset();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('SettingsStore', () => {
  it('init で settings_get を呼んで状態を更新する', async () => {
    const custom = {
      ...DEFAULT_SETTINGS,
      audio: { ...DEFAULT_SETTINGS.audio, master_volume: 0.42 },
    };
    invokeMock.mockResolvedValueOnce(custom);

    await settingsStore.init();
    expect(invokeMock).toHaveBeenCalledWith('settings_get');
    expect(settingsStore.settings.audio.master_volume).toBe(0.42);
  });

  it('init 失敗時はデフォルトのまま (warn のみ)', async () => {
    invokeMock.mockRejectedValueOnce(new Error('store error'));
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    await settingsStore.init();
    expect(warnSpy).toHaveBeenCalled();
    warnSpy.mockRestore();
  });

  it('updateNested で部分更新できる', () => {
    vi.useFakeTimers();
    settingsStore.updateNested('audio', { master_volume: 0.1 });
    expect(settingsStore.settings.audio.master_volume).toBe(0.1);
    expect(settingsStore.settings.audio.mode).toBe(DEFAULT_SETTINGS.audio.mode);
  });

  it('updateNested は 200ms デバウンスで settings_set を呼ぶ', () => {
    vi.useFakeTimers();
    invokeMock.mockResolvedValue(undefined);

    settingsStore.updateNested('audio', { master_volume: 0.2 });
    settingsStore.updateNested('audio', { master_volume: 0.3 });
    settingsStore.updateNested('audio', { master_volume: 0.4 });

    expect(invokeMock).not.toHaveBeenCalledWith('settings_set', expect.anything());

    vi.advanceTimersByTime(200);
    const setCalls = invokeMock.mock.calls.filter((c) => c[0] === 'settings_set');
    expect(setCalls.length).toBe(1);
    expect((setCalls[0][1] as { settings: typeof DEFAULT_SETTINGS }).settings.audio.master_volume).toBe(0.4);
  });
});
