# トキオドシ / バックエンド (Rust) — Tasks

凡例: `[P]` = 並列実行可、`(Xh)` = 工数見積もり (人間時間)、`DoD` = Definition of Done。

---

## Phase 0: セットアップ (フロント Phase 0 と並列可)

- [ ] **B0.1** `src-tauri/` 雛形整備 (0.5h)
  - フロント Phase 0 (`npm create tauri-app`) で生成された `src-tauri/` を確認
  - `Cargo.toml` の `[lib]` セクションが `name = "tokiodoshi_lib"`, `crate-type = ["staticlib","cdylib","rlib"]` であること
  - `main.rs` がパススルーのみで `tokiodoshi_lib::run()` を呼ぶ形になっていること
  - DoD: `cargo build` 成功 + 空の Tauri ウィンドウが起動
  - Files: `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`

- [ ] **B0.2** [P] プラグイン依存追加 (0.3h)
  - `tauri-plugin-store = "2"`
  - `tauri-plugin-sql = { version = "2", features = ["sqlite"] }`
  - `rodio = "0.20"`
  - `tokio = { version = "1", features = ["full"] }`
  - `thiserror = "1"`
  - `serde = { version = "1", features = ["derive"] }`
  - `chrono = { version = "0.4", features = ["serde"] }`
  - `log = "0.4"` + `env_logger = "0.11"`
  - DoD: `cargo build` 成功

- [ ] **B0.3** [P] capability 設定 (0.3h)
  - `src-tauri/capabilities/default.json` に必要な permissions:
    - `core:default`
    - `core:window:allow-close` / `allow-hide` / `allow-show` / `allow-set-focus`
    - `store:default`
    - `sql:default` / `sql:allow-execute` / `sql:allow-select`
  - DoD: `cargo tauri dev` で permission denied が起きない

- [ ] **B0.4** [P] ディレクトリ構造作成 (0.2h)
  - `src-tauri/src/{commands,core,models}/mod.rs` 空ファイル
  - `state.rs` / `error.rs` / `tray.rs` 空ファイル
  - DoD: `cargo build` 成功 (空でも mod 宣言が通る)

---

## Phase 1: Models (Phase 0 完了後)

- [ ] **B1.1** [P] `models/timer_state.rs` (1h)
  - `Phase` enum (`Idle`, `Work`, `ShortBreak`, `LongBreak`, `Paused`) + `Serialize`/`Deserialize`
  - `TimerState` 構造体 (phase, remaining_seconds, session_count, current_duration_seconds)
  - `SessionKind` enum (Work, ShortBreak, LongBreak)
  - `TimerConfig` (4 durations)
  - DoD: ユニットテスト 2 ケース (serialize / default)

- [ ] **B1.2** [P] `models/settings.rs` (1h)
  - `Settings` 構造体 (durations, audio, behavior, appearance)
  - `AudioMode` enum (`silent`, `kakon_only`, `full`) + `#[serde(rename_all = "snake_case")]`
  - `Theme` enum
  - `Default` impl で要件定義書 6.1 のデフォルト値
  - DoD: ユニットテスト 3 ケース (serialize / default / 部分更新)

- [ ] **B1.3** [P] `models/session.rs` (0.5h)
  - `SessionRecord` 構造体 (id, type, started_at, completed_at, was_completed, planned_duration_seconds)
  - `ActiveSession` 構造体 (type, started_at, planned_duration_seconds, end_at)
  - DoD: serialize テスト 1 ケース

- [ ] **B1.4** `error.rs` (0.5h)
  - `AppError` enum with `thiserror`
    - `#[error("Io: {0}")] Io(#[from] std::io::Error)`
    - `#[error("Store: {0}")] Store(String)`
    - `#[error("Sql: {0}")] Sql(String)`
    - `#[error("Audio: {0}")] Audio(String)`
    - `#[error("Not found: {0}")] NotFound(String)`
  - `serde::Serialize` の手動 impl (string にする)
  - DoD: `cargo test` で 1 ケース pass

---

