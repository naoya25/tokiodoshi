<script lang="ts">
  interface Props {
    tilt: number;
  }

  const { tilt }: Props = $props();

  // 軸の SVG 座標。CSS transform ではなく `setAttribute('transform', ...)` で
  // SVG 座標に絶対固定するため、軸ズレが起きない
  const AXIS_X = 200;
  const AXIS_Y = 240;

  let bambooEl: SVGGElement | undefined = $state();

  $effect(() => {
    if (bambooEl) {
      bambooEl.setAttribute('transform', `rotate(${tilt} ${AXIS_X} ${AXIS_Y})`);
    }
  });
</script>

<svg
  class="shishi"
  viewBox="0 0 400 400"
  xmlns="http://www.w3.org/2000/svg"
  role="img"
  aria-label="ししおどし"
>
  <!-- 1. 水（上から落ちる線、筒の先端=水入り口の真上に固定） -->
  <line
    class="water"
    x1="290"
    y1="60"
    x2="290"
    y2="218"
    stroke="currentColor"
    stroke-width="0.8"
    stroke-linecap="round"
    stroke-dasharray="3 6"
    opacity="0.55"
  />

  <!-- 2. 筒（軸を中心に rotate 属性で回転） -->
  <g bind:this={bambooEl}>
    <rect
      x="140"
      y="232"
      width="160"
      height="16"
      fill="none"
      stroke="currentColor"
      stroke-width="1.2"
    />
  </g>

  <!-- 3. 軸（点）— 筒の回転中心と完全一致 -->
  <circle cx={AXIS_X} cy={AXIS_Y} r="2.5" fill="currentColor" />
</svg>

<style>
  .shishi {
    width: 100%;
    height: 100%;
    display: block;
  }

  @keyframes water-fall {
    from {
      stroke-dashoffset: 0;
    }
    to {
      stroke-dashoffset: -9;
    }
  }
  .water {
    animation: water-fall 0.55s linear infinite;
  }
</style>
