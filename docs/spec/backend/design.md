# トキオドシ / バックエンド (Rust) — Design

## Technical Approach

`src-tauri/src/` を以下の責務で分割。`lib.rs` は集約のみ、ロジックは `core/` (Tauri 非依存) と `commands/` (薄い IPC 層) に分かれる。

```
┌────────────────────────────────────────────────┐
│  lib.rs                                         │
│   - Builder + invoke_handler! 集約              │
│   - plugin 登録 (store, sql)                    │
│   - .manage(AppState)                           │
│   - setup() で tray + ticker + audio init       │
├────────────────────────────────────────────────┤
│  commands/  (薄い IPC 層、引数を core に渡すだけ) │
│   timer.rs / settings.rs / history.rs / audio.rs │
├────────────────────────────────────────────────┤
│  core/  (Tauri 非依存、cargo test 可能)          │
│   timer_machine.rs — ステートマシン本体          │
│   ticker.rs        — tokio interval + 終端時刻基準│
│   persistence.rs   — store + sql の薄い wrapper │
│   audio_service.rs — rodio 抽象                  │
├────────────────────────────────────────────────┤
│  models/  (serde 型定義、フロントに渡る型)         │
│   timer_state.rs / settings.rs / session.rs      │
├────────────────────────────────────────────────┤
│  state.rs   AppState { Mutex<TimerMachine>, ... }│
│  error.rs   thiserror で AppError                │
│  tray.rs    TrayIconBuilder + メニュー           │
└────────────────────────────────────────────────┘
```

**重要原則:**
1. `core/timer_machine.rs` は `tauri::*` を import しない (純 Rust、ユニットテスト容易)
2. `lib.rs::run()` に `#[cfg_attr(mobile, tauri::mobile_entry_point)]` 付与 (規約準拠)
3. `Mutex` は std (tokio::sync ではない) — 軽量、blocking なし、tauri::State<'_, Mutex<T>> がベストプラクティス
4. エラーは `Result<T, AppError>`、`AppError` は `Serialize` を実装してフロントに渡す
5. ticker は tokio タスク 1 つだけ、`Arc<AppHandle>` で emit 用

---

## 確認できたこと

- `my-tauri-v2` スキルで `generate_handler![]` 登録漏れが代表エラー No.1 と確認
- `tauri-plugin-sql` の permissions は `sql:allow-execute` と `sql:allow-select` が分かれている
- async コマンドの引数は所有型 (`String`) のみ。借用 (`&str`) はエラー
- `lib.rs` 集約パターンは tauri v2 のモバイル規約に必須 (デスクトップ専用でも準拠)
- `tauri-plugin-store` は JSON 1 ファイルの key-value、シンプル
- rodio は cross-platform、macOS では coreaudio バックエンド

---

## 推測していること

- `tokio::time::interval` よりも、`Instant + sleep_until(end_at)` の方が macOS スリープ復帰後の整合性が良い
- `Mutex<TimerMachine>` は `.lock().unwrap()` で問題ない (poison 時はパニックで OK、データ整合性壊れたら復帰せず終了する)
- `tauri-plugin-store` への書き込みは fsync まで含めて 5-10ms 程度 (公式の数値未確認)
- rodio の `Sink::set_volume(linear)` は即時反映だが、フェードは自前実装が必要

---

## 未確認事項

- macOS スリープ中の `tokio::time::sleep` の挙動 (Instant 基準だと止まる、SystemTime 基準でリトライ必要)
- `tauri-plugin-sql` の SQLite ファイルパスのデフォルト位置 (`~/Library/Application Support/<bundle_id>/`)
- TrayIconBuilder で title にゼロ埋め `00:00` を表示する際の文字幅 (要件 F-02 で proportional フォント想定)
- `rodio` の MP3 デコーダーの起動レイテンシ (kakon を即時鳴らせるか、preload 必要か)

---

## Key Decisions

| 論点 | 選択 | 理由 |
|---|---|---|
| 状態管理 | `Mutex<TimerMachine>` (std) | std で十分軽量、Tauri State<'_, Mutex<T>> ベストプラクティス |
| ticker 駆動 | tokio タスク + `tokio::time::sleep_until(end_at)` | 終端時刻基準でスリープ復帰に強い |
| 残り時間計算 | `SystemTime::now()` 基準 | スリープ復帰時に Instant ではズレる |
| 音再生 | rodio 0.20+ | macOS coreaudio バックエンド、シンプル |
| 設定永続化 | tauri-plugin-store (JSON) | 設定は小さい、SQL は重い |
| 履歴永続化 | tauri-plugin-sql (SQLite) | 集計クエリが効く、将来の拡張に強い |
| Migration | Rust 側 `Migration` struct で `CREATE TABLE IF NOT EXISTS` | フロントから ad-hoc DDL を禁止 (R-5) |
| エラー型 | thiserror + serde::Serialize 自作 impl | フロントへ string で渡る |
| Tray | Tauri v2 組込 TrayIconBuilder | プラグイン不要 |
| 共有型同期 | TS 側手書き (ts-rs 不採用) | 型は 4-5 個のみ、ts-rs はオーバー |
| パニック処理 | std::panic::set_hook で log + クラッシュリカバリ | unwrap 起因の異常終了でも次回起動時に復元 |

