import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, cleanup, fireEvent } from '@testing-library/svelte';
import TimerControls from './TimerControls.svelte';

afterEach(cleanup);

describe('TimerControls', () => {
  it('停止中は再生アイコン (path 要素)、aria-label="開始"', () => {
    const { getByLabelText } = render(TimerControls, {
      props: {
        running: false,
        canReset: false,
        onToggle: () => {},
        onReset: () => {},
      },
    });
    const btn = getByLabelText('開始');
    expect(btn).toBeTruthy();
    // 再生アイコンは <path> 要素 (三角形)
    expect(btn.querySelector('path')).not.toBeNull();
  });

  it('再生中は一時停止アイコン (line 要素 2本)、aria-label="一時停止"', () => {
    const { getByLabelText } = render(TimerControls, {
      props: {
        running: true,
        canReset: true,
        onToggle: () => {},
        onReset: () => {},
      },
    });
    const btn = getByLabelText('一時停止');
    expect(btn).toBeTruthy();
    // 一時停止アイコンは <line> 2本
    expect(btn.querySelectorAll('line').length).toBe(2);
  });

  it('canReset=false ならリセットボタンは描画されない', () => {
    const { queryByLabelText } = render(TimerControls, {
      props: {
        running: false,
        canReset: false,
        onToggle: () => {},
        onReset: () => {},
      },
    });
    expect(queryByLabelText('リセット')).toBeNull();
  });

  it('canReset=true ならリセットボタンが描画される', () => {
    const { getByLabelText } = render(TimerControls, {
      props: {
        running: false,
        canReset: true,
        onToggle: () => {},
        onReset: () => {},
      },
    });
    expect(getByLabelText('リセット')).toBeTruthy();
  });

  it('再生ボタンクリックで onToggle が呼ばれる', async () => {
    const onToggle = vi.fn();
    const { getByLabelText } = render(TimerControls, {
      props: {
        running: false,
        canReset: false,
        onToggle,
        onReset: () => {},
      },
    });
    await fireEvent.click(getByLabelText('開始'));
    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it('リセットボタンクリックで onReset が呼ばれる', async () => {
    const onReset = vi.fn();
    const { getByLabelText } = render(TimerControls, {
      props: {
        running: false,
        canReset: true,
        onToggle: () => {},
        onReset,
      },
    });
    await fireEvent.click(getByLabelText('リセット'));
    expect(onReset).toHaveBeenCalledTimes(1);
  });
});