## Phase 2: Core / TimerMachine (Phase 1 完了後、最重要)

- [ ] **B2.1** `core/timer_machine.rs` 純Rustステートマシン (3h)
  - `TimerMachine` struct + `new()` / `start()` / `pause()` / `reset()` / `skip()` / `poll(now)` / `state()` / `set_config()`
  - `Tauri::*` を一切 import しない
  - DoD: 下記 7 ケースのユニットテスト全 pass

  **ユニットテストケース (M-2):**
  1. 初期状態は `Idle`、`session_count=0`
  2. start() で `Work` に遷移、`end_at` 設定、StateChanged event 返却
  3. poll() で時間経過に応じて `Tick` event 返却、`remaining_seconds` が減る
  4. Work 完了で `Completed` + `StateChanged(ShortBreak)` を順に返却、`session_count+1`
  5. 4 セッション後の Work 完了で `LongBreak` に遷移
  6. pause() → resume() で残り時間が保持される
  7. skip() で現フェーズが即完了扱い、次フェーズへ
  8. reset() で全状態が初期化される
  9. set_config() は次セッションから適用、現セッションは変更前 duration で完走 (E3)
  10. スリープ復帰相当: 大きな時間ジャンプの poll() で `Tick(0)` + `Completed` を1回だけ発火

  - File: `src-tauri/src/core/timer_machine.rs`

- [ ] **B2.2** [P] `core/ticker.rs` (1.5h)
  - `pub fn spawn(app: AppHandle)` で tokio タスク起動
  - 250ms 毎に `machine.poll(SystemTime::now())` を呼ぶ
  - 返ってきた `Vec<TimerEvent>` を emit + 副作用 (record_session, audio, tray) に振り分け
  - 二重 spawn 防止 (state.ticker_spawned フラグ)
  - DoD: 統合テスト 2 ケース (起動・停止 / 二重 spawn 拒否)
  - File: `src-tauri/src/core/ticker.rs`

- [ ] **B2.3** [P] `core/persistence.rs` (1.5h)
  - `load_settings()` / `save_settings()`
  - `save_active_session()` / `clear_active_session()` / `load_active_session()`
  - SQL migration 定義 (`migrations()` 関数)
  - `record_session()` / `list_sessions()`
  - DoD: ユニットテスト 5 ケース (in-memory SQLite で migration + insert + select)
  - File: `src-tauri/src/core/persistence.rs`

- [ ] **B2.4** [P] `core/audio_service.rs` (2h)
  - rodio 初期化 (OutputStream + Handle 保持)
  - `set_mode()` / `set_volume()` / `start_water()` / `stop_water()` / `play_kakon()`
  - フェードイン・フェードアウト (50ms × 10 ステップで線形補間、別 tokio タスクで)
  - 音アセット読み込み失敗時の silent fallback (E2)
  - 音アセットは `src-tauri/assets/water-loop.mp3` と `kakon.mp3` を include_bytes! で埋め込み (差し替え時に再ビルド)
  - DoD: ユニットテスト 3 ケース (mode 切替 / volume 補間 / fallback)
  - File: `src-tauri/src/core/audio_service.rs`

---

## Phase 3: State (Phase 2 完了後)

- [ ] **B3.1** `state.rs` (0.5h)
  - `pub struct AppState { pub machine: Mutex<TimerMachine>, pub audio: Mutex<AudioService>, pub ticker_spawned: AtomicBool }`
  - `impl AppState { pub fn new(config: TimerConfig) -> Result<Self, AppError> }`
  - DoD: コンパイル成功 + Default Settings から構築テスト 1 ケース

---

## Phase 4: Commands (Phase 3 完了後)

- [ ] **B4.1** [P] `commands/timer.rs` (1.5h)
  - `#[tauri::command] timer_start / timer_pause / timer_reset / timer_skip / timer_get_state`
  - 各コマンドは `State<'_, AppState>` と `AppHandle` を取り、`machine.lock()` で操作
  - 戻り値 `Result<TimerState, AppError>`
  - 関連 emit / persistence 呼び出しは ticker 側に委譲しないでここで完結 (start/pause/reset の即時 IPC)
  - DoD: 統合テスト 5 ケース (各 command が正しい TimerState を返す)
  - File: `src-tauri/src/commands/timer.rs`

