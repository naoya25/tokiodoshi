<script lang="ts">
  interface Props {
    value: number;        // 0.0 - 1.0
    onChange: (v: number) => void;
    ariaLabel: string;
    disabled?: boolean;
  }

  const { value, onChange, ariaLabel, disabled = false }: Props = $props();

  function handleInput(e: Event) {
    const v = parseFloat((e.target as HTMLInputElement).value);
    if (Number.isFinite(v)) {
      onChange(Math.min(1, Math.max(0, v)));
    }
  }
</script>

<input
  type="range"
  min="0"
  max="1"
  step="0.01"
  {value}
  {disabled}
  oninput={handleInput}
  aria-label={ariaLabel}
/>
<span class="percent">{Math.round(value * 100)}</span>

<style>
  input {
    width: 100px;
    accent-color: var(--sumi);
    cursor: pointer;
  }
  input:disabled {
    cursor: not-allowed;
    opacity: 0.3;
  }
  .percent {
    font-size: 10px;
    letter-spacing: 0.1em;
    opacity: 0.5;
    font-variant-numeric: tabular-nums;
    width: 22px;
    text-align: right;
  }
</style>
