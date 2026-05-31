<script lang="ts">
  import { onMount } from 'svelte';
  import ShishiOdoshi from '$lib/components/ShishiOdoshi.svelte';
  import TimerDisplay from '$lib/components/TimerDisplay.svelte';
  import TimerControls from '$lib/components/TimerControls.svelte';
  import { timerStore } from '$lib/stores/timer.svelte';
  import { formatMmss } from '$lib/utils/format';

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

  onMount(() => {
    void timerStore.init();
    document.addEventListener('keydown', onKeydown);

    return () => {
      document.removeEventListener('keydown', onKeydown);
      timerStore.destroy();
    };
  });
</script>

<main>
  <div class="timer-cell">
    <TimerDisplay mmss={formatMmss(timerStore.remainingSeconds)} />
  </div>

  <div class="shishi-cell">
    <ShishiOdoshi tilt={timerStore.tilt} />
  </div>
</main>

<TimerControls {running} {canReset} onToggle={handleToggle} onReset={handleReset} />

<style>
  main {
    height: 100vh;
    overflow: hidden;
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
</style>