- [ ] **B4.2** [P] `commands/settings.rs` (1h)
  - `settings_get` / `settings_set`
  - set 時に AudioService.set_mode / set_volume を呼ぶ
  - durations 変更は machine.set_config() (次セッションから有効、現セッションは継続)
  - DoD: 統合テスト 3 ケース (取得 / 設定 / 部分更新)

- [ ] **B4.3** [P] `commands/history.rs` (0.5h)
  - `history_list(from: String, to: String)`
  - DoD: 統合テスト 2 ケース (空 / データあり)

- [ ] **B4.4** [P] `commands/audio.rs` (0.5h)
  - `audio_set_mode(mode: AudioMode)` / `audio_set_volume(kind: String, value: f32)`
  - settings 側の wrapper だが、即時反映用に独立コマンドも用意
  - DoD: 統合テスト 2 ケース

---

## Phase 5: Tray (Phase 4 完了後)

- [ ] **B5.1** `tray.rs` TrayIconBuilder セットアップ (2h)
  - メニュー定義 (開始 / 一時停止 / リセット / スキップ / ウィンドウを開く / 設定 / 終了)
  - on_menu_event ハンドラ (各 commands を呼ぶ)
  - `update_title(remaining)` — `{:02}:{:02}` ゼロ埋め固定
  - `update_icon(phase)` — Idle / Work / Break の 3 種 template image を切替
  - アイコンは `src-tauri/icons/tray-idle.png` / `tray-work.png` / `tray-break.png` を用意 (16/32/64 px)
  - DoD: 手動確認 (起動 → メニュー操作 → title が時刻更新)
  - File: `src-tauri/src/tray.rs`

---

## Phase 6: lib.rs 集約 (Phase 5 完了後)

- [ ] **B6.1** `lib.rs::run()` の Builder 構築 (1h)
  - plugin 登録: store + sql (migrations 渡す)
  - .manage(AppState::new(load_settings から))
  - .invoke_handler!() に全コマンド (12個程度) 登録
  - .setup() で:
    1. crash recovery (load_active_session → 履歴確定 → clear)
    2. tray::setup()
    3. ticker::spawn()
    4. audio init
  - panic_hook 登録 (log + active_session 強制保存)
  - DoD: 起動して全コマンドが invoke 可能 (フロント未実装でもコマンド一覧テストで確認)

---

## Phase 7: クラッシュリカバリ統合 (Phase 6 完了後)

- [ ] **B7.1** active_session の自動保存・復元 (1.5h)
  - timer_start で save、timer_reset / 完了で clear
  - setup() の最初に load_active_session() → end_at と now() 比較で was_completed 判定 → record_session → clear
  - DoD: 統合テスト 3 ケース (正常完了パス / 強制終了パス / アプリ強制 kill → 再起動)

---

## Phase 8: テスト (Phase 7 完了後)

### 8.A ユニットテスト (cargo test, in-process)

- [ ] **B8.1** [P] core/timer_machine の状態遷移 10 ケース (B2.1 で対応済み、ここで再確認)
  - DoD: 10/10 pass、カバレッジ 95%

- [ ] **B8.2** [P] core/persistence の Migration + CRUD (B2.3 で対応済み、ここで再確認)
  - in-memory SQLite (`:memory:`) で migration 実行
  - INSERT → SELECT で型整合
  - DoD: 5/5 pass、カバレッジ 80%

- [ ] **B8.3** [P] core/audio_service (B2.4 で対応済み)
  - mode 切替、音アセット fallback
  - DoD: 3/3 pass

