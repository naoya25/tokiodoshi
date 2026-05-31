<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import Chrome from '$lib/components/Chrome.svelte';
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

    // 起動直後 2 秒だけ chrome を見せる (操作を発見してもらうため)
    document.body.classList.add('menu-shown');
    const t = setTimeout(() => document.body.classList.remove('menu-shown'), 2200);

    return () => {
      cleanup();
      clearTimeout(t);
    };
  });

  // 設定変更時にテーマを即時反映
  $effect(() => {
    applyTheme(settingsStore.settings.appearance.theme);
  });
</script>

<Chrome />

{@render children()}
