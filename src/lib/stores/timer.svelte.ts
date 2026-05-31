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
   * セッション完了時のカコン演出: `-12° → 0° → -12°` の往復だけ。
   * 走行中は静止しているので、ここが唯一の動きになる。
   *
   * シーケンス:
   * 1. 鳴る前の静寂 (400ms)
   * 2. -12° → 0° (280ms easeInQuad、重みで一気に倒れて石を打つ)
   * 3. 静止 (80ms、カコン音が乗る瞬間)
   * 4. 0° → -12° (700ms easeInOutCubic、ゆっくり元へ)
   * 5. 余韻 (500ms)
   */
  private async playKakon(): Promise<void> {
    this.isAnimating = true;
    const setTilt = (v: number) => {
      this.tilt = v;
    };

    await sleep(400);
    await tween(this.tilt, 0, 280, setTilt, easeInQuad).done;
    await sleep(80);
    await tween(0, INITIAL_TILT, 700, setTilt, easeInOutCubic).done;
    await sleep(500);

    this.isAnimating = false;
  }
}

export const timerStore = new TimerStore();
export { INITIAL_TILT };
