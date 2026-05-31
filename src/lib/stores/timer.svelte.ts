import type { UnlistenFn } from '@tauri-apps/api/event';
import { on } from '$lib/ipc/events';
import * as timerIpc from '$lib/ipc/timer';
import type { Phase, TimerState } from '$lib/types';
import { tween, sleep } from '$lib/utils/tween';
import { easeInOutCubic, easeInQuad, easeOutCubic } from '$lib/utils/easing';

const INITIAL_TILT = -12;
const FULL_TILT = 12;

class TimerStore {
  remainingSeconds = $state(0);
  phase = $state<Phase>('idle');
  sessionCount = $state(0);
  currentDurationSeconds = $state(1500);
  isAnimating = $state(false);
  tilt = $state(INITIAL_TILT);

  private unlistenFns: UnlistenFn[] = [];

  progress = $derived(
    this.currentDurationSeconds > 0
      ? 1 - this.remainingSeconds / this.currentDurationSeconds
      : 0,
  );

  async init(): Promise<void> {
    try {
      const state = await timerIpc.timerGetState();
      this.applyState(state);
    } catch (e) {
      console.warn('[timer] init failed:', e);
    }

    this.unlistenFns.push(
      await on('timer:tick', (p) => {
        if (this.isAnimating) return;
        this.remainingSeconds = p.remaining_seconds;
        this.updateTiltFromProgress();
      }),
      await on('timer:state_changed', (p) => {
        this.phase = p.phase;
        this.sessionCount = p.session_count;
      }),
      await on('timer:completed', (p) => {
        if (p.type === 'work') {
          void this.playKakon();
        }
      }),
    );
  }

  destroy(): void {
    for (const u of this.unlistenFns) u();
    this.unlistenFns = [];
  }

  private applyState(s: TimerState): void {
    this.phase = s.phase;
    this.remainingSeconds = s.remaining_seconds;
    this.sessionCount = s.session_count;
    this.currentDurationSeconds = s.current_duration_seconds;
    this.updateTiltFromProgress();
  }

  private updateTiltFromProgress(): void {
    if (this.isAnimating) return;
    if (this.phase === 'work') {
      const p = Math.min(1, Math.max(0, this.progress));
      this.tilt = INITIAL_TILT + p * (FULL_TILT - INITIAL_TILT);
    } else {
      this.tilt = INITIAL_TILT;
    }
  }

  async start(): Promise<void> {
    try {
      const s = await timerIpc.timerStart();
      this.applyState(s);
    } catch (e) {
      console.warn('[timer] start failed:', e);
    }
  }

  async pause(): Promise<void> {
    try {
      const s = await timerIpc.timerPause();
      this.applyState(s);
    } catch (e) {
      console.warn('[timer] pause failed:', e);
    }
  }

  async reset(): Promise<void> {
    try {
      const s = await timerIpc.timerReset();
      this.applyState(s);
      this.tilt = INITIAL_TILT;
    } catch (e) {
      console.warn('[timer] reset failed:', e);
    }
  }

  async skip(): Promise<void> {
    if (this.isAnimating) return;
    try {
      const s = await timerIpc.timerSkip();
      this.applyState(s);
    } catch (e) {
      console.warn('[timer] skip failed:', e);
    }
  }

  private async playKakon(): Promise<void> {
    this.isAnimating = true;
    const setTilt = (v: number) => {
      this.tilt = v;
    };

    // 1. 鳴る前の静寂
    await sleep(400);

    // 2. 重みで傾く: → +48°
    await tween(this.tilt, 48, 420, setTilt, easeInQuad).done;

    // 3. 排水の一拍
    await sleep(180);

    // 4. 反動で逆へ: 48° → -25°
    await tween(48, -25, 310, setTilt, easeOutCubic).done;

    // 5. 初期姿勢に戻る
    await tween(-25, INITIAL_TILT, 900, setTilt, easeInOutCubic).done;

    // 6. 余韻
    await sleep(600);
    this.isAnimating = false;
  }
}

export const timerStore = new TimerStore();
export { INITIAL_TILT, FULL_TILT };
