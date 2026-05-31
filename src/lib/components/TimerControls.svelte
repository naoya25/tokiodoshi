<script lang="ts">
  interface Props {
    running: boolean;
    canReset: boolean;
    onToggle: () => void;
    onReset: () => void;
  }

  const { running, canReset, onToggle, onReset }: Props = $props();
</script>

<div class="controls" data-testid="timer-controls">
  {#if canReset}
    <button
      class="icon-btn reset"
      onclick={onReset}
      aria-label="リセット"
      type="button"
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <!-- 回転矢印 (反時計回り) -->
        <path
          d="M5 10 A 7 7 0 1 1 6 16"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
        />
        <path
          d="M5 6 L5 10 L9 10"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
  {/if}

  <button
    class="icon-btn toggle"
    onclick={onToggle}
    aria-label={running ? '一時停止' : '開始'}
    type="button"
  >
    <svg viewBox="0 0 24 24" aria-hidden="true">
      {#if running}
        <!-- 一時停止 -->
        <line x1="9"  y1="6" x2="9"  y2="18" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        <line x1="15" y1="6" x2="15" y2="18" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      {:else}
        <!-- 再生 (三角形、塗りなし線画) -->
        <path
          d="M8 5 L19 12 L8 19 Z"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linejoin="round"
          stroke-linecap="round"
        />
      {/if}
    </svg>
  </button>
</div>

<style>
  .controls {
    position: fixed;
    right: 32px;
    bottom: 32px;
    display: flex;
    align-items: center;
    gap: 16px;
    opacity: 0;
    transition: opacity 0.6s ease;
    z-index: 10;
  }
  :global(body:hover) .controls,
  :global(body.menu-shown) .controls {
    opacity: 0.55;
  }
  .controls:hover {
    opacity: 1 !important;
  }

  .icon-btn {
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--sumi) 20%, transparent);
    border-radius: 50%;
    color: inherit;
    cursor: pointer;
    padding: 0;
    transition: border-color 0.15s ease, background 0.15s ease;
    display: grid;
    place-items: center;
  }
  .icon-btn:hover {
    border-color: color-mix(in srgb, var(--sumi) 50%, transparent);
    background: color-mix(in srgb, var(--sumi) 4%, transparent);
  }
  .icon-btn:focus-visible {
    outline: 1.5px solid var(--sumi);
    outline-offset: 3px;
  }

  .toggle {
    width: 56px;
    height: 56px;
  }
  .toggle svg {
    width: 22px;
    height: 22px;
  }

  .reset {
    width: 36px;
    height: 36px;
  }
  .reset svg {
    width: 16px;
    height: 16px;
  }
</style>
