import { describe, it, expect, vi, beforeEach } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { historyList } from './history';

beforeEach(() => {
  invokeMock.mockReset();
});

describe('historyList', () => {
  it("invoke('history_list', { from, to }) を呼ぶ", async () => {
    invokeMock.mockResolvedValueOnce([]);
    await historyList('2026-05-25T00:00:00Z', '2026-06-01T00:00:00Z');
    expect(invokeMock).toHaveBeenCalledWith('history_list', {
      from: '2026-05-25T00:00:00Z',
      to: '2026-06-01T00:00:00Z',
    });
  });

  it('結果は配列で返る', async () => {
    invokeMock.mockResolvedValueOnce([
      {
        id: 1,
        type: 'work',
        started_at: '2026-06-01T09:00:00Z',
        completed_at: '2026-06-01T09:25:00Z',
        was_completed: true,
        planned_duration_seconds: 1500,
      },
    ]);
    const result = await historyList('a', 'b');
    expect(result).toHaveLength(1);
    expect(result[0].type).toBe('work');
  });
});
