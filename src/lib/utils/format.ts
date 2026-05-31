import type { Phase } from '$lib/types';

export const formatMmss = (totalSeconds: number): string => {
  const safe = Math.max(0, Math.floor(totalSeconds));
  const m = Math.floor(safe / 60);
  const s = safe % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
};

/**
 * Phase をテキストラベルに変換。
 * MVP 現状の UI ではどこからも参照されないが、Tray アイコンや
 * 通知の文言で再利用する可能性があるためユーティリティとして残す。
 */
export const phaseLabel = (phase: Phase): string => {
  switch (phase) {
    case 'work':
      return '仕';
    case 'paused':
      return '止';
    case 'idle':
    default:
      return '';
  }
};
