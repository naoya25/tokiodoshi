<script lang="ts">
  import { onMount } from 'svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';

  onMount(() => {
    void settingsStore.init();
  });
</script>

<main>
  <h1>設定</h1>
  <p class="todo">設定 UI は Phase 5 で実装予定</p>

  <pre>{JSON.stringify(settingsStore.settings, null, 2)}</pre>
</main>

<style>
  main {
    padding: 32px;
    font-family: 'Hiragino Mincho ProN', serif;
  }
  h1 {
    font-weight: 300;
    letter-spacing: 0.2em;
    font-size: 18px;
  }
  .todo {
    opacity: 0.4;
    font-size: 11px;
    letter-spacing: 0.2em;
  }
  pre {
    font-family: 'SF Mono', monospace;
    font-size: 11px;
    opacity: 0.5;
    margin-top: 24px;
  }
</style>
