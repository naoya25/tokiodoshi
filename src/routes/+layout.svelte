<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { applyTheme, watchSystemTheme } from '$lib/utils/theme';

  let { children } = $props();

  onMount(() => {
    void settingsStore.init().then(() => {
      applyTheme(settingsStore.settings.appearance.theme);
    });

    const cleanup = watchSystemTheme(
      () => settingsStore.settings.appearance.theme,
      () => applyTheme(settingsStore.settings.appearance.theme),
    );

    return cleanup;
  });

  // 設定変更時にテーマを即時反映
  $effect(() => {
    applyTheme(settingsStore.settings.appearance.theme);
  });
</script>

{@render children()}
