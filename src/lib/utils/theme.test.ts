import { describe, it, expect, vi, afterEach } from 'vitest';
import { resolveTheme, applyTheme, watchSystemTheme, systemTheme } from './theme';

function mockMatchMedia(matches: boolean) {
  let listener: ((e: { matches: boolean }) => void) | null = null;
  const mq = {
    matches,
    addEventListener: (_t: string, cb: () => void) => {
      listener = cb;
    },
    removeEventListener: vi.fn(),
  };
  vi.stubGlobal('matchMedia', vi.fn(() => mq));
  vi.stubGlobal('window', { matchMedia: vi.fn(() => mq) });
  return {
    mq,
    triggerChange: (newMatches: boolean) => {
      mq.matches = newMatches;
      listener?.({ matches: newMatches });
    },
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
  document.documentElement.removeAttribute('data-theme');
});

describe('systemTheme', () => {
  it('matchMedia が dark に match すれば dark', () => {
    mockMatchMedia(true);
    expect(systemTheme()).toBe('dark');
  });

  it('matchMedia が match しなければ light', () => {
    mockMatchMedia(false);
    expect(systemTheme()).toBe('light');
  });
});

describe('resolveTheme', () => {
  it('light/dark は素通し', () => {
    expect(resolveTheme('light')).toBe('light');
    expect(resolveTheme('dark')).toBe('dark');
  });

  it("system は systemTheme の値を返す", () => {
    mockMatchMedia(true);
    expect(resolveTheme('system')).toBe('dark');
  });
});

describe('applyTheme', () => {
  it('document に data-theme 属性をセットする', () => {
    applyTheme('light');
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    applyTheme('dark');
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });
});

describe('watchSystemTheme', () => {
  it('system モードのときだけ onChange が呼ばれる', () => {
    const { triggerChange } = mockMatchMedia(false);
    const onChange = vi.fn();
    let currentTheme: 'system' | 'light' = 'system';
    const cleanup = watchSystemTheme(() => currentTheme, onChange);

    triggerChange(true);
    expect(onChange).toHaveBeenCalledWith('dark');

    onChange.mockClear();
    currentTheme = 'light';
    triggerChange(false);
    expect(onChange).not.toHaveBeenCalled();

    cleanup();
  });

  it('cleanup 関数が removeEventListener を呼ぶ', () => {
    const { mq } = mockMatchMedia(false);
    const cleanup = watchSystemTheme(
      () => 'system',
      () => {},
    );
    cleanup();
    expect(mq.removeEventListener).toHaveBeenCalled();
  });
});