---

## TimerMachine 設計

### 状態遷移図

```
            ┌──────────┐  start  ┌──────────┐
            │   Idle   │ ──────→ │   Work   │
            └──────────┘         └────┬─────┘
                  ↑                    │ remaining→0
                  │                    ↓
        reset 後  │              ┌────────────┐
                  │              │  Completing │  → emit completed
                  │              │  (一瞬)     │  → save SessionRecord
                  │              └────┬───────┘
                  │                    │
                  │  session_count<4   │   session_count==4
                  │      ↓              ↓
                  │  ┌──────────┐   ┌──────────┐
                  │  │ShortBreak│   │LongBreak │
                  │  └────┬─────┘   └────┬─────┘
                  │       │ remaining→0   │
                  │       └────────┬──────┘
                  └────────────────┤
                                   ↓
                              次の Work へ

  Paused: 任意の Work/Break から ⇄ pause/resume で双方向
```

### 公開 API (Tauri 非依存)

```rust
pub struct TimerMachine {
    state: TimerState,
    config: TimerConfig,        // durations 4個
    end_at: Option<SystemTime>,  // ticker 用
    session_count: u32,
}

pub enum TimerEvent {
    Tick(u32),                                 // remaining_seconds
    StateChanged { phase: Phase, count: u32 },
    Completed { kind: SessionKind },
}

impl TimerMachine {
    pub fn new(config: TimerConfig) -> Self;
    pub fn start(&mut self) -> TimerEvent;
    pub fn pause(&mut self) -> TimerEvent;
    pub fn reset(&mut self) -> TimerEvent;
    pub fn skip(&mut self) -> Vec<TimerEvent>;
    pub fn poll(&mut self, now: SystemTime) -> Vec<TimerEvent>;
    pub fn state(&self) -> &TimerState;
    pub fn set_config(&mut self, c: TimerConfig);   // 次セッションから有効
}
```

`poll()` は ticker から定期的に呼ばれ、残り時間を再計算 + 完了検知。

---

## Ticker 設計 (core/ticker.rs)

```rust
pub fn spawn(app: AppHandle) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(250));  // 250ms 刻みで poll
        loop {
            interval.tick().await;
            let now = SystemTime::now();
            let events: Vec<TimerEvent> = {
                let state = app.state::<Mutex<TimerMachine>>();
                let mut machine = state.lock().unwrap();
                machine.poll(now)
            };
            for ev in events {
                emit_event(&app, ev).await;
            }
        }
    });
}
```

**250ms 刻みの意図:**
- 1 秒タイマー精度 ±50ms には十分
- macOS スリープ復帰直後でも次の poll で必ず捕捉
- CPU 使用率は無視できるレベル

---

## AudioService 設計

```rust
pub struct AudioService {
    mode: AudioMode,
    master_volume: f32,
    water_sink: Option<Sink>,    // ループ再生用
    kakon_sink: Option<Sink>,    // 一発再生用
    _stream: OutputStream,        // ライフタイム保持
    handle: OutputStreamHandle,
}

impl AudioService {
    pub fn new() -> Result<Self, AudioError>;
    pub fn set_mode(&mut self, mode: AudioMode);
    pub fn set_volume(&mut self, kind: VolumeKind, v: f32);
    pub fn start_water(&mut self);   // mode=full のとき水音ループ開始 (0.5s fade-in)
    pub fn stop_water(&mut self);    // 0.5s fade-out
    pub fn play_kakon(&mut self);    // mode in [kakon_only, full] のとき再生
}
```

**フェードの実装:**
- `Sink::set_volume()` を 50ms 毎に 10 ステップで線形補間 (専用タスクで)
- mode 切替時にも適用

---

## Persistence 設計 (core/persistence.rs)

### 設定 (tauri-plugin-store)

```rust
const STORE_PATH: &str = "settings.json";

pub async fn load_settings(app: &AppHandle) -> Settings {
    let store = app.store(STORE_PATH)?;
    store.get("settings")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_else(|| {
            log::warn!("settings.json 破損 or 不在、デフォルトを使用");
            Settings::default()
        })
}

pub async fn save_settings(app: &AppHandle, s: &Settings) -> Result<(), AppError>;
pub async fn save_active_session(app: &AppHandle, s: &ActiveSession) -> Result<(), AppError>;
pub async fn clear_active_session(app: &AppHandle) -> Result<(), AppError>;
pub async fn load_active_session(app: &AppHandle) -> Option<ActiveSession>;
```

