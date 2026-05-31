import type { Theme } from '$lib/types';

const DARK_MQ = '(prefers-color-scheme: dark)';

/** 現在の OS テーマ */
export const systemTheme = (): 'light' | 'dark' =>
  typeof window !== 'undefined' && window.matchMedia(DARK_MQ).matches ? 'dark' : 'light';

/** Theme 設定値 → 実際に適用すべきテーマ */
export const resolveTheme = (theme: Theme): 'light' | 'dark' =>
  theme === 'system' ? systemTheme() : theme;

/** body の `data-theme` 属性で CSS を切り替える */
export const applyTheme = (theme: Theme): void => {
  if (typeof document === 'undefined') return;
  document.documentElement.setAttribute('data-theme', resolveTheme(theme));
};

/**
 * OS のテーマ変更を監視し、Theme 設定が 'system' のときだけ自動反映する。
 * cleanup 関数を返す。
 */
export const watchSystemTheme = (
  getCurrentTheme: () => Theme,
  onChange: (resolved: 'light' | 'dark') => void,
): (() => void) => {
  if (typeof window === 'undefined') return () => {};
  const mq = window.matchMedia(DARK_MQ);
  const handler = () => {
    if (getCurrentTheme() === 'system') {
      onChange(systemTheme());
    }
  };
  mq.addEventListener('change', handler);
  return () => mq.removeEventListener('change', handler);
};
