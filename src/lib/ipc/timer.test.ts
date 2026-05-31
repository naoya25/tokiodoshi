import { describe, it, expect, vi, beforeEach } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

import * as timerIpc from './timer';

beforeEach(() => {
  invokeMock.mockReset();
});

describe('timer IPC wrappers', () => {
  const stubState = {
    phase: 'idle',
    remaining_seconds: 1500,
    session_count: 0,
    current_duration_seconds: 1500,
  };

  for (const [name, fn, expectedCommand] of [
    ['timerStart', timerIpc.timerStart, 'timer_start'],
    ['timerPause', timerIpc.timerPause, 'timer_pause'],
    ['timerReset', timerIpc.timerReset, 'timer_reset'],
    ['timerSkip', timerIpc.timerSkip, 'timer_skip'],
    ['timerGetState', timerIpc.timerGetState, 'timer_get_state'],
  ] as const) {
    it(`${name} は invoke('${expectedCommand}') を呼ぶ`, async () => {
      invokeMock.mockResolvedValueOnce(stubState);
      const result = await fn();
      expect(invokeMock).toHaveBeenCalledWith(expectedCommand);
      expect(result).toEqual(stubState);
    });
  }

  it('エラーは throw される (UI 側でハンドル)', async () => {
    invokeMock.mockRejectedValueOnce(new Error('backend down'));
    await expect(timerIpc.timerStart()).rejects.toThrow('backend down');
  });
});
