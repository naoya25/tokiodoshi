import { describe, it, expect } from 'vitest';
import { parseDuration, formatDuration } from './duration';

describe('parseDuration', () => {
  describe('数値単独', () => {
    it('"90" は 90 秒', () => expect(parseDuration('90')).toBe(90));
    it('"0" は 0 秒', () => expect(parseDuration('0')).toBe(0));
  });

  describe('単位付き', () => {
    it('"1h" は 3600 秒', () => expect(parseDuration('1h')).toBe(3600));
    it('"25m" は 1500 秒', () => expect(parseDuration('25m')).toBe(1500));
    it('"30s" は 30 秒', () => expect(parseDuration('30s')).toBe(30));
    it('"1.5h" は 5400 秒', () => expect(parseDuration('1.5h')).toBe(5400));
    it('"0.5m" は 30 秒', () => expect(parseDuration('0.5m')).toBe(30));
    it('大文字小文字を許容: "10M"', () => expect(parseDuration('10M')).toBe(600));
    it('前後の空白も許容', () => expect(parseDuration('  10m  ')).toBe(600));
  });

  describe('複合単位', () => {
    it('"1h30m" は 5400 秒', () => expect(parseDuration('1h30m')).toBe(5400));
    it('"1h30m45s" は 5445 秒', () =>
      expect(parseDuration('1h30m45s')).toBe(5445));
    it('"30m45s" は 1845 秒', () => expect(parseDuration('30m45s')).toBe(1845));
    it('"1h0m0s" は 3600 秒', () => expect(parseDuration('1h0m0s')).toBe(3600));
  });

  describe('コロン区切り', () => {
    it('"25:00" は 1500 秒 (MM:SS)', () =>
      expect(parseDuration('25:00')).toBe(1500));
    it('"10:5" は 605 秒 (MM:SS)', () => expect(parseDuration('10:5')).toBe(605));
    it('"1:30:00" は 5400 秒 (HH:MM:SS)', () =>
      expect(parseDuration('1:30:00')).toBe(5400));
    it('"2:30:15" は 9015 秒 (HH:MM:SS)', () =>
      expect(parseDuration('2:30:15')).toBe(9015));
  });

  describe('無効入力', () => {
    it('空文字は null', () => expect(parseDuration('')).toBeNull());
    it('空白のみは null', () => expect(parseDuration('   ')).toBeNull());
    it('意味不明な文字列は null', () =>
      expect(parseDuration('abc')).toBeNull());
    it('単位が無く非数値は null', () =>
      expect(parseDuration('25x')).toBeNull());
  });
});

describe('formatDuration', () => {
  it('1500 秒は "25:00"', () => expect(formatDuration(1500)).toBe('25:00'));
  it('0 秒は "00:00"', () => expect(formatDuration(0)).toBe('00:00'));
  it('1 秒は "00:01"', () => expect(formatDuration(1)).toBe('00:01'));
  it('60 秒は "01:00"', () => expect(formatDuration(60)).toBe('01:00'));
  it('3599 秒は "59:59" (1 時間未満)', () =>
    expect(formatDuration(3599)).toBe('59:59'));
  it('3600 秒は "1:00:00" (1 時間以上)', () =>
    expect(formatDuration(3600)).toBe('1:00:00'));
  it('5445 秒は "1:30:45"', () => expect(formatDuration(5445)).toBe('1:30:45'));
  it('負の値は "00:00"', () => expect(formatDuration(-10)).toBe('00:00'));
});

describe('parseDuration ↔ formatDuration ラウンドトリップ', () => {
  for (const s of [0, 1, 60, 1500, 3600, 5445, 9015]) {
    it(`${s} → format → parse で同値`, () => {
      const back = parseDuration(formatDuration(s));
      expect(back).toBe(s);
    });
  }
});
