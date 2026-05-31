# トキオドシ / フロントエンド — Requirements

**優先度:** Critical
**対象ユーザー:** 集中作業を業務の中心とする knowledge worker (エンジニア / デザイナー / ライター)
**対象モジュール:** SvelteKit フロント全体 (`src/`)
**親仕様:** `docs/requirements.md` (v0.3)
**前段プロトタイプ:** `docs/prototype.html`

---

## Problem

### 現在起きていること

プロトタイプ (`docs/prototype.html`) はビジュアル・アニメ・操作感の検証だけが目的の単一 HTML。以下が未実装:

- Tauri バックエンドとの IPC が無い (`@tauri-apps/api` への置き換えが必要)
- 永続化が無い (リロードで設定・状態が消える)
- 設定変更 UI が無い (デモ速度切替のみ)
- 履歴閲覧 UI が無い
- システム OS 連携が無い (テーマ追従は最小限のみ)
- テストが無い (Vitest / Playwright 未導入)

ししおどしの SVG アニメーションと操作感の方向性はプロトタイプで承認済みなので、これを Svelte コンポーネントに移植しつつ、Tauri 統合・テスト・状態管理を整備する。

### なぜ問題なのか

プロトタイプのままでは MVP 配布できない:
- ブラウザ単体では macOS のメニューバー常駐・SQLite 永続化・ログイン時起動・rodio 音再生が一切できない
- バックエンドのタイマーロジック (Rust 側 `core/timer_machine.rs`) と接続する責務がフロントにある
- リファクタを後回しにすると、状態管理 (旧 Svelte stores と runes の混在) や IPC ラッパーの中途半端さが負債化する

**Why Now:** プロトタイプで美学とアニメが合意済み、要件定義書 v0.3 で全要件確定、バックエンド spec と並列で着手できるタイミング
**Expected Impact:** Phase 0〜8 完走で MVP のフロント部分が完成し、バックエンドと結合すれば即時動作する

---

## Functional Requirements

### User Story

**As a** 集中時間を視覚と聴覚で感じたい knowledge worker
**I want** ししおどしのアニメと数字で時の流れを知れる静かなウィンドウ + メニューバー
**So that** 通知に邪魔されずポモドーロサイクルを回せて、必要なときだけ画面を見ればよい

### Requirements (EARS)

#### F1: タイマー操作 (IPC コマンド送信)

**WHEN** ユーザーが Start ボタンを押すか Space キーを押す
**THE SYSTEM SHALL** `invoke('timer_start')` を実行し、戻り値の `TimerState` で runes ストアを更新する

**WHEN** ユーザーが Pause ボタンを押す
**THE SYSTEM SHALL** `invoke('timer_pause')` を実行し、戻り値で表示状態を更新する

**WHEN** ユーザーが Reset ボタンを押す
**THE SYSTEM SHALL** `invoke('timer_reset')` を実行し、ストアを初期状態に戻す

**WHEN** ユーザーが Skip ボタンを押す
**THE SYSTEM SHALL** `invoke('timer_skip')` を実行し、戻り値の `TimerState` で次フェーズに遷移する

#### F2: IPC イベント受信

**WHEN** バックエンドから `timer:tick` イベントを受信する
**THE SYSTEM SHALL** `remaining_seconds` を runes ストアに反映し、UI を再描画する

**WHEN** バックエンドから `timer:state_changed` イベントを受信する
**THE SYSTEM SHALL** `phase` と `session_count` をストアに反映する

**WHEN** バックエンドから `timer:completed` イベントを受信する
**THE SYSTEM SHALL** ししおどしのカコンアニメーションシーケンスを再生する (4-7 のフェーズ遷移)

#### F3: ししおどし SVG アニメーション

**WHEN** アプリが待機状態 (Idle / Paused)
**THE SYSTEM SHALL** 竹筒を初期傾き `-12°` (水入り口上向き) で静止表示する

**WHILE** タイマーが Work フェーズで走っている
**THE SYSTEM SHALL** 進行率 0→1 に応じて竹筒の傾きを `-12° → +12°` で線形補間する (アニメは 1.2 秒イーズイン・アウト)

**WHEN** Work セッション完了の `timer:completed` を受信する
**THE SYSTEM SHALL** 以下のカコンシーケンスを順に再生する:
  1. 鳴る前の静寂 (400ms)
  2. Tipping: `+12° → +48°` (420ms easeInQuad)
  3. 排水の静止 (180ms)
  4. 反動: `+48° → -25°` (310ms easeOutCubic)
  5. 落ち着き: `-25° → -12°` (900ms easeInOutCubic)
  6. 余韻 (600ms)

**WHEN** ししおどしの竹筒を回転させる
**THE SYSTEM SHALL** SVG `transform` 属性に `rotate(deg cx cy)` を `setAttribute` で書き、CSS `transform` は使わない (軸ズレ回避)

**WHILE** WebView が表示中で Work フェーズ
**THE SYSTEM SHALL** 水滴を `stroke-dashoffset` ループ (0.55s linear) で流す

#### F4: 設定画面

