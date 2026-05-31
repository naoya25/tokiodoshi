# トキオドシ / バックエンド (Rust) — Requirements

**優先度:** Critical
**対象ユーザー:** フロントエンド (SvelteKit) + 直接の API 利用者 (将来の CLI ツールなど)
**対象モジュール:** Rust 側全体 (`src-tauri/`)
**親仕様:** `docs/requirements.md` (v0.3)

---

## Problem

### 現在起きていること

`src-tauri/` ディレクトリ自体が未実装。要件定義書 v0.3 に書かれているタイマー本体・音再生・永続化・トレイなどの責務を満たす Rust コードが必要:

- タイマーステートマシン (Idle / Work / ShortBreak / LongBreak / Paused)
- tokio ベースの ticker (終端時刻基準で精度確保)
- rodio による音再生 (silent / kakon_only / full の 3 モード)
- `tauri-plugin-store` での設定永続化
- `tauri-plugin-sql` (SQLite) での履歴永続化
- TrayIconBuilder でのメニューバー常駐
- macOS スリープ復帰時の整合性
- クラッシュリカバリ (`active_session` の保存・復元)

### なぜ問題なのか

- フロントだけでは macOS のシステム機能 (メニューバー・rodio・SQLite・ログイン時起動) に触れない
- WebView の background throttling 下では JS のタイマーは精度が保てない (R-3 で確認済み)
- 音をフロントの Web Audio で鳴らすとウィンドウ閉時に止まる (R-1)
- これらは要件定義書 v0.3 で全て解消されているが、コードとしては未着手

**Why Now:** フロントエンド spec と並走できる、IPC インタフェースが要件定義書で凍結済み
**Expected Impact:** Phase 0〜9 完走で MVP のバックエンド部分が完成し、フロントと結合すれば即時動作する

---

## Functional Requirements

### User Story

**As a** トキオドシのコアロジックを担うバックエンド
**I want** WebView の状態に依存しない正確なタイマーと、永続化・音・トレイの責務を担う
**So that** フロントは「描画とユーザー入力」だけに集中でき、ウィンドウを閉じても動き続けるアプリになる

### Requirements (EARS)

#### F1: タイマーコマンド (フロントから invoke)

**WHEN** フロントが `timer_start` を invoke する
**THE SYSTEM SHALL** `TimerMachine` を Work フェーズに遷移させ、ticker を起動し、現在の `TimerState` を返す

**WHEN** フロントが `timer_pause` を invoke する
**THE SYSTEM SHALL** ticker を停止し、`Paused` フェーズに遷移し、`TimerState` を返す

**WHEN** フロントが `timer_reset` を invoke する
**THE SYSTEM SHALL** すべての状態を初期化 (`Idle` / `sessionCount=0`) し、ticker を停止する

**WHEN** フロントが `timer_skip` を invoke する
**THE SYSTEM SHALL** 現フェーズを即完了扱いし、次フェーズへ遷移する (work→break / break→work)

**WHEN** フロントが `timer_get_state` を invoke する
**THE SYSTEM SHALL** 現在の `TimerState` を同期的に返す

#### F2: タイマーイベント発火

**WHILE** タイマーが走っている
**THE SYSTEM SHALL** 1 秒ごとに `timer:tick { remaining_seconds }` イベントを emit する

**WHEN** タイマーフェーズが遷移する
**THE SYSTEM SHALL** `timer:state_changed { phase, session_count }` イベントを emit する

**WHEN** セッションが完了する (remaining → 0)
**THE SYSTEM SHALL** `timer:completed { type }` イベントを emit し、SessionRecord を SQLite に保存する

#### F3: 終端時刻基準の ticker (R-3)

**WHEN** ticker がスタートする
**THE SYSTEM SHALL** `end_at = now() + duration_seconds` を計算し、tick ごとに `(end_at - now()).clamp(0, ...).as_secs()` で残り秒を求める

**WHEN** macOS がスリープから復帰する (Instant::now の進行が止まっていた場合)
**THE SYSTEM SHALL** `SystemTime::now()` 基準で残り時間を再計算し、残り < 0 なら即 `completed` を発火する

#### F4: 設定の永続化

**WHEN** フロントが `settings_get` を invoke する
**THE SYSTEM SHALL** `tauri-plugin-store` から `Settings` 構造体を読み、なければデフォルト値を返す

**WHEN** フロントが `settings_set` を invoke する
**THE SYSTEM SHALL** 受け取った `Settings` を store に保存し、`AudioService` と `TrayService` に反映する

**IF** 設定ファイルが破損していて読めない
**THE SYSTEM SHALL** デフォルト設定にフォールバックし、デフォルトを書き戻し、ログ出力する

#### F5: 履歴の永続化

**WHEN** セッションが完了する
**THE SYSTEM SHALL** SQLite `sessions` テーブルに `(type, started_at, completed_at, was_completed, planned_duration_seconds)` を INSERT する

**WHEN** フロントが `history_list(from, to)` を invoke する
**THE SYSTEM SHALL** 該当範囲のセッションを `started_at ASC` で返す

**WHEN** アプリ初回起動 or マイグレーション必要
**THE SYSTEM SHALL** `Migration` struct で `CREATE TABLE IF NOT EXISTS` を実行する

#### F6: 音再生 (rodio, 3 モード)

**WHEN** 設定の `audio.mode` が `full` で Work フェーズが開始する
**THE SYSTEM SHALL** `water-loop.mp3` をループ再生し、0.5 秒フェードインする

