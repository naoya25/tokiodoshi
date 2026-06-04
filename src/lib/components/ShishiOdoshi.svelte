<script lang="ts">
  interface Props {
    tilt: number;
    /** Pause 中なら水流を止める (上から徐々にフェード、下の水滴は流れ切る) */
    paused?: boolean;
  }

  const { tilt, paused = false }: Props = $props();

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
  class:paused
  viewBox="0 0 400 400"
  xmlns="http://www.w3.org/2000/svg"
  role="img"
  aria-label="ししおどし"
>
  <defs>
    <!--
      水流マスク: rect の y を 60 (= line.y1) から 220 (= line.y2 より下) に動かすことで、
      水を「上から徐々に消す」効果を作る。
      下方の水滴は流れ続けるので「最後の一滴まで落ちきってから止まる」見え方になる。
    -->
    <mask id="water-mask" maskUnits="userSpaceOnUse" x="280" y="0" width="20" height="400">
      <rect
        class="water-mask-rect"
        x="280"
        width="20"
        height="160"
        fill="white"
      />
    </mask>
  </defs>

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
    mask="url(#water-mask)"
  />

  <!-- 2. 筒（軸を中心に rotate 属性で回転）
       右端は斜めにカット（開口部が上向き=水を受ける向き）。
       top edge は x:290 で終わり、bottom tip が x:314 まで伸びる。 -->
  <g bind:this={bambooEl}>
    <!-- 本体: 右端を斜めにカットした筒（cut faces up） -->
    <path
      d="M 140 232 L 290 232 L 314 248 L 140 248 Z"
      fill="none"
      stroke="currentColor"
      stroke-width="1.2"
      stroke-linejoin="round"
    />

    <!-- 斜めカットの内側リム: カット線の少し内側に沿った曲線で中空の円筒感を匂わせる -->
    <path
      d="M 292 234 Q 302 242 311 247"
      fill="none"
      stroke="currentColor"
      stroke-width="0.8"
      opacity="0.4"
    />

    <!-- 左端の節 (閉じた cap): 内側に薄いカーブを置いて円筒の奥行きを示す -->
    <path
      d="M 142 234 Q 145 240 142 246"
      fill="none"
      stroke="currentColor"
      stroke-width="0.7"
      opacity="0.5"
    />

    <!-- 竹の節 (節間の境界線) -->
    <line
      x1="215"
      y1="232"
      x2="215"
      y2="248"
      stroke="currentColor"
      stroke-width="0.7"
      opacity="0.55"
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

  /* マスク矩形を「上から下へ」スライドさせて水を消す。
     paused へ向かう時だけ transition を効かせて、resume 時は瞬時に戻すことで
     「下から上へ水が登る」逆再生に見えるのを防ぐ。
     resume 後は通常の dashoffset アニメで自然に「上から下へ流れる」見え方になる。 */
  .water-mask-rect {
    y: 60px;
  }
  .shishi.paused .water-mask-rect {
    y: 220px;
    transition: y 1000ms ease-out;
  }
</style>
