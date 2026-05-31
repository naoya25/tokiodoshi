<script lang="ts">
  import { page } from '$app/state';

  // 各画面で共通の補助ナビゲーション。
  // - body:hover または body.menu-shown のときだけ薄く表示
  // - 現在のルートのリンクは隠す (どこにいるか分かるように)
  const isMain     = $derived(page.url.pathname === '/');
  const isSettings = $derived(page.url.pathname.startsWith('/settings'));
  const isHistory  = $derived(page.url.pathname.startsWith('/history'));
</script>

<nav class="chrome-nav" aria-label="補助メニュー">
  {#if !isMain}
    <a href="/">主</a>
  {/if}
  {#if !isSettings}
    <a href="/settings">設定</a>
  {/if}
  {#if !isHistory}
    <a href="/history">履歴</a>
  {/if}
</nav>

<style>
  .chrome-nav {
    position: fixed;
    top: 16px;
    right: 24px;
    display: flex;
    gap: 16px;
    font-size: 10px;
    letter-spacing: 0.3em;
    opacity: 0;
    transition: opacity 0.6s ease;
    z-index: 10;
  }
  :global(body:hover) .chrome-nav,
  :global(body.menu-shown) .chrome-nav {
    opacity: 0.45;
  }
  a {
    color: inherit;
    text-decoration: none;
    padding: 4px 0;
  }
  a:hover {
    opacity: 1;
  }
  a:focus-visible {
    outline: 1.5px solid var(--sumi);
    outline-offset: 4px;
  }
</style>
