# トキオドシ / フロントエンド — Tasks

凡例: `[P]` = 並列実行可、`(Xh)` = 工数見積もり (人間時間)、`DoD` = Definition of Done。
依存関係はフェーズ番号で表現。同フェーズ内は並列可。

---

## Phase 0: セットアップ (依存なし)

- [ ] **T0.1** SvelteKit プロジェクト初期化と Tauri 統合 (1h)
  - `npm create tauri-app@latest tokiodoshi -- --template svelte-ts --identifier com.naoya.tokiodoshi` で雛形を `~/Desktop/Repos/tokiodoshi/` (既存ディレクトリ) に統合
  - `npm install` 成功
  - `npm run tauri dev` で空ウィンドウ起動
  - DoD: 起動できる + `gen/schemas/` が生成される
  - Files: `package.json`, `svelte.config.js`, `vite.config.ts`, `tauri.conf.json`

- [ ] **T0.2** [P] adapter-static 設定 (0.3h)
  - `svelte.config.js`: `adapter-static` + `fallback: 'index.html'` + `paths.base = ''`
  - `src/routes/+layout.ts`: `export const ssr = false; export const prerender = false;`
  - DoD: `npm run build` 成功 + `build/index.html` 生成

- [ ] **T0.3** [P] TypeScript / ESLint / Prettier 設定 (0.5h)
  - `tsconfig.json` の strict 有効
  - `eslint-plugin-svelte` + `prettier-plugin-svelte` 導入
  - DoD: `npm run lint` / `npm run format` がエラーなしで通る

- [ ] **T0.4** [P] Vitest セットアップ (0.5h)
  - `vitest`, `@vitest/coverage-v8`, `jsdom`, `@testing-library/svelte` 追加
  - `vitest.config.ts` 作成
  - サンプルテスト 1 本 (例: `src/lib/utils/format.test.ts`)
  - DoD: `npm run test` で 1 件 pass

- [ ] **T0.5** [P] Playwright + tauri-driver セットアップ (1h)
  - `playwright.config.ts` 作成、`tauri-driver` インストール
  - 雛形 E2E (ウィンドウ起動確認) 1 本
  - DoD: `npm run test:e2e` で 1 件 pass

---

## Phase 1: 型 + IPC ラッパー (Phase 0 完了後)

- [ ] **T1.1** [P] TS 型定義 (1h)
  - `src/lib/types/index.ts`: `Phase`, `TimerState`, `Settings`, `AudioMode`, `SessionRecord`, `EventMap`
  - バック spec の `models/` と整合
  - DoD: `tsc --noEmit` でエラーなし

- [ ] **T1.2** [P] IPC コマンドラッパー (1h)
  - `src/lib/ipc/timer.ts` — `timerStart` / `timerPause` / `timerReset` / `timerSkip` / `timerGetState`
  - `src/lib/ipc/settings.ts` — `settingsGet` / `settingsSet`
  - `src/lib/ipc/history.ts` — `historyList`
  - `src/lib/ipc/audio.ts` — `audioSetMode` / `audioSetVolume`
  - 全関数が `Promise<TYPE>` を返す型付き wrapper
  - DoD: 型エラーなし + 各 wrapper のユニットテスト (mock invoke で戻り値型を検証)

- [ ] **T1.3** [P] イベントリスナーラッパー (0.5h)
  - `src/lib/ipc/events.ts` — `on<K extends keyof EventMap>(name, handler): Promise<UnlistenFn>`
  - DoD: 型エラーなし + ユニットテスト (listen mock で正しい event 名・handler 呼び出しを検証)

---

## Phase 2: 状態管理 (runes ストア) (Phase 1 完了後)

- [ ] **T2.1** TimerStore 実装 (2h)
  - `src/lib/stores/timer.svelte.ts`
  - `$state`: `remainingSeconds`, `phase`, `sessionCount`, `tilt`, `isAnimating`
  - `$derived`: `progress`, `mmss`
  - メソッド: `init()` / `destroy()` / `start()` / `pause()` / `reset()` / `skip()` / `playKakon()`
  - `init()` で `timer_get_state` + 3 種 listen 起動
  - DoD: 各メソッドのユニットテスト + IPC mock で動作確認 (5 ケース以上)
  - File: `src/lib/stores/timer.svelte.ts`

- [ ] **T2.2** SettingsStore 実装 (1.5h)
  - `src/lib/stores/settings.svelte.ts`
  - `$state`: `settings: Settings`
  - `init()` で `settings_get`、`update(partial)` で `settings_set` (200ms デバウンス)
  - DoD: ユニットテスト 3 ケース (init / update / debounce)
  - File: `src/lib/stores/settings.svelte.ts`

