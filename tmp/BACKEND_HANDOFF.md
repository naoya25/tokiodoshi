# トキオドシ バックエンド (Rust) 引き継ぎ書

| 項目 | 内容 |
|---|---|
| 作成日 | 2026-06-01 |
| 作成者 | フロント担当 (Naoya) |
| 受領者 | バックエンド担当者 |
| プロジェクト | macOS メニューバー型ポモドーロタイマー「トキオドシ」 |
| リポジトリ | `~/Desktop/Repos/tokiodoshi/` (ローカル、まだ git remote なし) |
| 想定工数 | 約 4.2 人日 (`docs/spec/backend/tasks.md` 参照) |

---

## 0. 結論ファースト

- **読むべきもの: 4 つだけ** — このファイル → `docs/requirements.md` → `docs/spec/backend/` 3 ファイル
- **着手すべきフェーズ**: `docs/spec/backend/tasks.md` の **Phase 0 は完了済み**。次は **Phase 1 (B1.1 から)**
- **IPC 契約は凍結済み**: フロント側が既に `src/lib/types/` と `src/lib/ipc/` で利用中。型・コマンド名は変更不可
- **フロントは動く** (`npm run dev` で UI が出る)。バック側のコマンドは全部スタブで、TimerState::default() を返すだけ
- **コミットはまだ無し**。git init はされている状態。最初の意味あるコミットを Phase 1 完了後に
- **Rust が未インストール**の可能性あり (雛形生成時に警告が出た)。`rustup` で入れてから着手

---

## 1. このプロジェクトは何か

ししおどし (鹿威し) の物理的なリズムをポモドーロタイマーに重ねた macOS メニューバーアプリ。

- 25 分の集中 → 5 分休憩 → 繰り返し → 4 セッション後に 15 分の長休憩
- 完了の合図はメニューバーアイコン変化 + 「カコン」音 (silent / kakon_only / full の 3 モード)
- 通知は一切出さない (Slack/メールに干渉しない)
- macOS 12.0+ 専用 (Apple Silicon / Intel Universal)

**美学**: 引き算の和モダン。色 2 / フォント 1 / 線種 1。プロトタイプ `docs/prototype.html` を見ると一発で雰囲気がわかる (ブラウザで開ける)。

---

## 2. 必読ドキュメント

| ファイル | 用途 | 読む順序 |
|---|---|---|
| `README.md` | 概要・セットアップ手順 | 1 |
| **このファイル** | バックエンド作業の起点 | 2 |
| `docs/requirements.md` (v0.3) | 全体の要件定義書。F-01〜F-06, R-1〜R-5, M-2〜M-8 が網羅されている | 3 |
| `docs/spec/backend/requirements.md` | バック側 EARS 形式要件 (F1-F8, E1-E4) | 4 |
| `docs/spec/backend/design.md` | アーキ構造・状態遷移・データフロー・Risks | 5 |
| `docs/spec/backend/tasks.md` | Phase 0-9 のタスクリスト + DoD | 6 (着手時の主参照) |
| `docs/prototype.html` | ビジュアル・アニメの参考。ブラウザで開く | 任意 |
| `docs/spec/frontend/*` | フロント側 spec (IPC 契約理解のため斜め読み推奨) | 任意 |

---

## 3. 環境セットアップ

### 3.1 前提

```bash
# Node はインストール済み前提 (フロント基盤は npm install 完了済み)
node --version    # 20+
npm --version

# Rust の確認 (未インストールの可能性)
rustc --version
cargo --version
```

Rust 未インストールなら:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Xcode Command Line Tools (macOS):
```bash
xcode-select --install   # 既に入っていれば noop
```

### 3.2 動作確認

リポジトリ直下で:

```bash
cd ~/Desktop/Repos/tokiodoshi

# フロントのテスト (既に通る)
npm test

# Tauri 開発起動 (Rust 初回ビルドで数分かかる)
npm run tauri dev
```

正常起動すると:
- 720x540 のウィンドウが開く
- 中央右下にししおどしの SVG が表示
- 左下に巨大なタイマー数字 `25:00`
- ホバーで「始」「戻」ボタンが薄く出る
- Space キーで「始」を押した状態になる (が、ticks は来ないので画面は更新されない = バックがスタブのため)

### 3.3 開発ループ

```bash
npm run tauri dev      # Rust 変更で自動再ビルド + Svelte HMR
cargo test --manifest-path src-tauri/Cargo.toml  # Rust 単体テスト
npm test               # フロント単体テスト (今は 23 件 pass)
npm run check          # TypeScript 型チェック
```

---

## 4. 現状のディレクトリと、あなたが触る場所

