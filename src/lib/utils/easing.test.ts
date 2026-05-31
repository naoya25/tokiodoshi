import { describe, it, expect } from 'vitest';
import { easeInOutCubic, easeInQuad, easeOutCubic, linear } from './easing';

describe('easing functions', () => {
  describe('境界値', () => {
    for (const [name, fn] of [
      ['linear', linear],
      ['easeInOutCubic', easeInOutCubic],
      ['easeInQuad', easeInQuad],
      ['easeOutCubic', easeOutCubic],
    ] as const) {
      it(`${name}: f(0) = 0`, () => {
        expect(fn(0)).toBeCloseTo(0);
      });
      it(`${name}: f(1) = 1`, () => {
        expect(fn(1)).toBeCloseTo(1);
      });
    }
  });

  it('linear は恒等', () => {
    expect(linear(0.3)).toBeCloseTo(0.3);
    expect(linear(0.7)).toBeCloseTo(0.7);
  });

  it('easeInQuad は中間値が線形より遅い', () => {
    expect(easeInQuad(0.5)).toBeLessThan(0.5);
  });

  it('easeOutCubic は中間値が線形より速い', () => {
    expect(easeOutCubic(0.5)).toBeGreaterThan(0.5);
  });

  it('easeInOutCubic は中点で 0.5', () => {
    expect(easeInOutCubic(0.5)).toBeCloseTo(0.5);
  });
});
