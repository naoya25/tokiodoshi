import type { SessionRecord } from '$lib/types';

/** YYYY-MM-DD のキー */
const dateKey = (iso: string): string => iso.slice(0, 10);

export interface DailyAggregate {
  date: string;          // YYYY-MM-DD
  completedWork: number; // 完了した Work セッション数
  totalSessions: number; // 全種類の完了/中断含む
}

/** 直近 N 日 (今日を含む) のキー配列を返す（過去→今日順） */
export const lastNDateKeys = (n: number, today: Date = new Date()): string[] => {
  const keys: string[] = [];
  for (let i = n - 1; i >= 0; i--) {
    const d = new Date(today);
    d.setDate(today.getDate() - i);
    keys.push(d.toISOString().slice(0, 10));
  }
  return keys;
};

/** セッション履歴を日別に集計。指定したキー配列の順序を保つ */
export const aggregateByDay = (
  sessions: SessionRecord[],
  keys: string[],
): DailyAggregate[] => {
  const map = new Map<string, DailyAggregate>();
  for (const k of keys) {
    map.set(k, { date: k, completedWork: 0, totalSessions: 0 });
  }
  for (const s of sessions) {
    const k = dateKey(s.started_at);
    const agg = map.get(k);
    if (!agg) continue;
    agg.totalSessions += 1;
    if (s.type === 'work' && s.was_completed) {
      agg.completedWork += 1;
    }
  }
  return keys.map((k) => map.get(k)!);
};

/** YYYY-MM-DD → "M/D (曜)" 表示 */
export const formatDayShort = (key: string): string => {
  const d = new Date(`${key}T00:00:00`);
  const weekdays = ['日', '月', '火', '水', '木', '金', '土'];
  return `${d.getMonth() + 1}/${d.getDate()} (${weekdays[d.getDay()]})`;
};