```
tokiodoshi/
├── src/                          ← フロント。基本的に触らない (IPC 契約変更時のみ types/ 同期)
│   └── lib/
│       └── types/index.ts        ← Rust models と同期する型。Rust 側変更時は要更新
├── src-tauri/                    ← ★ ここがあなたの作業領域
│   ├── Cargo.toml                  依存追加済み (rodio, tokio, thiserror, chrono, log etc.)
│   ├── tauri.conf.json             productName, window 設定済み
│   ├── capabilities/default.json   permissions 追加済み (store, sql)
│   └── src/
│       ├── main.rs                 そのままで OK (薄いパススルー)
│       ├── lib.rs                  ★ TODO: setup() の中身 (ticker, tray, recovery)
│       ├── state.rs                ★ TODO: TimerMachine / AudioService を統合
│       ├── error.rs                ✅ AppError 定義済み
│       ├── tray.rs                 ★ TODO: TrayIconBuilder 実装
│       ├── commands/               全 5 関数スタブで動く。中身を core/ に繋ぐ
│       │   ├── timer.rs            ★ TODO: machine.lock().start() 等
│       │   ├── settings.rs         ★ TODO: persistence + audio 反映
│       │   ├── history.rs          ★ TODO: persistence::list_sessions
│       │   └── audio.rs            ★ TODO: AudioService 呼び出し
│       ├── core/                   ロジック層 (Tauri 非依存)
│       │   ├── timer_machine.rs    ★ TODO: 状態マシン本体 (最重要、unit test 10 ケース)
│       │   ├── ticker.rs           ★ TODO: tokio interval + poll
│       │   ├── persistence.rs      ★ TODO: store + sql
│       │   └── audio_service.rs    ★ TODO: rodio
│       └── models/                 ✅ 型定義済み (TimerState, Settings, Session, AudioMode...)
├── docs/                         触らない (要件・設計の参照元)
└── tmp/
    └── BACKEND_HANDOFF.md        このファイル
```

### スタブの目印

スタブ関数には:
- `// TODO(backend):` コメント
- `unimplemented!("backend担当者が実装")` または `log::info!("[stub] ... called")`

これらが残っている = まだ実装されていない。

---

## 5. IPC 契約 (フロントと固定済み、変更不可)

### 5.1 Commands (フロント → Rust)

| コマンド | 引数 | 戻り値 | 副作用 |
|---|---|---|---|
| `timer_start` | なし | `Result<TimerState, AppError>` | StateChanged emit、ticker 起動、water 開始 (mode=full)、active_session 保存 |
| `timer_pause` | なし | `Result<TimerState, AppError>` | StateChanged emit、ticker 停止、water 停止 |
| `timer_reset` | なし | `Result<TimerState, AppError>` | active_session 削除、Idle に戻る |
| `timer_skip` | なし | `Result<TimerState, AppError>` | 現フェーズ即完了、次へ |
| `timer_get_state` | なし | `Result<TimerState, AppError>` | 同期取得のみ |
| `settings_get` | なし | `Result<Settings, AppError>` | 同期取得のみ |
| `settings_set` | `{ settings: Settings }` | `Result<(), AppError>` | store 保存、audio/tray 反映、durations は次セッションから有効 |
| `history_list` | `{ from: string, to: string }` (ISO 8601) | `Result<Vec<SessionRecord>, AppError>` | SQLite SELECT |
| `audio_set_mode` | `{ mode: AudioMode }` | `Result<(), AppError>` | AudioService.set_mode |
| `audio_set_volume` | `{ kind: string, value: f32 }` | `Result<(), AppError>` | kind は "master" \| "water" \| "kakon" |

### 5.2 Events (Rust → フロント)

| イベント | ペイロード | 発火タイミング |
|---|---|---|
| `timer:tick` | `{ remaining_seconds: u32 }` | 走行中 1 秒毎 |
| `timer:state_changed` | `{ phase: Phase, session_count: u32 }` | フェーズ遷移時 |
| `timer:completed` | `{ type: SessionKind }` | セッション完了時 (`work`/`short_break`/`long_break`) |

`AppHandle::emit("timer:tick", payload)` で発火する。

### 5.3 型の同期ルール

- Rust 側の `src-tauri/src/models/*.rs` がソース・オブ・トゥルース
- フロント `src/lib/types/index.ts` は手書きで合わせる
- Rust 側で型を変更したらこのファイルを必ず合わせて修正する (ts-rs/specta は不採用)
- enum は `#[serde(rename_all = "snake_case")]` を付けて TS 側と一致させる (例: `Phase::ShortBreak` → `"short_break"`)

---

## 6. 着手手順 (推奨フロー)

### Day 1 (~6h)

