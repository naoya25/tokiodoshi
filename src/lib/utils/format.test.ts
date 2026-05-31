import { describe, it, expect } from 'vitest';
import { formatMmss, phaseLabel } from './format';

describe('formatMmss', () => {
  it('25 分は 25:00', () => {
    expect(formatMmss(1500)).toBe('25:00');
  });

  it('0 秒は 00:00', () => {
    expect(formatMmss(0)).toBe('00:00');
  });

  it('1 秒は 00:01', () => {
    expect(formatMmss(1)).toBe('00:01');
  });

  it('1 分は 01:00', () => {
    expect(formatMmss(60)).toBe('01:00');
  });

  it('負の値は 00:00 にクランプ', () => {
    expect(formatMmss(-5)).toBe('00:00');
  });

  it('99:59 を超えても破綻しない', () => {
    expect(formatMmss(6000)).toBe('100:00');
  });
});

describe('phaseLabel', () => {
  it('work -> 仕', () => {
    expect(phaseLabel('work')).toBe('仕');
  });

  it('paused -> 止', () => {
    expect(phaseLabel('paused')).toBe('止');
  });

  it('idle -> 空文字', () => {
    expect(phaseLabel('idle')).toBe('');
  });
});
