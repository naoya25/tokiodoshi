<script lang="ts">
  import { onMount } from 'svelte';
  import { historyList } from '$lib/ipc/history';
  import type { SessionRecord } from '$lib/types';
  import { aggregateByDay, lastNDateKeys, formatDayShort } from '$lib/utils/history';

  let sessions = $state<SessionRecord[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  const keys = lastNDateKeys(7);
  const daily = $derived(aggregateByDay(sessions, keys));
  const total = $derived(daily.reduce((sum, d) => sum + d.completedWork, 0));

  onMount(() => {
    const now = new Date();
    const from = new Date(now);
    from.setDate(now.getDate() - 7);
    historyList(from.toISOString(), now.toISOString())
      .then((r) => {
        sessions = r;
      })
      .catch((e) => {
        console.warn('[history] failed:', e);
        error = '履歴を読み込めませんでした';
      })
      .finally(() => {
        loading = false;
      });
  });
</script>

<svelte:head>
  <title>トキオドシ / 履歴</title>
</svelte:head>

<main>
  <header>
    <h1>履歴</h1>
    <p class="sub">直近 7 日に完了した作業セッション</p>
  </header>

  {#if loading}
    <p class="status">読み込み中...</p>
  {:else if error}
    <p class="status" role="alert">{error}</p>
  {:else}
    <div class="total" aria-live="polite">
      <span class="num">{total}</span>
      <span class="unit">セッション</span>
    </div>

    <ul aria-label="日別セッション数">
      {#each daily as d (d.date)}
        <li>
          <span class="date">{formatDayShort(d.date)}</span>
          <span class="dots" aria-hidden="true">
            {#each Array(d.completedWork) as _, i (i)}
              <span class="dot"></span>
            {/each}
            {#if d.completedWork === 0}
              <span class="empty">—</span>
            {/if}
          </span>
          <span class="count">{d.completedWork}</span>
        </li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  main {
    max-width: 560px;
    margin: 0 auto;
    padding: 48px 32px 64px;
  }
  header {
    margin-bottom: 32px;
  }
  h1 {
    font-weight: 200;
    letter-spacing: 0.4em;
    font-size: 22px;
    margin: 0 0 8px;
  }
  .sub {
    font-size: 11px;
    letter-spacing: 0.15em;
    opacity: 0.5;
    margin: 0;
  }
  .status {
    font-size: 12px;
    opacity: 0.5;
    text-align: center;
    margin: 32px 0;
  }
  .total {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 16px 0;
    margin-bottom: 16px;
  }
  .total .num {
    font-size: 48px;
    font-weight: 200;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.05em;
  }
  .total .unit {
    font-size: 11px;
    letter-spacing: 0.2em;
    opacity: 0.5;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  li {
    display: grid;
    grid-template-columns: 80px 1fr 32px;
    align-items: center;
    padding: 12px 0;
    border-bottom: 1px solid color-mix(in srgb, var(--sumi) 8%, transparent);
    gap: 16px;
  }
  .date {
    font-size: 11px;
    letter-spacing: 0.1em;
    opacity: 0.7;
    font-variant-numeric: tabular-nums;
  }
  .dots {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--sumi);
  }
  .empty {
    font-size: 11px;
    opacity: 0.3;
  }
  .count {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    text-align: right;
    opacity: 0.6;
  }
</style>
