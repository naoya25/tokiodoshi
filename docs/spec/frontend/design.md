# トキオドシ / フロントエンド — Design

## Technical Approach

SvelteKit (adapter-static / SPA mode) + Svelte 5 runes + TypeScript で構築する。`src/` 配下を以下のレイヤーに分離:

```
┌──────────────────────────────────────────────────────┐
│  routes/ (薄い)                                       │
│    +page.svelte (ししおどしメイン)                   │
│    settings/+page.svelte                             │
│    history/+page.svelte                              │
│        ↓ 参照                                         │
├──────────────────────────────────────────────────────┤
│  lib/components/ (純粋な UI、stores 直接参照)         │
│    ShishiOdoshi.svelte / TimerDisplay.svelte /       │
│    TimerControls.svelte / SettingsForm.svelte         │
│        ↓                                              │
├──────────────────────────────────────────────────────┤
│  lib/stores/ (runes、状態の単一の真実)                │
│    timer.svelte.ts    — フェーズ・残り秒・進行率      │
│    settings.svelte.ts — Settings オブジェクト         │
│        ↓ Rust から受け取る、Rust に送る               │
├──────────────────────────────────────────────────────┤
│  lib/ipc/ (型付き invoke / listen ラッパー)           │
│    timer.ts / settings.ts / history.ts / audio.ts    │
│    events.ts (listener 管理 + cleanup)                │
│        ↓ 使う                                         │
├──────────────────────────────────────────────────────┤
│  lib/types/  Rust 側 models/ に対応する手書き TS 型   │
│  lib/audio/  (将来 v1.1+ で WebAudio fallback、現状空)│
└──────────────────────────────────────────────────────┘
```

**重要原則:**
1. ルートは薄く保つ — コンポーネントを並べて props を渡すだけ
2. コンポーネントは純粋 — IPC を直接呼ばない、store 経由で読み書き
3. store は IPC を呼ぶ単一のエントリ — 副作用集約
4. ipc 層は型付きラッパー — 各コマンド・イベントを関数化

---

## 確認できたこと

- プロトタイプ (`docs/prototype.html`) で SVG の `setAttribute('transform', 'rotate(deg cx cy)')` 方式は軸ズレなく動作する
- `stroke-dashoffset` の linear ループで水の縦流れが自然に表現できる
- `cubic-bezier(0.4, 0, 0.2, 1)` 等のイージングで自然な物理的余韻が出る
- 明朝体 (`Hiragino Mincho ProN`) は和の静謐さに合う
- ライト/ダーク両対応は `prefers-color-scheme` + CSS variables で十分

---

## 推測していること

- Tauri v2 の `@tauri-apps/api/event` の `listen<T>` 戻り値 `UnlistenFn` を Svelte の `$effect` 内で `return` すると自動 cleanup される
- Runes ($state) は `.svelte.ts` 拡張子を持つファイルでのみリアクティブ動作する (Svelte 5 公式)
- バックの SQLite 操作は同期で返ってくる速度感 (10ms 未満) なので、UI スピナーは不要
- `adapter-static` の `fallback: 'index.html'` 設定でルーティングは問題なく動く (実例 5/5 で機能)

---

## 未確認事項

- Tauri v2 で複数ウィンドウ (メイン + 設定) のラベル分離方法 — 単一ウィンドウ内ルートで済ますか別ウィンドウ生成か
- runes と `tweened` / `spring` の組み合わせは可能か (Svelte 5 の motion API 仕様確認)
- Playwright で Tauri アプリを E2E する `tauri-driver` の安定度

---

## Key Decisions

