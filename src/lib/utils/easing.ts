export const easeInOutCubic = (t: number): number =>
  t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;

export const easeInQuad = (t: number): number => t * t;

export const easeOutCubic = (t: number): number => 1 - Math.pow(1 - t, 3);

export const linear = (t: number): number => t;

export type EasingFn = (t: number) => number;