- [ ] **B8.4** [P] models/* serialize テスト (1h)
  - 各 enum / struct の Serialize / Deserialize 往復
  - DoD: 8/8 pass

### 8.B 統合テスト (commands → core → models)

- [ ] **B8.5** [P] commands/timer の 5 関数 (B4.1 で対応済み、追加確認)
  - 各 command の戻り値 + 副作用 (event 発火) を確認
  - DoD: 5/5 pass

- [ ] **B8.6** [P] commands/settings の即時反映 (1h)
  - settings_set → AudioService.mode が変わる
  - DoD: 3/3 pass

- [ ] **B8.7** [P] フェーズ遷移 + record_session フロー (1.5h)
  - スピードを上げた config (work=1秒 等) で start → 完走 → ShortBreak → 完走 → ... を実行
  - sessions テーブルに正しい行が追加される
  - DoD: 1 ケース pass

### 8.C 実機テスト (手動 + macOS specific)

- [ ] **B8.8** スリープ復帰テスト (1h)
  - 25 分タイマー走行中に macOS をスリープ → 復帰
  - 残り時間が正しく再計算されている
  - DoD: 手動確認 OK

- [ ] **B8.9** [P] ウィンドウ閉時の継続再生 (0.5h)
  - mode=full でセッション開始 → メインウィンドウ閉じる → 水音が継続
  - DoD: 手動確認 OK

- [ ] **B8.10** [P] アプリ強制終了 → 再起動 (1h)
  - セッション走行中に `kill -9` で強制終了
  - 再起動で active_session が確定 (was_completed=0)
  - DoD: 手動確認 OK

- [ ] **B8.11** [P] Tray メニュー全項目動作確認 (0.5h)
  - DoD: 7 項目すべて動作

---

## Phase 9: 仕上げ (Phase 8 完了後)

- [ ] **B9.1** ログイン時自動起動 (1.5h)
  - `tauri-plugin-autostart` 追加 or `launchctl` 直接連携
  - 設定で ON/OFF 切替
  - DoD: 設定 ON でログイン後にアプリ自動起動

- [ ] **B9.2** [P] LSUIElement 切替 (要件 v0.3 「要再起動」) (1h)
  - Info.plist の LSUIElement を設定で書き換え
  - 設定変更時に再起動を促す UI (フロント側) と協調
  - DoD: ON で Dock 非表示

- [ ] **B9.3** [P] アプリアイコン .icns 作成 (1h)
  - 1024px の PNG から `iconutil` で .icns 生成
  - tauri.conf.json で参照
  - DoD: ビルド後の .app に正しいアイコン

- [ ] **B9.4** codesign + notarize 準備 (2h)
  - Apple Developer Program 加入確認
  - Tauri の bundle 設定 + signing identity
  - DoD: `cargo tauri build` で notarize 済み .dmg が生成

- [ ] **B9.5** [P] パフォーマンス計測 (1h)
  - Activity Monitor で CPU/メモリ確認
  - アイドル時 < 1% CPU / 80MB メモリ
  - DoD: 数値が目標内

---

## Verification Checklist

- [ ] requirements の F1-F8 と E1-E4 をすべて満たす
- [ ] cargo test 30+ ケース pass、カバレッジ 70%+
- [ ] フロント spec の IPC インタフェースと整合
- [ ] スリープ復帰テスト pass
- [ ] クラッシュ → 再起動の active_session 復元 pass
- [ ] codesign + notarize 済み .dmg が生成できる
- [ ] panic_hook が active_session を保存する

---

## 工数サマリ

| Phase | 工数 |
|---|---|
| Phase 0 (セットアップ) | 1.3h |
| Phase 1 (models) | 3h |
| Phase 2 (core) | 8h |
| Phase 3 (state) | 0.5h |
| Phase 4 (commands) | 3.5h |
| Phase 5 (tray) | 2h |
| Phase 6 (lib.rs) | 1h |
| Phase 7 (crash recovery) | 1.5h |
| Phase 8 (テスト) | 6h |
| Phase 9 (仕上げ) | 6.5h |
| **合計** | **33.3h ≒ 4.2 人日** |

---

## Progress Log

| Date | Status | Notes |
|---|---|---|
| 2026-06-01 | Draft | Spec 作成完了。実装着手前 |
