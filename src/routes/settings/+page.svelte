<script lang="ts">
  import { onMount } from 'svelte';
  import Row from '$lib/components/settings/Row.svelte';
  import Slider from '$lib/components/settings/Slider.svelte';
  import SegmentedControl from '$lib/components/settings/SegmentedControl.svelte';
  import Toggle from '$lib/components/settings/Toggle.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import { applyTheme } from '$lib/utils/theme';
  import type { AudioMode, Theme } from '$lib/types';

  onMount(() => {
    void settingsStore.init();
  });
</script>

<svelte:head>
  <title>トキオドシ / 設定</title>
</svelte:head>

<main>
  <header>
    <h1>設定</h1>
  </header>

  <section>
    <h2>音</h2>

    <Row label="モード">
      <SegmentedControl
        ariaLabel="音モード"
        value={settingsStore.settings.audio.mode}
        options={[
          { value: 'silent' as AudioMode, label: '無音' },
          { value: 'kakon_only' as AudioMode, label: 'カコンのみ' },
          { value: 'full' as AudioMode, label: '水音 + カコン' },
        ]}
        onChange={(v) => settingsStore.updateNested('audio', { mode: v })}
      />
    </Row>

    <Row label="マスター音量">
      <Slider
        ariaLabel="マスター音量"
        value={settingsStore.settings.audio.master_volume}
        onChange={(v) => settingsStore.updateNested('audio', { master_volume: v })}
      />
    </Row>

    <Row label="水音">
      <Slider
        ariaLabel="水音の音量"
        value={settingsStore.settings.audio.water_volume}
        disabled={settingsStore.settings.audio.mode !== 'full'}
        onChange={(v) => settingsStore.updateNested('audio', { water_volume: v })}
      />
    </Row>

    <Row label="カコン">
      <Slider
        ariaLabel="カコン音の音量"
        value={settingsStore.settings.audio.kakon_volume}
        disabled={settingsStore.settings.audio.mode === 'silent'}
        onChange={(v) => settingsStore.updateNested('audio', { kakon_volume: v })}
      />
    </Row>
  </section>

  <section>
    <h2>佇まい</h2>

    <Row label="テーマ">
      <SegmentedControl
        ariaLabel="テーマ"
        value={settingsStore.settings.appearance.theme}
        options={[
          { value: 'system' as Theme, label: '自動' },
          { value: 'light' as Theme, label: '昼' },
          { value: 'dark' as Theme, label: '夜' },
        ]}
        onChange={(v) => {
          settingsStore.updateNested('appearance', { theme: v });
          applyTheme(v);
        }}
      />
    </Row>

    <Row label="セッション開始時にウィンドウを前面化">
      <Toggle
        ariaLabel="セッション開始時にウィンドウを前面化"
        checked={settingsStore.settings.behavior.auto_show_window_on_start}
        onChange={(v) =>
          settingsStore.updateNested('behavior', { auto_show_window_on_start: v })}
      />
    </Row>

    <Row label="ループ再生" note="完了後に自動で次のセッションを開始">
      <Toggle
        ariaLabel="ループ再生"
        checked={settingsStore.settings.behavior.loop_sessions}
        onChange={(v) =>
          settingsStore.updateNested('behavior', { loop_sessions: v })}
      />
    </Row>
  </section>

  <section>
    <h2>起動</h2>

    <Row label="ログイン時に自動起動">
      <Toggle
        ariaLabel="ログイン時に自動起動"
        checked={settingsStore.settings.behavior.launch_at_login}
        onChange={(v) =>
          settingsStore.updateNested('behavior', { launch_at_login: v })}
      />
    </Row>

    <Row label="Dock アイコンを隠す" note="変更には再起動が必要">
      <Toggle
        ariaLabel="Dock アイコンを隠す"
        checked={settingsStore.settings.behavior.hide_dock_icon}
        onChange={(v) =>
          settingsStore.updateNested('behavior', { hide_dock_icon: v })}
      />
    </Row>
  </section>
</main>

<style>
  main {
    max-width: 560px;
    margin: 0 auto;
    padding: 48px 32px 64px;
  }
  header {
    margin-bottom: 40px;
  }
  h1 {
    font-weight: 200;
    letter-spacing: 0.4em;
    font-size: 22px;
    margin: 0;
  }
  section {
    margin-bottom: 40px;
  }
  h2 {
    font-weight: 300;
    letter-spacing: 0.4em;
    font-size: 12px;
    opacity: 0.5;
    margin: 0 0 8px;
  }
</style>
