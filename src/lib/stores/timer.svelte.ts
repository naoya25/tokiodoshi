import type { UnlistenFn } from '@tauri-apps/api/event';
import { on } from '$lib/ipc/events';
import * as timerIpc from '$lib/ipc/timer';
import type { Phase, TimerState } from '$lib/types';
import { tween, sleep } from '$lib/utils/tween';
import { easeInOutCubic, easeInQuad } from '$lib/utils/easing';

/**
 * 筒の基本姿勢 (-12° = 水入り口がやや上向き)。
 * 走行中は時間経過で角度を変えず、この姿勢で静止する。
 * セッション完了時に一度だけ playKakon で 0° へ振れて石を打ち、戻る。
 */
const INITIAL_TILT = -12;

class TimerStore {
  remainingSeconds = $state(0);
  phase = $state<Phase>('idle');
  sessionCount = $state(0);
  currentDurationSeconds = $state(1500);
  isAnimating = $state(false);
  tilt = $state(INITIAL_TILT);

  private unlistenFns: UnlistenFn[] = [];

  async init(): Promise<void> {
    try {
      const state = await timerIpc.timerGetState();
      this.applyState(state);
    } catch (e) {
      console.warn('[timer] init failed:', e);
    }

    this.unlistenFns.push(
      await on('timer:tick', (p) => {
        // tilt は時間経過では変えない (走行中も -12° で静止)
        this.remainingSeconds = p.remaining_seconds;
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
    if (!this.isAnimating) {
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

  /**
   * セッション完了時のカコン演出: `-12° → +12° → -12°` の往復だけ。
   * バック側で `end_at - 280ms` に Completed が発火するので、
   * 倒れ終わり (+12°) がちょうどタイマー 00:00 の瞬間 = カコン音タイミング。
   *
   * シーケンス:
   * 1. -12° → +12° (280ms easeInQuad、重みで倒れて石を打つ)
   * 2. 静止 (80ms、カコン音が乗る瞬間)
   * 3. +12° → -12° (700ms easeInOutCubic、ゆっくり元へ)
   * 4. 余韻 (500ms)
   */
  private async playKakon(): Promise<void> {
    this.isAnimating = true;
    const setTilt = (v: number) => {
      this.tilt = v;
    };

    await tween(this.tilt, 12, 280, setTilt, easeInQuad).done;
    await sleep(80);
    await tween(12, INITIAL_TILT, 700, setTilt, easeInOutCubic).done;
    await sleep(500);

    this.isAnimating = false;
  }
}

export const timerStore = new TimerStore();
export { INITIAL_TILT };