- [ ] **T2.3** [P] easing / tween ユーティリティ (1h)
  - `src/lib/utils/easing.ts` — `easeInOutCubic`, `easeInQuad`, `easeOutCubic`
  - `src/lib/utils/tween.ts` — `tween(fromDeg, toDeg, durationMs, easing): Promise<void>` (requestAnimationFrame ベース)
  - DoD: ユニットテスト (各 easing の境界値、tween の cancel 機能)

---

## Phase 3: SVG コンポーネント (Phase 2 完了後)

- [ ] **T3.1** ShishiOdoshi.svelte (2.5h)
  - プロトタイプ (`docs/prototype.html`) の SVG を Svelte コンポーネントに移植
  - Props: `tilt: number`
  - 内部 `$effect` で SVG `<g id="bamboo">` の `transform` 属性を `setAttribute` で更新
  - 水流の `<line>` に `stroke-dashoffset` CSS animation
  - 軸座標 (200, 240) を SVG 内に固定
  - DoD: コンポーネントテスト 4 ケース (tilt=-12/0/+12/+48 で正しい transform 属性)
  - File: `src/lib/components/ShishiOdoshi.svelte`

- [ ] **T3.2** [P] TimerDisplay.svelte (1h)
  - Props: `mmss: string`, `phaseLabel: string` (漢字 1 字 "仕/休/長休")
  - 明朝体、巨大数字 + 極小ラベル
  - DoD: 描画テスト + アクセシビリティ (aria-live="polite")
  - File: `src/lib/components/TimerDisplay.svelte`

- [ ] **T3.3** [P] TimerControls.svelte (0.5h)
  - Props: callbacks 4 個 (`onStart` / `onPause` / `onReset` / `onSkip`) + `running: boolean`
  - hover 時のみ薄く表示する chrome
  - DoD: クリック・キーボード Tab 確認

---

## Phase 4: メインウィンドウルート (Phase 3 完了後)

- [ ] **T4.1** メインページ (`+page.svelte`) 実装 (1.5h)
  - レイアウト: 不均整グリッド (左下タイマー / 右下ししおどし)
  - timerStore を $effect で init/destroy
  - `<ShishiOdoshi tilt={timerStore.tilt} />` を配置
  - キーボードショートカット (Space / R / S) を `document.addEventListener` で
  - DoD: 手動操作で Start → Tick が動く (バック未実装でも mock invoke でOK)
  - File: `src/routes/+page.svelte`

- [ ] **T4.2** [P] レイアウト (`+layout.svelte`) 実装 (0.5h)
  - CSS variables の定義 (`--ji`, `--sumi`)
  - フォント import (明朝体)
  - DoD: ライト/ダーク切替で配色が反映される

- [ ] **T4.3** [P] テーマ追従 (1h)
  - `src/lib/utils/theme.ts` — matchMedia + manual override の解決ロジック
  - settings の `theme: system/light/dark` に従う
  - DoD: ユニットテスト 3 ケース (system / light / dark) + 手動で OS テーマ切替して即時反映確認

---

## Phase 5: 設定画面 (Phase 4 完了後)

- [ ] **T5.1** 設定ページルート (1h)
  - `src/routes/settings/+page.svelte`
  - 左ペイン分類 / 右ペインフォーム
  - settingsStore で双方向バインド
  - DoD: 全項目が表示される + 入力で IPC が呼ばれる
  - File: `src/routes/settings/+page.svelte`

- [ ] **T5.2** [P] SettingsForm コンポーネント分割 (1.5h)
  - `<DurationField />`, `<VolumeSlider />`, `<AudioModeSelect />`, `<ThemeSelect />` を切り出し
  - 各フィールドに「反映タイミング」注記 (即時 / 次セッション / 要再起動)
  - DoD: 各 4 コンポーネントの単体テスト
  - Files: `src/lib/components/settings/*.svelte`

---

## Phase 6: 履歴画面 (Phase 5 と並列可)

- [ ] **T6.1** 履歴ページ (1h)
  - `src/routes/history/+page.svelte`
  - 当日 + 直近 6 日のセッション件数一覧 (テキストのみ、グラフ無し)
  - `historyList` を mount 時に呼ぶ
  - DoD: 表示確認 + 空状態の表示
  - File: `src/routes/history/+page.svelte`

---

## Phase 7: 結合・微調整 (Phase 4-6 完了後)

