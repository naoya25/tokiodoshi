<script lang="ts">
  import { tick } from 'svelte';
  import { parseDuration, formatDuration } from '$lib/utils/duration';

  interface Props {
    /** 秒数 */
    value: number;
    /** 確定時に呼ばれる (秒) */
    onChange: (seconds: number) => void;
    /** 入力に対する最小値 (秒)。確定時のクランプにも使う */
    min?: number;
    /** 入力に対する最大値 (秒)。確定時のクランプにも使う */
    max?: number;
    ariaLabel: string;
  }

  const {
    value,
    onChange,
    min = 1,
    max = 24 * 3600,
    ariaLabel,
  }: Props = $props();

  let editing = $state(false);
  let inputValue = $state('');
  let invalid = $state(false);
  let inputEl: HTMLInputElement | undefined = $state();

  async function startEdit() {
    inputValue = formatDuration(value);
    invalid = false;
    editing = true;
    await tick();
    inputEl?.select();
  }

  function commit() {
    const sec = parseDuration(inputValue);
    if (sec === null) {
      invalid = true;
      return;
    }
    const clamped = Math.min(max, Math.max(min, sec));
    invalid = false;
    editing = false;
    if (clamped !== value) {
      onChange(clamped);
    }
  }

  function cancel() {
    editing = false;
    invalid = false;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      commit();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      cancel();
    }
  }
</script>

{#if editing}
  <input
    bind:this={inputEl}
    bind:value={inputValue}
    onkeydown={onKeydown}
    onblur={commit}
    aria-label={ariaLabel}
    class="input"
    class:invalid
    spellcheck="false"
    autocomplete="off"
  />
{:else}
  <button
    type="button"
    class="display"
    onclick={startEdit}
    aria-label="{ariaLabel} {formatDuration(value)} (クリックで編集)"
  >
    {formatDuration(value)}
  </button>
{/if}
<span class="hint">1h / 25m / 10:5 など</span>

<style>
  .display,
  .input {
    font-family: inherit;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    color: inherit;
    background: transparent;
    padding: 6px 8px;
    border-radius: 4px;
    text-align: right;
    min-width: 80px;
  }
  .display {
    border: 1px solid color-mix(in srgb, var(--sumi) 15%, transparent);
    cursor: text;
    letter-spacing: 0.05em;
  }
  .display:hover {
    border-color: color-mix(in srgb, var(--sumi) 35%, transparent);
  }
  .display:focus-visible {
    outline: none;
    border-color: color-mix(in srgb, var(--sumi) 50%, transparent);
  }
  .input {
    border: 1px solid color-mix(in srgb, var(--sumi) 50%, transparent);
    outline: none;
  }
  .input.invalid {
    border-color: color-mix(in srgb, #b85450 60%, transparent);
  }
  .hint {
    margin-left: 8px;
    font-size: 10px;
    letter-spacing: 0.1em;
    opacity: 0.35;
  }
</style>