**WHEN** Work フェーズが完了する
**THE SYSTEM SHALL** `audio.mode` が `kakon_only` または `full` なら `kakon.mp3` を 1 回再生する

**WHEN** 設定の `audio.mode` が `silent`
**THE SYSTEM SHALL** 一切音を再生しない

**WHEN** フロントが `audio_set_mode` / `audio_set_volume` を invoke する
**THE SYSTEM SHALL** 即時反映する (再生中の音にも適用、音量はリニア補間 50ms で滑らかに)

**WHEN** Work フェーズが終わって休憩に入る
**THE SYSTEM SHALL** 水音を 0.5 秒フェードアウトして停止する

#### F7: メニューバー (Tray)

**WHEN** アプリが起動する
**THE SYSTEM SHALL** TrayIconBuilder で macOS メニューバーにアイコンと title (ゼロ埋め `MM:SS`) を表示する

**WHEN** タイマーフェーズが変わる
**THE SYSTEM SHALL** Tray アイコンを 3 種 (Idle / Work / Break) から切り替える

**WHEN** ユーザーが Tray メニューから操作する
**THE SYSTEM SHALL** 「開始 / 一時停止 / リセット / スキップ / ウィンドウを開く / 設定 / 終了」を提供する

#### F8: クラッシュリカバリ (M-3)

**WHEN** Work セッションが開始する
**THE SYSTEM SHALL** `active_session` 構造体 (`type`, `started_at`, `planned_duration_seconds`, `end_at`) を store に保存する

**WHEN** セッションが正常完了 or リセットされる
**THE SYSTEM SHALL** `active_session` を store から削除する

**WHEN** アプリ起動時に `active_session` が残っている
**THE SYSTEM SHALL** `end_at` を現在時刻と比較し:
  - `now() >= end_at` なら `was_completed=1` で履歴に確定
  - `now() < end_at` なら `was_completed=0` (クラッシュ扱い) で履歴に確定
  - いずれの場合も `active_session` を削除

### Edge Cases

#### E1: ticker と完了タイミングの競合

**WHEN** tick で `remaining_seconds == 0` を検知する
**THE SYSTEM SHALL** ticker を停止してから `timer:completed` を emit する順序を保証する (二重 emit 防止)

#### E2: 音ファイルの読み込み失敗

**IF** 音アセットファイル (water-loop / kakon) が読めない
**THE SYSTEM SHALL** ログ出力して silent モードに自動切替し、その後の動作は通常通り続行する

#### E3: 設定の `durations` 変更中にセッション実行中

**WHEN** 走行中に `settings_set` で `durations` が変わる
**THE SYSTEM SHALL** 現セッションは変更前の duration で完走させ、次セッションから新値を適用する (M-4 反映タイミング表に従う)

#### E4: SQLite 書き込み失敗

**IF** sessions INSERT が失敗する
**THE SYSTEM SHALL** ログ出力するが、ユーザー操作のフローはブロックしない (タイマーは続行)

---

## Non-Functional Requirements

### Performance

- タイマー精度: 1 秒あたり ±50ms 以内
- アイドル時 CPU 使用率 1% 以下
- アイドル時メモリ 80MB 以下
- 起動時間 1.5 秒以内 (コールドスタート)

### Reliability

- パニックは `lib.rs::run()` の panic_hook で捕捉し、ログ出力 + クラッシュリカバリで再開可能に
- `unwrap()` / `expect()` は起動時のクリティカルパスのみ許容、それ以外は `Result` + `thiserror` で扱う
- 設定ファイル破損時の自動回復

### Security

- IPC コマンドの引数は `serde::Deserialize` で型検証
- パスは Tauri の `app.path()` 経由でのみ取得 (ハードコード禁止)
- 必要最小限の capability のみ許可

### Testability

- `core/timer_machine.rs` は Tauri 非依存で `cargo test` 可能
- 単体テストカバレッジ 70% 以上 (特に core/)
- 統合テスト: commands → core → models のレイヤー横断

---

## Success Metrics

### Technical

- [ ] EARS 全項目が手動操作で確認できる
- [ ] `cargo test` で 30+ ケース pass、カバレッジ 70%+
- [ ] timer_machine の 7 ケース状態遷移テストが全 pass
- [ ] スリープ復帰時に残り時間が正しく再計算される (実機テスト)
- [ ] codesign + notarize 通過 (配布前)

### Business / Craft

- [ ] フロントと結合した状態で 1 セッション完走
- [ ] ウィンドウを閉じても水音が継続再生
- [ ] アプリ強制終了後の再起動で active_session が正しく確定

---

## Dependencies

- 親仕様 `docs/requirements.md` v0.3 確定済み
- フロントエンド spec の IPC インタフェース (コマンド名・イベント名・型) と一致
- Rust 1.77+、Tauri CLI v2、tokio 1.x、rodio 0.20+、`tauri-plugin-store` v2、`tauri-plugin-sql` v2、`thiserror` 1
- macOS 12.0+ 実機 (Apple Silicon または Intel)
- Apple Developer Program ($99/year) — 配布フェーズで必要

---

## Out of Scope

- Windows / Linux 対応
- ネットワーク機能 (アップデータ・テレメトリ・同期)
- macOS Focus Mode 連携 (v2.0+)
- 通知 (要件定義書で除外)
- 履歴の高度な集計 (グラフ・タグ分類)
- 複数プロファイル
- iOS / Android 対応