1. **B1.1〜B1.4** モデル + エラー (実は型は全て骨格として用意済み、レビューと微調整のみで足りる可能性)
   - `models/timer_state.rs`, `models/settings.rs`, `models/session.rs`, `error.rs` を確認
   - `cargo build` が通ることを確認
2. **B2.1** `core/timer_machine.rs` — 最重要。先にユニットテスト 10 ケースを書いてから実装 (TDD)
   - `docs/spec/backend/tasks.md` の B2.1 にケース 1〜10 が列挙されている
   - `unimplemented!()` を 1 つずつ消していく

### Day 2 (~6h)

3. **B2.2** `core/ticker.rs` — tokio タスク + 250ms poll
4. **B2.3** `core/persistence.rs` — store + sql の wrapper + Migration
5. **B2.4** `core/audio_service.rs` — rodio 抽象

### Day 3 (~6h)

6. **B3.1** `state.rs` の AppState 完成
7. **B4.1-B4.4** `commands/*.rs` を core に接続 (各コマンドを実装)
8. **B6.1** `lib.rs::run()` の Builder 完成 (handler 全部 manage、setup() で ticker/tray/audio init)

### Day 4 (~6h)

9. **B5.1** `tray.rs` TrayIconBuilder 実装
10. **B7.1** active_session 復元ロジック
11. **B8 シリーズ** 統合テスト + 実機テスト

### Day 5 (~6h)

12. **B9.1〜B9.5** 仕上げ (自動起動 / LSUIElement / アイコン / codesign)

---

## 7. 重要なクリティカルパス・罠

### 7.1 必ず守ること

- **`generate_handler![]` 登録漏れ厳禁** — `lib.rs::run()` の invoke_handler に新コマンドを追加するときは絶対に追記。エラーが出ずに silent fail する
- **async コマンドの借用型禁止** — `&str` ではなく `String` を引数に
- **`SystemTime` 基準で残り時間を計算する** — `Instant` だと macOS スリープ復帰でズレる (`docs/requirements.md` R-3)
- **`active_session` は state 遷移時のみ書く** — tick 毎の write は厳禁 (I/O 削減)
- **音アセット読み込み失敗時は silent fallback** — クラッシュさせない (E2)

### 7.2 ハマりやすいポイント

- `tauri-plugin-sql` の permissions は `sql:default` だけでは足りない。`sql:allow-execute` / `sql:allow-select` も必要 (capability に追加済み)
- `Mutex<TimerMachine>` は std (tokio::sync ではない)。tauri の `State<'_, Mutex<T>>` のベストプラクティス
- rodio の `OutputStream` はライフタイム保持が必要 (drop 時に音が止まる)。AudioService struct で持つ
- `tauri-plugin-store` の `.save()` は明示的に呼ぶ必要がある (auto save なし)

### 7.3 触らないで欲しいもの

- `src/lib/types/index.ts` — IPC 契約変更時を除き、勝手に書き換えない
- `docs/spec/backend/*.md` — 計画ドキュメント。実装中に判断が変わったら PR コメントで提案 → 合意後に更新
- `docs/prototype.html` — ビジュアル参照用

---

## 8. テスト方針

- **core/timer_machine.rs**: 状態遷移 10 ケースを単体テストで先に書く (TDD 推奨)
- **core/persistence.rs**: `:memory:` SQLite を使って migration → insert → select
- **commands/**: 統合テストで AppHandle mock を使う (`tauri::test::mock_app()` 等)
- **実機テスト**: スリープ復帰、ウィンドウ閉時の継続再生、強制 kill 後の active_session 復元 (B8.8-B8.10)

DoD は `docs/spec/backend/tasks.md` の各タスクに明記してあるので、それを満たせばOK。

---

## 9. 連絡・相談

- 設計判断で迷ったら `docs/spec/backend/design.md` の「Key Decisions」「Trade-offs」を確認
- それでも判断つかない場合: Naoya に Slack DM or GitHub issue (リポジトリは現状ローカルのみ。GitHub 公開は後段)
- IPC 契約を変更したい場合: **必ず事前相談**。フロント側の型 + IPC ラッパー + ストアの広範囲に影響
- 仕様書に書かれていない判断が必要な場合: 必ずコミット時のコメントで残す

---

## 10. 最後に

このプロジェクトは:
- **個人プロジェクト** (商用ではない)
- **OSS にする可能性あり**
- **craft 重視** — 動けばいい、ではなく $10k 級の質感を目指す
- **静寂の哲学** — エラートーストすら出さない (console.warn / log で十分)

ステートマシンの完成度がすべての品質を決めます。先にテストを書く、ドメインを Tauri から切り離す、ここを丁寧にやってもらえると非常に助かります。

よろしくお願いします。

— フロント担当 Naoya / 2026-06-01
