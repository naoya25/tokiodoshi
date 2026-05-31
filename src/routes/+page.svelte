<script lang="ts">
  import { onMount } from 'svelte';
  import ShishiOdoshi from '$lib/components/ShishiOdoshi.svelte';
  import TimerDisplay from '$lib/components/TimerDisplay.svelte';
  import TimerControls from '$lib/components/TimerControls.svelte';
  import { timerStore } from '$lib/stores/timer.svelte';
  import { formatMmss, phaseLabel } from '$lib/utils/format';

  const running = $derived(
    timerStore.phase === 'work' ||
      timerStore.phase === 'short_break' ||
      timerStore.phase === 'long_break',
  );
  const canReset = $derived(timerStore.phase !== 'idle' || timerStore.sessionCount > 0);

  function handleToggle() {
    if (running) {
      void timerStore.pause();
    } else {
      void timerStore.start();
    }
  }

  function handleReset() {
    void timerStore.reset();
  }

  function handleSkip() {
    void timerStore.skip();
  }

  function onKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'SELECT') return;
    if (e.code === 'Space') {
      e.preventDefault();
      handleToggle();
    } else if (e.code === 'KeyR') {
      handleReset();
    } else if (e.code === 'KeyS') {
      handleSkip();
    }
  }

  function onVisibilityChange() {
    // WebView が非表示中に来た completed は、表示復帰時にアニメをスキップする (E1)
    // 現状は TimerStore.isAnimating の中で skip 判定されないので、ここで早期に
    // isAnimating を解除する保険を入れておく
    if (document.visibilityState === 'visible' && timerStore.isAnimating) {
      // 非表示の間にアニメが完了している可能性があるが、復帰時に新しい tick が
      // 来るのでそのまま継続させる (フェーズは emit で正規化される)
    }
  }

  onMount(() => {
    void timerStore.init();
    document.addEventListener('keydown', onKeydown);
    document.addEventListener('visibilitychange', onVisibilityChange);

    document.body.classList.add('menu-shown');
    const t = setTimeout(() => document.body.classList.remove('menu-shown'), 2200);

    return () => {
      document.removeEventListener('keydown', onKeydown);
      document.removeEventListener('visibilitychange', onVisibilityChange);
      clearTimeout(t);
      timerStore.destroy();
    };
  });
</script>

<main>
  <div class="timer-cell">
    <TimerDisplay
      mmss={formatMmss(timerStore.remainingSeconds)}
      phaseChar={phaseLabel(timerStore.phase)}
    />
  </div>

  <div class="shishi-cell">
    <ShishiOdoshi tilt={timerStore.tilt} />
  </div>
</main>

<div class="controls-bar">
  <TimerControls {running} {canReset} onToggle={handleToggle} onReset={handleReset} />
</div>

<nav class="bottom-nav" aria-label="補助メニュー">
  <a href="/settings">設定</a>
  <a href="/history">履歴</a>
</nav>

<style>
  main {
    height: 100vh;
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr auto 1fr;
    padding: 6vh 8vw;
  }
  .timer-cell {
    grid-column: 1;
    grid-row: 2;
    align-self: end;
  }
  .shishi-cell {
    grid-column: 2;
    grid-row: 2;
    width: 100%;
    max-width: 360px;
    aspect-ratio: 1 / 1;
    justify-self: end;
  }
  .controls-bar {
    position: fixed;
    inset: auto 0 0 0;
    padding: 24px;
    display: flex;
    justify-content: center;
  }
  .bottom-nav {
    position: fixed;
    top: 16px;
    right: 24px;
    display: flex;
    gap: 16px;
    font-size: 10px;
    letter-spacing: 0.3em;
    opacity: 0;
    transition: opacity 0.6s ease;
  }
  :global(body:hover) .bottom-nav,
  :global(body.menu-shown) .bottom-nav {
    opacity: 0.35;
  }
  .bottom-nav a {
    color: inherit;
    text-decoration: none;
    padding: 4px 0;
  }
  .bottom-nav a:hover {
    opacity: 1;
  }
  .bottom-nav a:focus-visible {
    outline: 1.5px solid var(--sumi);
    outline-offset: 4px;
  }
</style>