| 論点 | 選択 | 理由 |
|---|---|---|
| ルーティング | SvelteKit ファイルベース | 3 ルート (main / settings / history) で素直に表現できる |
| 状態管理 | Svelte 5 Runes (`.svelte.ts`) | 公式の最新、stores との混在を避ける |
| アニメーション | `requestAnimationFrame` + 手書きイージング | プロトタイプで検証済み、`tweened` だと角度計算の制御が効きにくい |
| SVG 回転 | `setAttribute('transform', ...)` | CSS transform-origin の軸ズレ問題を完全に回避 |
| 設定 UI | 別ウィンドウ (Tauri label="settings") | メインの没入を壊さない、macOS の慣習に従う |
| IPC エラー処理 | console.warn のみ、UI 状態維持 | 静寂を保つ哲学。トーストは出さない |
| TS 型 | Rust から手書き同期 (`lib/types/`) | ts-rs/specta 不採用。型は 4-5 個のみ |
| テスト | Vitest (ユニット) + Playwright + tauri-driver (E2E) | 公式推奨、CI 統合容易 |
| Lint / Format | ESLint (svelte plugin) + Prettier (svelte plugin) | デファクト |
| Dark mode 切替 | CSS variables + `matchMedia` listener | OS 追従、ユーザー上書き両対応 |

---

## UIUX

### メインウィンドウ

- ししおどしを右下 1/3 のエリアに配置 (不均整 fukinsei)
- タイマー数字を左上、ラベル極小、`14.4:1` のサイズ落差で階層を作る
- chrome (操作ボタン) は hover 時のみ表示
- 明朝体 1 ファミリーで統一
- 余白 80%、要素 20% 以下

### 設定画面

- 左ペインに分類 (タイマー / 音 / 表示 / 起動)
- 右ペインに該当項目のフォーム
- 入力即時保存 (デバウンス 200ms)
- 1 列レイアウト、装飾なし
- 設定反映タイミング (即時 / 次セッション / 再起動) は項目横に小さく注記

### 履歴画面 (簡易)

- 日付ごとの完了セッション数を縦リスト
- 当日 + 直近 6 日 = 7 行
- グラフは v1.1+

---

## Component Hierarchy

```
+page.svelte
└── <ShishiOdoshi axisTilt={derived} animating={state} />
└── <TimerDisplay phase={state} remaining={state} />
└── <TimerControls onStart onPause onReset onSkip />

settings/+page.svelte
└── <SettingsForm bind:settings={store} />
    ├── <DurationField />
    ├── <VolumeSlider />
    ├── <AudioModeSelect />
    └── <ThemeSelect />

history/+page.svelte
└── <SessionList sessions={list} />
```

### ShishiOdoshi.svelte の責務

- props: `tilt: number` (現在の傾き角度), `animating: boolean` (カコン演出中フラグ)
- 内部: SVG (water line + bamboo group + axis dot)
- `$effect` で `tilt` 変化に応じて `setAttribute('transform', ...)` を呼ぶ
- カコンアニメは親 (store) が requestAnimationFrame で `tilt` を時間補間する
- 単体テスト容易: 与えた tilt で transform 属性が正しいかを assert

### TimerStore (timer.svelte.ts) の責務

```typescript
class TimerStore {
  remainingSeconds = $state(0);
  phase = $state<Phase>('idle');
  sessionCount = $state(0);
  isAnimating = $state(false);
  tilt = $state(-12);

  progress = $derived(/* 残りと total から計算 */);

  async init() { /* listen 起動、初期 invoke('timer_get_state') */ }
  destroy() { /* unlisten 呼び出し */ }

  async start() { ... }
  async pause() { ... }
  async reset() { ... }
  async skip() { ... }
}
```

---

## IPC Wrapper Design

### lib/ipc/timer.ts

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { TimerState } from '$lib/types';

export const timerStart = () => invoke<TimerState>('timer_start');
export const timerPause = () => invoke<TimerState>('timer_pause');
export const timerReset = () => invoke<TimerState>('timer_reset');
export const timerSkip  = () => invoke<TimerState>('timer_skip');
export const timerGetState = () => invoke<TimerState>('timer_get_state');
```

### lib/ipc/events.ts (リスナー管理)

```typescript
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type EventMap = {
  'timer:tick':          { remaining_seconds: number };
  'timer:state_changed': { phase: Phase; session_count: number };
  'timer:completed':     { type: 'work' | 'short_break' | 'long_break' };
};

