<script lang="ts">
  import { tick } from 'svelte';
  import { parseDuration, formatDuration } from '$lib/utils/duration';

  interface Props {
    /** 表示する秒数 */
    value: number;
    /** 編集できるかどうか (走行中は false にする) */
    editable: boolean;
    /** 確定時に呼ばれる。秒数を受け取る */
    onChange: (seconds: number) => void;
  }

  const { value, editable, onChange }: Props = $props();

  let editing = $state(false);
  let inputValue = $state('');
  let inputEl: HTMLInputElement | undefined = $state();
  let invalid = $state(false);

  async function startEdit() {
    if (!editable || editing) return;
    inputValue = formatDuration(value);
    invalid = false;
    editing = true;
    await tick();
    inputEl?.select();
  }

  function commit() {
    const sec = parseDuration(inputValue);
    if (sec === null || sec <= 0) {
      invalid = true;
      return;
    }
    invalid = false;
    editing = false;
    if (sec !== value) {
      onChange(sec);
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
    aria-label="作業時間"
    class="input"
    class:invalid
    spellcheck="false"
    autocomplete="off"
  />
{:else}
  <button
    type="button"
    class="display"
    class:editable
    onclick={startEdit}
    aria-label={editable ? `作業時間 ${formatDuration(value)} (クリックで編集)` : `タイマー ${formatDuration(value)}`}
    tabindex={editable ? 0 : -1}
  >
    {formatDuration(value)}
  </button>
{/if}

<style>
  .display,
  .input {
    font-family: inherit;
    font-size: clamp(72px, 12vw, 144px);
    font-weight: 200;
    line-height: 0.9;
    letter-spacing: 0.04em;
    font-variant-numeric: tabular-nums;
    color: inherit;
    background: transparent;
    border: none;
    padding: 0;
    margin: 0;
    text-align: left;
  }
  .display {
    cursor: default;
    border-bottom: 1px solid transparent;
    transition: border-color 0.2s ease;
  }
  .display.editable {
    cursor: text;
  }
  .display.editable:hover {
    border-bottom-color: color-mix(in srgb, var(--sumi) 25%, transparent);
  }
  .display:focus-visible {
    outline: none;
    border-bottom-color: color-mix(in srgb, var(--sumi) 50%, transparent);
  }
  .input {
    outline: none;
    border-bottom: 1px solid color-mix(in srgb, var(--sumi) 35%, transparent);
    width: 5ch;
  }
  .input.invalid {
    border-bottom-color: color-mix(in srgb, #b85450 60%, transparent);
  }
</style>
