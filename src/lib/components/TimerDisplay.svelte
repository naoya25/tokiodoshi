<script lang="ts">
  interface Props {
    mmss: string;
  }

  const { mmss }: Props = $props();
</script>

<div class="num" aria-live="polite">{mmss}</div>

<style>
  .num {
    font-size: clamp(72px, 12vw, 144px);
    font-weight: 200;
    line-height: 0.9;
    letter-spacing: 0.04em;
    font-variant-numeric: tabular-nums;
  }
</style>
