import type { UnlistenFn } from '@tauri-apps/api/event';
import { on } from '$lib/ipc/events';
import * as timerIpc from '$lib/ipc/timer';
import type { Phase, TimerState } from '$lib/types';
import { tween, sleep } from '$lib/utils/tween';
import { easeInQuad, easeOutCubic } from '$lib/utils/easing';

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
   * 実物のししおどしは「戻ってきて石を打つ瞬間」にカコンが鳴る。
   * バック側が `end_at - KAKON_LEAD_MS` に Completed を先行発火するので、
   * **戻り終わり**がちょうどタイマー 00:00 の瞬間 = カコン音タイミングになる。
   *
   * シーケンス (合計 1500ms 想定 — バック側 KAKON_LEAD_MS と整合):
   * 1. -12° → +12° (1000ms easeInQuad、重みでゆっくり倒れて水排出)
   * 2. +12° → -12° (500ms easeOutCubic、勢いよく戻って石にぶつかる)
   * 3. 余韻 (500ms)
   */
  private async playKakon(): Promise<void> {
    this.isAnimating = true;
    const setTilt = (v: number) => {
      this.tilt = v;
    };

    await tween(this.tilt, 12, 1000, setTilt, easeInQuad).done;
    await tween(12, INITIAL_TILT, 500, setTilt, easeOutCubic).done;
    await sleep(500);

    this.isAnimating = false;
  }
}

export const timerStore = new TimerStore();
export { INITIAL_TILT };