- [ ] **T7.1** バックエンドとの結合 (2h)
  - 全 IPC 動作確認 (start / pause / reset / skip)
  - 全 listen 動作確認 (tick / state_changed / completed)
  - 設定変更が即時反映される
  - 履歴が SQLite から読める
  - DoD: バックエンド spec の Phase 8 完了とセットで MVP 動作

- [ ] **T7.2** [P] WebView 非表示時の挙動 (1h)
  - E1 対応: 非表示中 completed を受信したらアニメスキップ
  - DoD: 手動で非表示 → 表示確認

- [ ] **T7.3** [P] エラーハンドリング (0.5h)
  - 全 IPC ラッパーで try/catch → console.warn
  - DoD: バック停止時に UI がクラッシュしない

---

## Phase 8: テスト (Phase 7 完了後)

### 8.A ユニット (Vitest)

- [ ] **T8.1** [P] ipc ラッパー全関数テスト (1h)
  - `lib/ipc/*.test.ts`
  - mock `invoke` で型・引数・戻り値を検証
  - DoD: 全 wrapper に 1 ケース以上、カバレッジ 95%

- [ ] **T8.2** [P] easing / tween テスト (0.5h)
  - 境界値、cancel、複数回呼出
  - DoD: 6 ケース以上 pass

- [ ] **T8.3** [P] TimerStore テスト (2h)
  - 状態遷移 6 ケース: init / start / pause / reset / skip / tick受信
  - completed 受信時に playKakon が呼ばれる
  - destroy で unlisten が呼ばれる
  - DoD: 全ケース pass、カバレッジ 80%

- [ ] **T8.4** [P] SettingsStore テスト (1h)
  - init / update / debounce 動作 / 不正値拒否
  - DoD: 4 ケース pass

### 8.B コンポーネント (Vitest + @testing-library/svelte)

- [ ] **T8.5** [P] ShishiOdoshi 描画テスト (1h)
  - 各 tilt 値で SVG transform 属性が正しい
  - currentColor の DOM 解決確認
  - DoD: 4 ケース pass

### 8.C E2E (Playwright + tauri-driver)

- [ ] **T8.6** フルセッションフロー (2h)
  - Start → tick が表示更新 → completed でアニメ再生 → 次フェーズへ
  - デモ速度を爆速に設定して 1 分以内に完走
  - DoD: 1 ケース pass

- [ ] **T8.7** [P] 設定変更フロー (1h)
  - 音量変更が即時 IPC 発火
  - 音モード切替で水音停止確認
  - DoD: 2 ケース pass

- [ ] **T8.8** [P] 一時停止・再開・リセット (1h)
  - Pause / Reset / Skip 操作の UI 確認
  - DoD: 3 ケース pass

---

## Phase 9: 仕上げ

- [ ] **T9.1** アクセシビリティ確認 (1h)
  - VoiceOver でタイマー読み上げ確認
  - Tab フォーカス順序確認
  - DoD: 主要操作が VoiceOver で完結

- [ ] **T9.2** [P] パフォーマンス計測 (0.5h)
  - DevTools Performance でアニメ中 CPU 5% 以下確認
  - FCP 1.5 秒以内確認
  - DoD: 数値が目標値以内

- [ ] **T9.3** [P] バンドルサイズ確認 (0.3h)
  - `npm run build` 後の gzipped サイズが 200KB 以下
  - DoD: サイズ確認

---

## Verification Checklist

- [ ] `requirements.md` の F1-F7 と E1-E3 を満たす
- [ ] `design.md` のコンポーネント階層とデータフローと一致
- [ ] Vitest カバレッジ 70%+
- [ ] Playwright E2E が CI で通る
- [ ] バックエンド spec と IPC 整合性が取れている
- [ ] アニメ品質が prototype.html 同等以上
- [ ] ダークモード OS 追従確認
- [ ] VoiceOver 動作確認

---

## 工数サマリ

| Phase | 工数 |
|---|---|
| Phase 0 (セットアップ) | 3.3h |
| Phase 1 (型 + IPC) | 2.5h |
| Phase 2 (ストア) | 4.5h |
| Phase 3 (SVG コンポーネント) | 4h |
| Phase 4 (メイン) | 3h |
| Phase 5 (設定) | 2.5h |
| Phase 6 (履歴) | 1h |
| Phase 7 (結合) | 3.5h |
| Phase 8 (テスト) | 9.5h |
| Phase 9 (仕上げ) | 1.8h |
| **合計** | **35.6h ≒ 4.5 人日** |

---

## Progress Log

| Date | Status | Notes |
|---|---|---|
| 2026-06-01 | Draft | Spec 作成完了。実装着手前 |
