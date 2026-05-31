import { describe, it, expect, vi, beforeEach } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import * as settingsIpc from './settings';
import { DEFAULT_SETTINGS } from '$lib/types';

beforeEach(() => {
  invokeMock.mockReset();
});

describe('settings IPC wrappers', () => {
  it("settingsGet は invoke('settings_get') を呼び結果を返す", async () => {
    invokeMock.mockResolvedValueOnce(DEFAULT_SETTINGS);
    const result = await settingsIpc.settingsGet();
    expect(invokeMock).toHaveBeenCalledWith('settings_get');
    expect(result).toEqual(DEFAULT_SETTINGS);
  });

  it("settingsSet は invoke('settings_set', { settings }) を呼ぶ", async () => {
    invokeMock.mockResolvedValueOnce(undefined);
    await settingsIpc.settingsSet(DEFAULT_SETTINGS);
    expect(invokeMock).toHaveBeenCalledWith('settings_set', {
      settings: DEFAULT_SETTINGS,
    });
  });
});
