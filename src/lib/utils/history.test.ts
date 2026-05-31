import { describe, it, expect } from 'vitest';
import { aggregateByDay, lastNDateKeys, formatDayShort } from './history';
import type { SessionRecord } from '$lib/types';

const mkSession = (overrides: Partial<SessionRecord> = {}): SessionRecord => ({
  id: 1,
  type: 'work',
  started_at: '2026-06-01T09:00:00Z',
  completed_at: '2026-06-01T09:25:00Z',
  was_completed: true,
  planned_duration_seconds: 1500,
  ...overrides,
});

describe('lastNDateKeys', () => {
  it('指定日数分のキーを過去→今日順で返す', () => {
    const today = new Date('2026-06-03T12:00:00Z');
    const keys = lastNDateKeys(3, today);
    expect(keys).toEqual(['2026-06-01', '2026-06-02', '2026-06-03']);
  });

  it('1 日を渡せば今日だけ', () => {
    const today = new Date('2026-06-03T12:00:00Z');
    expect(lastNDateKeys(1, today)).toEqual(['2026-06-03']);
  });
});

describe('aggregateByDay', () => {
  it('空配列でも全キーがゼロで返る', () => {
    const result = aggregateByDay([], ['2026-06-01', '2026-06-02']);
    expect(result).toHaveLength(2);
    expect(result[0]).toEqual({
      date: '2026-06-01',
      completedWork: 0,
      totalSessions: 0,
    });
  });

  it('work かつ was_completed のみ completedWork にカウント', () => {
    const sessions = [
      mkSession({ id: 1, type: 'work', was_completed: true }),
      mkSession({ id: 2, type: 'work', was_completed: false }),
      mkSession({ id: 3, type: 'work', was_completed: true }),
    ];
    const result = aggregateByDay(sessions, ['2026-06-01']);
    expect(result[0].completedWork).toBe(2);
    expect(result[0].totalSessions).toBe(3);
  });

  it('キーに含まれない日のセッションは無視', () => {
    const sessions = [
      mkSession({ started_at: '2026-05-01T09:00:00Z' }),
    ];
    const result = aggregateByDay(sessions, ['2026-06-01']);
    expect(result[0].totalSessions).toBe(0);
  });

  it('キーの順序が保たれる', () => {
    const result = aggregateByDay([], ['2026-06-03', '2026-06-01', '2026-06-02']);
    expect(result.map((d) => d.date)).toEqual([
      '2026-06-03',
      '2026-06-01',
      '2026-06-02',
    ]);
  });
});

describe('formatDayShort', () => {
  it('M/D (曜) 形式で出力する', () => {
    // 2026-06-01 は月曜日
    expect(formatDayShort('2026-06-01')).toBe('6/1 (月)');
  });

  it('1 桁の日付もパディングなし', () => {
    expect(formatDayShort('2026-06-09')).toMatch(/^6\/9 \(.\)$/);
  });
});
