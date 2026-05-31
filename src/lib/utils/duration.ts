/**
 * 時間文字列を秒数に変換するパーサ。
 *
 * サポートする入力:
 * - 数値のみ: `90` → 90 秒
 * - 単位: `1h` / `25m` / `30s` (小数も可: `1.5h` → 5400)
 * - 複合: `1h30m` / `1h30m45s` / `30m45s`
 * - コロン区切り: `25:00` (MM:SS) / `1:30:00` (HH:MM:SS) / `10:5` (= 10m5s)
 *
 * 失敗時は null を返す (UI 側は元の値に戻す or 無視する)。
 */
export function parseDuration(input: string): number | null {
  const s = input.trim().toLowerCase();
  if (!s) return null;

  // 純粋な数値 (秒として扱う)
  if (/^\d+$/.test(s)) {
    return parseInt(s, 10);
  }

  // 単独単位 (小数対応): "1.5h" / "25m" / "30s"
  const single = s.match(/^(\d+(?:\.\d+)?)\s*(h|m|s)$/);
  if (single) {
    const n = parseFloat(single[1]);
    const u = single[2];
    if (u === 'h') return Math.round(n * 3600);
    if (u === 'm') return Math.round(n * 60);
    if (u === 's') return Math.round(n);
  }

  // 複合単位 "1h30m" / "1h30m45s" / "30m45s"
  // 少なくとも 1 つの単位がある場合のみ採用
  const compound = s.match(/^(?:(\d+)h)?(?:(\d+)m)?(?:(\d+)s)?$/);
  if (compound && (compound[1] || compound[2] || compound[3])) {
    const h = compound[1] ? parseInt(compound[1], 10) : 0;
    const m = compound[2] ? parseInt(compound[2], 10) : 0;
    const sec = compound[3] ? parseInt(compound[3], 10) : 0;
    return h * 3600 + m * 60 + sec;
  }

  // コロン区切り
  const colon = s.match(/^(\d+):(\d+)(?::(\d+))?$/);
  if (colon) {
    const a = parseInt(colon[1], 10);
    const b = parseInt(colon[2], 10);
    if (colon[3] !== undefined) {
      // HH:MM:SS
      const c = parseInt(colon[3], 10);
      return a * 3600 + b * 60 + c;
    }
    // MM:SS
    return a * 60 + b;
  }

  return null;
}

/**
 * 秒数を表示用にフォーマット。
 * - 1 時間未満: `MM:SS` (例: `25:00`, `09:30`)
 * - 1 時間以上: `H:MM:SS` (例: `1:00:00`, `2:30:15`)
 */
export function formatDuration(seconds: number): string {
  const safe = Math.max(0, Math.floor(seconds));
  const h = Math.floor(safe / 3600);
  const m = Math.floor((safe % 3600) / 60);
  const s = safe % 60;
  if (h > 0) {
    return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  }
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
}
