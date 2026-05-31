<script lang="ts">
  interface Props {
    label: string;
    note?: string;
    children: import('svelte').Snippet;
  }

  const { label, note, children }: Props = $props();
</script>

<div class="row">
  <div class="left">
    <div class="label">{label}</div>
    {#if note}
      <div class="note">{note}</div>
    {/if}
  </div>
  <div class="right">
    {@render children()}
  </div>
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    padding: 14px 0;
    border-bottom: 1px solid color-mix(in srgb, var(--sumi) 8%, transparent);
    gap: 24px;
  }
  .left {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .label {
    font-size: 13px;
    letter-spacing: 0.15em;
  }
  .note {
    font-size: 10px;
    letter-spacing: 0.1em;
    opacity: 0.45;
  }
  .right {
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>
