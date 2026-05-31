<script lang="ts">
  import { onMount } from 'svelte';
  import { historyList } from '$lib/ipc/history';
  import type { SessionRecord } from '$lib/types';

  let sessions = $state<SessionRecord[]>([]);

  onMount(() => {
    const now = new Date();
    const from = new Date(now);
    from.setDate(now.getDate() - 7);
    historyList(from.toISOString(), now.toISOString())
      .then((r) => (sessions = r))
      .catch((e) => console.warn('[history] failed:', e));
  });
</script>

<main>
  <h1>履歴</h1>
  <p class="todo">履歴 UI は Phase 6 で実装予定</p>

  {#if sessions.length === 0}
    <p class="empty">まだ記録はありません</p>
  {:else}
    <ul>
      {#each sessions as s (s.id)}
        <li>
          {s.started_at} — {s.type} — {s.was_completed ? '完了' : '中断'}
        </li>
      {/each}
    </ul>
  {/if}
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
  .todo,
  .empty {
    opacity: 0.4;
    font-size: 11px;
    letter-spacing: 0.2em;
  }
  ul {
    list-style: none;
    padding: 0;
    font-size: 12px;
  }
</style>