### 履歴 (tauri-plugin-sql)

```rust
const DB_URL: &str = "sqlite:sessions.db";

pub fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "create sessions table",
            sql: r#"
                CREATE TABLE IF NOT EXISTS sessions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    type TEXT NOT NULL CHECK(type IN ('work','short_break','long_break')),
                    started_at TEXT NOT NULL,
                    completed_at TEXT,
                    was_completed INTEGER NOT NULL CHECK(was_completed IN (0,1)),
                    planned_duration_seconds INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);
            "#,
            kind: MigrationKind::Up,
        },
    ]
}

pub async fn record_session(db: Db, r: &SessionRecord) -> Result<i64, AppError>;
pub async fn list_sessions(db: Db, from: &str, to: &str) -> Result<Vec<SessionRecord>, AppError>;
```

---

## Tray 設計 (tray.rs)

```rust
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text("start", "開始")
        .text("pause", "一時停止")
        .text("reset", "リセット")
        .text("skip", "スキップ")
        .separator()
        .text("open", "ウィンドウを開く")
        .text("settings", "設定")
        .separator()
        .text("quit", "終了")
        .build()?;

    let tray = TrayIconBuilder::new()
        .icon(icon_idle())   // 起動時は Idle アイコン
        .title("00:00")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "start" => { /* invoke timer_start equivalent */ }
            // ...
        })
        .build(app)?;

    Ok(())
}

pub fn update_title(tray: &TrayIcon, remaining: u32) {
    let m = remaining / 60;
    let s = remaining % 60;
    tray.set_title(Some(format!("{:02}:{:02}", m, s))).ok();
}

pub fn update_icon(tray: &TrayIcon, phase: Phase) { /* 3 種 template image を切替 */ }
```

---

## Data Flow

### timer_start の流れ

```
Frontend invoke('timer_start')
   ↓
commands/timer.rs::start(state: State<Mutex<TimerMachine>>)
   ↓
machine.start() → TimerEvent::StateChanged { Work, count }
   ↓
emit_event(app, StateChanged) → emit `timer:state_changed`
   ↓
active_session を store に保存
   ↓
AudioService::start_water() (mode=full のとき)
   ↓
tray::update_icon(Work)
   ↓
return TimerState (フロントに)
```

### 完了の流れ (ticker から発火)

```
ticker tick (250ms 毎)
   ↓
machine.poll(now) → [Tick(0), Completed { kind: Work }]
   ↓
emit `timer:tick` { 0 }
   ↓
emit `timer:completed` { type: 'work' }
   ↓
persistence::record_session() → SQLite INSERT
   ↓
clear_active_session()
   ↓
machine が ShortBreak へ自動遷移 (poll の中で)
   ↓
emit `timer:state_changed` { ShortBreak, count: 1 }
   ↓
AudioService::stop_water() + play_kakon()
   ↓
tray::update_icon(Break)
```

---

## Trade-offs

**優先すること:**
- タイマーの正確性 (スリープ復帰でズレない)
- フロントが落ちてもバックは生き続ける
- パニック耐性

**受け入れる制約:**
- core/timer_machine は Tauri 非依存を維持するため、emit は commands 層で行う (テスタビリティとのトレードオフ)
- 共有型の手書き同期 (Rust 変更時に TS 側を手で直す)
- 単体テストでは rodio の音は再生しない (モック)

---

## Risks & Mitigations

| リスク | 対策 |
|---|---|
| スリープ復帰で残り時間がズレる (R-3) | `SystemTime::now()` 基準 + 250ms poll で必ず再計算 |
| ウィンドウ閉時に音が止まる (R-1) | rodio を Rust 側で持つ、WebView と独立 |
| ticker 起動の二重化 | spawn は setup() で 1 回のみ、Mutex で再起動防止 |
| panic で履歴が消える | active_session を都度 store に保存、起動時復元 (F8) |
| audio ファイル不在で起動失敗 | E2: silent モードに自動 fallback |
| SQLite ファイル破損 | 起動時に migration 再実行で table 再作成 (データは失われるが起動はする) |
| Mutex poison (panic 後) | poison_recovery 不採用、unwrap で fast fail (active_session で次起動時に復元) |
| Tray title の文字幅問題 (R-4) | ゼロ埋め固定 `00:00`、フォント指定不可前提 |
| capability 不足の silent failure | 開発時に v2.tauri.app の各 plugin permission を確認、CI で `tauri info` 確認 |
