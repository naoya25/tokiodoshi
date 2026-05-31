import type { EasingFn } from './easing';
import { linear } from './easing';

export type TweenHandle = {
  cancel: () => void;
  done: Promise<void>;
};

export function tween(
  from: number,
  to: number,
  durationMs: number,
  onUpdate: (value: number) => void,
  easing: EasingFn = linear,
): TweenHandle {
  let rafId: number | null = null;
  let cancelled = false;
  let resolveDone!: () => void;

  const done = new Promise<void>((resolve) => {
    resolveDone = resolve;
    const start = performance.now();
    const step = (now: number) => {
      if (cancelled) {
        resolve();
        return;
      }
      const t = Math.min(1, (now - start) / durationMs);
      const eased = easing(t);
      onUpdate(from + (to - from) * eased);
      if (t < 1) {
        rafId = requestAnimationFrame(step);
      } else {
        resolve();
      }
    };
    rafId = requestAnimationFrame(step);
  });

  return {
    cancel: () => {
      if (cancelled) return;
      cancelled = true;
      if (rafId !== null) cancelAnimationFrame(rafId);
      resolveDone();
    },
    done,
  };
}

export const sleep = (ms: number): Promise<void> =>
  new Promise((r) => setTimeout(r, ms));