**WHEN** ユーザーがメニューバーから「設定」を選ぶ
**THE SYSTEM SHALL** `/settings` ルートを別ウィンドウまたはモーダルで表示する

**WHEN** ユーザーが設定画面で音量を変更する
**THE SYSTEM SHALL** 即時 `invoke('settings_set', { ... })` を呼び、バック側に反映する (デバウンス 200ms)

**WHEN** ユーザーが音モード (silent / kakon_only / full) を切り替える
**THE SYSTEM SHALL** 即時バックに反映し、現在再生中の音にも適用する

**WHEN** ユーザーがタイマー長 (作業/休憩) を変更する
**THE SYSTEM SHALL** 即時保存するが、現在走っているセッションには適用しない (次セッションから反映、UI で明示)

#### F5: 履歴画面 (v1.0 スコープ確認)

**WHEN** ユーザーが履歴画面に遷移する
**THE SYSTEM SHALL** 当日と直近 7 日のセッション件数を一覧表示する

注: 親仕様 v0.3 では履歴可視化 (グラフ) は v1.1 だが、テキスト一覧 (件数のみ) は MVP に含める。

#### F6: テーマ追従

**WHEN** macOS のシステムテーマが切り替わる (作業中も含む)
**THE SYSTEM SHALL** CSS variables `--ji` / `--sumi` を `matchMedia('(prefers-color-scheme: dark)')` の change イベントで即時切り替える

**WHEN** ユーザーが設定で `theme: 'light' | 'dark' | 'system'` を選択する
**THE SYSTEM SHALL** 選択に従って即時上書きする (`system` のときだけ OS 追従)

#### F7: メインウィンドウの振る舞い

**WHEN** ウィンドウの閉じるボタン (赤丸) が押される
**THE SYSTEM SHALL** ウィンドウを hide するだけでプロセスは終了させない (Tauri 側設定と協調)

**WHEN** 新セッションが開始される
**THE SYSTEM SHALL** 設定 `auto_show_window_on_start` が true ならウィンドウを前面化する

### Edge Cases

#### E1: WebView 表示中にバックでアニメ完了が発生

**WHEN** WebView が非表示中に `timer:completed` を受信する
**THE SYSTEM SHALL** イベントをキューに保持し、再表示時にアニメをスキップして最終状態 (Idle / 次フェーズ) で再開する (過去のカコンを今鳴らさない)

#### E2: IPC 通信エラー

**IF** invoke がエラーを返す
**THE SYSTEM SHALL** トースト等の派手な UI を出さず、コンソールに警告ログを出し、UI 状態は維持する (静寂の哲学を守る)

#### E3: イベントリスナーの cleanup

**WHEN** コンポーネントが unmount される
**THE SYSTEM SHALL** `unlisten()` を必ず呼び出してリスナーリークを防ぐ

---

## Non-Functional Requirements

### Performance

- アニメ中 (カコン演出) の CPU 使用率 5% 以下
- 60fps を維持 (アニメ中の長フレーム < 16.7ms)
- 初回ペイント (FCP) 1.5 秒以内
- メインウィンドウバンドルサイズ 200KB 以下 (gzipped, code-split)

### Accessibility

- VoiceOver 対応 (タイマー残り時間と現フェーズを読み上げ可能、`aria-live="polite"`)
- キーボードのみで全操作可能 (Tab / Space / R / S)
- フォーカスリングは墨色 1.5px、装飾的なシャドウは禁止

### Security

- IPC で受け取った値は型ガードで検証してからストアに入れる (Rust 側型変更に対する fail-fast)
- `dangerouslySetInnerHTML` 相当の生 HTML 注入禁止
- 外部 URL を開く操作なし (本 MVP のフロント単体では)

---

## Success Metrics

### Technical

- [ ] EARS 全項目が手動操作で確認できる
- [ ] Vitest ユニットテストカバレッジ 70% 以上 (ipc ラッパー、ストア、ユーティリティ)
- [ ] Playwright E2E でフルセッションフロー (作業→完了アニメ→休憩→次作業) が走る
- [ ] `npm run build` 成功、`adapter-static` のフォールバック動作確認

### Business / Craft

- [ ] プロトタイプの引き算美学を維持 (色2 / フォント1 / 線種1)
- [ ] アニメ品質が prototype.html と同等以上に保たれる
- [ ] バックエンドと結合した状態でセッション完走

---

## Dependencies

- 親仕様 `docs/requirements.md` v0.3 が確定済み
- バックエンド spec `docs/spec/backend/` と IPC インタフェース (コマンド名・イベント名・型) が一致していること
- Node 20+、Tauri CLI v2、`@tauri-apps/api` v2、SvelteKit 2.x、Svelte 5 (runes)
- バックエンドが `timer:tick` / `timer:state_changed` / `timer:completed` を emit する実装が完了していること (結合フェーズ)

---

## Out of Scope

- 履歴の可視化グラフ (v1.1+)
- 複数プロファイル
- ショートカットキーのグローバルバインド (アプリ非フォーカス時のホットキー)
- 多言語化 (日本語のみ)
- カスタムテーマ・色設定
- ウィジェット
- iCloud 同期
