<script lang="ts">
  import { onMount } from 'svelte';
  import ShishiOdoshi from '$lib/components/ShishiOdoshi.svelte';
  import EditableDuration from '$lib/components/EditableDuration.svelte';
  import TimerControls from '$lib/components/TimerControls.svelte';
  import { timerStore } from '$lib/stores/timer.svelte';
  import { settingsStore } from '$lib/stores/settings.svelte';
  import * as timerIpc from '$lib/ipc/timer';

  const running = $derived(timerStore.phase === 'work');
  const canReset = $derived(timerStore.phase !== 'idle' || timerStore.sessionCount > 0);
  const isIdle = $derived(timerStore.phase === 'idle');

  // EditableDuration のインスタンス参照 (R リセット時に focus() を呼ぶため)
  let durationInputEl: { focus: () => void } | undefined = $state();

  // Idle かつアニメ完了後 = 「次セッションの長さ」として設定値を見せる。
  // 走行中 / Paused / カコン演出中は現セッションの残り時間を見せる。
  // → アニメ最中 (isAnimating) は remaining_seconds=0 を維持して「00:00」を見せる。
  const displaySeconds = $derived(
    isIdle && !timerStore.isAnimating
      ? settingsStore.settings.durations.work_seconds
      : timerStore.remainingSeconds,
  );

  function handleToggle() {
    if (running) {
      void timerStore.pause();
    } else {
      void timerStore.start();
    }
  }

  async function handleReset() {
    await timerStore.reset();
    // reset 後に明示的に編集モードへ。$effect の editable 変化検知に頼らないことで
    // タイミング問題 (Idle→Idle で発火しない、tick 順序のズレ等) を回避する。
    durationInputEl?.focus();
  }

  function handleSkip() {
    void timerStore.skip();
  }

  /** メイン時計を直接編集して確定したとき。
   *  Idle のときだけ呼ばれる (走行中は editable=false なのでこの handler は呼ばれない)。
   *  設定を保存しつつ、即座にバック側 TimerMachine の config も commit させる。 */
  async function handleDurationChange(seconds: number) {
    settingsStore.updateNested('durations', { work_seconds: seconds });
    try {
      // 設定をすぐ flush し、reset を呼ぶことで pending_config を commit
      // → メイン画面の数字が即新値に反映される
      await timerIpc.timerReset();
      await timerStore.init();
    } catch (e) {
      console.warn('[main] apply duration failed:', e);
    }
  }

  /** Enter で時間を確定したとき: そのままタイマーを開始する */
  async function handleDurationSubmit() {
    // handleDurationChange の reset / init を待ってから start
    await timerStore.start();
  }

  function onKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;

    // R は input にフォーカスがあっても reset として扱う (時間入力に "r" は出てこないため)。
    // これがないと編集モード中の R が input への "r" 入力になりリセットされない。
    if (e.code === 'KeyR' && !e.metaKey && !e.ctrlKey && !e.altKey) {
      e.preventDefault();
      void handleReset();
      return;
    }

    // それ以外のショートカットは input/select の通常入力を妨げない
    if (target.tagName === 'INPUT' || target.tagName === 'SELECT') return;
    if (e.code === 'Space') {
      e.preventDefault();
      handleToggle();
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
    <EditableDuration
      bind:this={durationInputEl}
      value={displaySeconds}
      editable={isIdle}
      onChange={handleDurationChange}
      onSubmit={handleDurationSubmit}
      autoFocus
    />
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
