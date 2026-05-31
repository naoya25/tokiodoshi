import { describe, it, expect, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import ShishiOdoshi from './ShishiOdoshi.svelte';

afterEach(cleanup);

describe('ShishiOdoshi', () => {
  it('SVG が描画される', () => {
    const { container } = render(ShishiOdoshi, { props: { tilt: 0 } });
    const svg = container.querySelector('svg');
    expect(svg).not.toBeNull();
    expect(svg?.getAttribute('viewBox')).toBe('0 0 400 400');
  });

  it('tilt=-12 で transform が rotate(-12 200 240)', async () => {
    const { container } = render(ShishiOdoshi, { props: { tilt: -12 } });
    // $effect は次の microtask で走るので待つ
    await Promise.resolve();
    await Promise.resolve();
    const bamboo = container.querySelector('svg > g');
    expect(bamboo?.getAttribute('transform')).toBe('rotate(-12 200 240)');
  });

  it('tilt=0 で transform が rotate(0 200 240)', async () => {
    const { container } = render(ShishiOdoshi, { props: { tilt: 0 } });
    await Promise.resolve();
    await Promise.resolve();
    const bamboo = container.querySelector('svg > g');
    expect(bamboo?.getAttribute('transform')).toBe('rotate(0 200 240)');
  });

  it('tilt=48 で transform が rotate(48 200 240)', async () => {
    const { container } = render(ShishiOdoshi, { props: { tilt: 48 } });
    await Promise.resolve();
    await Promise.resolve();
    const bamboo = container.querySelector('svg > g');
    expect(bamboo?.getAttribute('transform')).toBe('rotate(48 200 240)');
  });

  it('軸の円が SVG 座標 (200, 240) にある', () => {
    const { container } = render(ShishiOdoshi, { props: { tilt: 0 } });
    const axisCircle = container.querySelector('svg > circle');
    expect(axisCircle?.getAttribute('cx')).toBe('200');
    expect(axisCircle?.getAttribute('cy')).toBe('240');
  });

  it('水のラインが筒の先端 (x=290) に固定されている', () => {
    const { container } = render(ShishiOdoshi, { props: { tilt: 0 } });
    const waterLine = container.querySelector('svg > line');
    expect(waterLine?.getAttribute('x1')).toBe('290');
    expect(waterLine?.getAttribute('x2')).toBe('290');
  });
});
