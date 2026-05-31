<script lang="ts">
  interface Props {
    mmss: string;
    phaseChar: string;
  }

  const { mmss, phaseChar }: Props = $props();
</script>

<div class="timer">
  <div class="num" aria-live="polite">{mmss}</div>
  <div class="label" aria-hidden="true">{phaseChar}</div>
</div>

<style>
  .timer {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .num {
    font-size: clamp(72px, 12vw, 144px);
    font-weight: 200;
    line-height: 0.9;
    letter-spacing: 0.04em;
    font-variant-numeric: tabular-nums;
  }
  .label {
    font-size: 10px;
    letter-spacing: 0.4em;
    margin-top: 16px;
    opacity: 0.5;
  }
</style>
