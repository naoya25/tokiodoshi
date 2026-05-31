export const formatMmss = (totalSeconds: number): string => {
  const safe = Math.max(0, Math.floor(totalSeconds));
  const m = Math.floor(safe / 60);
  const s = safe % 60;
  return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
};

import type { Phase } from '$lib/types';

export const phaseLabel = (phase: Phase): string => {
  switch (phase) {
    case 'work':
      return '仕';
    case 'short_break':
      return '休';
    case 'long_break':
      return '長休';
    case 'paused':
      return '止';
    case 'idle':
    default:
      return '';
  }
};
