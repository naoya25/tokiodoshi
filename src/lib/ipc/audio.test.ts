import { describe, it, expect, vi, beforeEach } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { audioSetMode, audioSetVolume } from './audio';

beforeEach(() => {
  invokeMock.mockReset();
});

describe('audio IPC wrappers', () => {
  it('audioSetMode が正しい引数で呼ばれる', async () => {
    invokeMock.mockResolvedValueOnce(undefined);
    await audioSetMode('full');
    expect(invokeMock).toHaveBeenCalledWith('audio_set_mode', { mode: 'full' });
  });

  it('audioSetVolume が kind/value 付きで呼ばれる', async () => {
    invokeMock.mockResolvedValueOnce(undefined);
    await audioSetVolume('master', 0.5);
    expect(invokeMock).toHaveBeenCalledWith('audio_set_volume', {
      kind: 'master',
      value: 0.5,
    });
  });
});
