<script lang="ts">
  interface Props {
    running: boolean;
    canReset: boolean;
    onToggle: () => void;
    onReset: () => void;
  }

  const { running, canReset, onToggle, onReset }: Props = $props();
</script>

<div class="chrome">
  <button onclick={onToggle} aria-label={running ? '一時停止' : '開始'}>
    {running ? '止' : '始'}
  </button>
  <button onclick={onReset} disabled={!canReset} aria-label="リセット">戻</button>
</div>

<style>
  .chrome {
    display: flex;
    gap: 32px;
    font-size: 10px;
    letter-spacing: 0.3em;
    opacity: 0;
    transition: opacity 0.6s ease;
  }
  :global(body:hover) .chrome,
  :global(body.menu-shown) .chrome {
    opacity: 0.45;
  }

  button {
    background: none;
    border: none;
    font: inherit;
    color: inherit;
    letter-spacing: 0.3em;
    cursor: pointer;
    padding: 0;
    opacity: 0.7;
  }
  button:hover:not(:disabled) {
    opacity: 1;
  }
  button:disabled {
    opacity: 0.2;
    cursor: default;
  }
</style>
