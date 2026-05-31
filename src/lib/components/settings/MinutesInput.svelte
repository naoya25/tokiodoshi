<script lang="ts">
  interface Props {
    /** 秒単位 */
    value: number;
    min?: number;
    max?: number;
    onChange: (seconds: number) => void;
    ariaLabel: string;
  }

  const { value, min = 1, max = 180, onChange, ariaLabel }: Props = $props();

  // 分単位で扱う (UI 側)
  const minutes = $derived(Math.round(value / 60));

  function handleInput(e: Event) {
    const v = parseInt((e.target as HTMLInputElement).value, 10);
    if (Number.isFinite(v)) {
      const clamped = Math.min(max, Math.max(min, v));
      onChange(clamped * 60);
    }
  }
</script>

<input
  type="number"
  {min}
  {max}
  value={minutes}
  oninput={handleInput}
  aria-label={ariaLabel}
/>
<span class="unit">分</span>

<style>
  input {
    width: 60px;
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--sumi) 15%, transparent);
    border-radius: 4px;
    padding: 6px 8px;
    font: inherit;
    color: inherit;
    font-variant-numeric: tabular-nums;
    text-align: right;
    outline: none;
  }
  input:focus {
    border-color: color-mix(in srgb, var(--sumi) 50%, transparent);
  }
  /* スピナーの控えめ化 */
  input::-webkit-inner-spin-button,
  input::-webkit-outer-spin-button {
    opacity: 0.35;
  }
  .unit {
    font-size: 11px;
    letter-spacing: 0.2em;
    opacity: 0.5;
  }
</style>
