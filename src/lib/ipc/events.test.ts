import { describe, it, expect, vi, beforeEach } from 'vitest';

const { listenMock } = vi.hoisted(() => ({ listenMock: vi.fn() }));

vi.mock('@tauri-apps/api/event', () => ({ listen: listenMock }));

import { on } from './events';

beforeEach(() => {
  listenMock.mockReset();
});

describe('on', () => {
  it('指定イベント名で listen を呼ぶ', async () => {
    const unlisten = vi.fn();
    listenMock.mockResolvedValueOnce(unlisten);

    const result = await on('timer:tick', () => {});

    expect(listenMock).toHaveBeenCalledTimes(1);
    expect(listenMock.mock.calls[0][0]).toBe('timer:tick');
    expect(result).toBe(unlisten);
  });

  it('handler に payload が渡される', async () => {
    let capturedListener: ((e: { payload: unknown }) => void) | null = null;
    listenMock.mockImplementationOnce((_name: string, cb: (e: { payload: unknown }) => void) => {
      capturedListener = cb;
      return Promise.resolve(() => {});
    });

    const handler = vi.fn();
    await on('timer:tick', handler);

    expect(capturedListener).not.toBeNull();
    capturedListener!({ payload: { remaining_seconds: 42 } });
    expect(handler).toHaveBeenCalledWith({ remaining_seconds: 42 });
  });
});
