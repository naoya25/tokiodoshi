import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { tween, sleep } from './tween';
import { linear, easeInOutCubic } from './easing';

describe('tween', () => {
  let rafCallbacks: FrameRequestCallback[] = [];
  let now = 0;

  beforeEach(() => {
    now = 0;
    rafCallbacks = [];
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
      rafCallbacks.push(cb);
      return rafCallbacks.length;
    });
    vi.stubGlobal('cancelAnimationFrame', (_id: number) => {});
    vi.stubGlobal('performance', { now: () => now });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  function flushOneFrame(elapsed: number) {
    now += elapsed;
    const callbacks = [...rafCallbacks];
    rafCallbacks = [];
    for (const cb of callbacks) cb(now);
  }

  it('開始値で onUpdate が呼ばれる', () => {
    const updates: number[] = [];
    tween(0, 100, 100, (v) => updates.push(v), linear);
    flushOneFrame(0);
    expect(updates[0]).toBeCloseTo(0);
  });

  it('終了時に目標値で完了する', async () => {
    const updates: number[] = [];
    const handle = tween(0, 100, 100, (v) => updates.push(v), linear);
    flushOneFrame(0);
    flushOneFrame(100);
    await handle.done;
    expect(updates[updates.length - 1]).toBeCloseTo(100);
  });

  it('中点で線形補間される (linear)', () => {
    const updates: number[] = [];
    tween(0, 100, 100, (v) => updates.push(v), linear);
    flushOneFrame(0);
    flushOneFrame(50);
    expect(updates[updates.length - 1]).toBeCloseTo(50);
  });

  it('easing 関数が反映される (easeInOutCubic 中点 = 0.5)', () => {
    const updates: number[] = [];
    tween(0, 100, 100, (v) => updates.push(v), easeInOutCubic);
    flushOneFrame(0);
    flushOneFrame(50);
    expect(updates[updates.length - 1]).toBeCloseTo(50);
  });

  it('負の方向の補間も動く', () => {
    const updates: number[] = [];
    tween(50, -50, 100, (v) => updates.push(v), linear);
    flushOneFrame(0);
    flushOneFrame(50);
    expect(updates[updates.length - 1]).toBeCloseTo(0);
  });

  it('cancel で完了 Promise が解決する', async () => {
    const handle = tween(0, 100, 100, () => {}, linear);
    flushOneFrame(0);
    handle.cancel();
    await expect(handle.done).resolves.toBeUndefined();
  });
});

describe('sleep', () => {
  it('指定ミリ秒待ってから解決する', async () => {
    vi.useFakeTimers();
    const p = sleep(100);
    vi.advanceTimersByTime(100);
    await expect(p).resolves.toBeUndefined();
    vi.useRealTimers();
  });
});