export async function on<K extends keyof EventMap>(
  name: K,
  handler: (payload: EventMap[K]) => void
): Promise<UnlistenFn> {
  return listen<EventMap[K]>(name, (e) => handler(e.payload));
}
```

### lib/types/index.ts

```typescript
export type Phase = 'idle' | 'work' | 'short_break' | 'long_break' | 'paused';

export interface TimerState {
  phase: Phase;
  remaining_seconds: number;
  session_count: number;
  current_duration_seconds: number;
}

export type AudioMode = 'silent' | 'kakon_only' | 'full';

export interface Settings {
  durations: { work_seconds: number; /* ... */ };
  audio: { mode: AudioMode; master_volume: number; /* ... */ };
  behavior: { /* ... */ };
  appearance: { theme: 'system' | 'light' | 'dark' };
}

export interface SessionRecord {
  id: number;
  type: 'work' | 'short_break' | 'long_break';
  started_at: string;
  completed_at: string | null;
  was_completed: boolean;
}
```

---

## Data Flow

### 起動時シーケンス

```
1. +layout.svelte $effect で timerStore.init() を呼ぶ
2. timerStore.init() が:
   - invoke('timer_get_state') で初期状態取得
   - listen('timer:tick' / 'timer:state_changed' / 'timer:completed') を起動
3. settingsStore.init() で invoke('settings_get') を呼ぶ
4. テーマ追従の matchMedia リスナー起動
5. ウィンドウ表示完了
```

### Work セッション完了の流れ

```
Rust ticker tick → remaining=0
   ↓ emit
'timer:completed' { type: 'work' }
   ↓ JS listen
TimerStore.handleComplete():
  isAnimating = true
  → playKakon() (requestAnimationFrame で tilt を補間)
    Phase 1: sleep 400ms
    Phase 2: tween tilt to +48° (420ms easeInQuad)
    Phase 3: sleep 180ms
    Phase 4: tween tilt to -25° (310ms easeOutCubic)
    Phase 5: tween tilt to -12° (900ms easeInOutCubic)
    Phase 6: sleep 600ms
  isAnimating = false
   ↓
'timer:state_changed' { phase: 'short_break', ... } を受信
  → phase 更新、tilt は -12° のまま
```

---

## Trade-offs

**優先すること:**
- 静寂と craft の質感を保つ
- バック側との型整合
- アニメ品質 (60fps)

**受け入れる制約:**
- 型は手書き同期 (Rust 側変更時に TS 側も手で直す必要がある)
- IPC エラーで UI が「何も起きないように見える」可能性 (ログでしか追えない)
- Playwright + tauri-driver の E2E はやや脆い (UI 操作の安定性に課題)

---

## Risks & Mitigations

| リスク | 対策 |
|---|---|
| バックエンド型変更時のフロント側型ズレ | `lib/types/` を Rust の `models/` と同期する規約を CONTRIBUTING に明記、PR テンプレに「TS 型同期チェック」項目 |
| WebView バックグラウンド時のアニメ間引き | アニメ本体は emit 駆動なので、バックグラウンド中は emit を受けても describe をスキップする (E1) |
| イベントリスナーリーク | `lib/ipc/events.ts` で UnlistenFn を必ず返す + `$effect` の return で cleanup を強制 |
| Runes と既存ライブラリの非互換 | Svelte 5 対応ライブラリのみ採用、`tweened`/`spring` は使わず手書き rAF tween |
| ダークモード切替時の SVG `currentColor` がアニメ中に取りこぼし | `currentColor` ではなく CSS variable 経由で確実に切り替える |
| `adapter-static` での 404 | `svelte.config.js` で `fallback: 'index.html'` を必ず設定 |
