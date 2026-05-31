# トキオドシ 要件定義書

| 項目 | 内容 |
|---|---|
| 作成日 | 2026-06-01 |
| ステータス | Draft (v0.3) |
| 対象バージョン | MVP |

---

## 1. プロダクト概要

### 1.1 何を作るか

ししおどしのメタファーを用いた macOS 向けポモドーロタイマー。
集中時間と休憩時間を、視覚（ししおどしのアニメーション）と聴覚（水音とカコン音）で表現する。
**通知を出さず、静かに時の流れだけを伝える** ことをコア体験とする。

### 1.2 コアコンセプト

| キーワード | 意味 |
|---|---|
| **静と動** | 集中中は静か、セッション完了時にだけ「動」と「音」が走る |
| **時を落とす** | 通知や催促ではなく、時が「ぽとり」と落ちる感覚を作る |
| **craft 重視** | $10k 級 SaaS の質感を、和のミニマリズムで実現する |

### 1.3 ターゲットユーザー

- 集中作業を業務の中心とする knowledge worker（エンジニア / デザイナー / ライターなど）
- 既存のポモドーロアプリ（Be Focused, Forest, Bear Focus Timer 等）の「通知過多」「派手すぎる UI」に違和感を持つ層
- macOS をメイン作業環境とするユーザー

### 1.4 利用シーン

1. 朝の集中作業開始時にメニューバーから 1 クリックで起動
2. 25 分の作業中は、ししおどしウィンドウを閉じて静かに水音だけ鳴らす（あるいは完全無音）
3. 完了の「カコン」で意識を引き戻し、5 分の休憩中にウィンドウを眺めて余韻に浸る
4. 1 日の終わりに履歴画面でセッション数を確認する

---

## 2. 機能要件

### 2.1 MVP（v1.0）必須機能

#### F-01: タイマー機能
- 作業時間 / 短休憩 / 長休憩を順に巡回する
- デフォルト: 作業 25 分 / 短休憩 5 分 / 長休憩 15 分
- 長休憩は 4 セッション後に挿入される（標準ポモドーロ準拠）
- Start / Pause / Reset / Skip の4操作が可能
- タイマー本体は Rust 側で実行（WebView スリープに影響されない）

#### F-02: メニューバー表示
- macOS メニューバーに常駐
- アイコン + 残り時間 `MM:SS` を表示（**ゼロ埋め固定**: `00:00` 形式で桁数を固定し、proportional フォントでも桁ずれを抑える）
- アイコンは状態に応じて変化（3 種類、いずれも macOS 標準 22pt 高、template image として黒の単色で実装し、メニューバーのライト/ダークに自動追従）
  - `Idle`: 竹筒水平の線画シルエット
  - `Work`: 竹筒が傾き、水滴がある状態
  - `Break`: 竹筒が完全に倒れた状態
- 制約: Tauri v2 `TrayIcon::set_title()` ではフォント指定不可。SF Mono 等は適用できない前提で「ゼロ埋め固定」運用とする
- クリックでメニュー展開
  - 開始 / 一時停止 / リセット / スキップ
  - ウィンドウを開く
  - 設定
  - 終了

#### F-03: ししおどしウィンドウ
- 専用ウィンドウとして開閉可能（メニューバーから「ウィンドウを開く」）
- SVG ベースのししおどしアニメーション
- 状態遷移:
  1. **Idle**: 竹筒水平、無音
  2. **Filling (作業中)**: 進行率 0→100% で竹筒の傾きが 0°→25° に増加、水滴が一定間隔で落ちる
  3. **Tipping (セッション完了 0.8 秒間)**: 竹筒が 25°→60° に倒れ、水排出
  4. **Knock (完了直後 0.3 秒)**: 反動で戻る瞬間にカコン音、石にヒットエフェクト
  5. **Settling (休憩中)**: ゆっくり水平に戻り、新サイクル開始

#### F-04: 音

3 つの音モードを切替可能（デフォルト: `full`）:

| モード | 水音 | カコン音 | 用途 |
|---|---|---|---|
| `silent` | × | × | 完全無音。会議中・図書館・深夜など音を一切出したくないとき |
| `kakon_only` | × | ◯ | 作業中は静寂、完了の合図だけ欲しいとき（最もシンプルな運用） |
| `full` | ◯ | ◯ | 水音で「時が満ちる」体感を作る。標準体験 |

- **水音 (water-loop.mp3)**: 作業中にループ再生、デフォルト音量小（0.3）。0.5 秒フェードイン / アウト
- **カコン音 (kakon.mp3)**: セッション完了時に1回だけ再生、デフォルト音量中（0.6）
- 音量は 0.0〜1.0 で個別調整可能（master / water / kakon）
- 音源は freesound.org の CC0 ライセンス素材を厳選
- **再生実装**: Rust 側で `rodio` を使う。ウィンドウを閉じた状態でも水音は鳴り続ける（WebView の background throttling の影響を受けないため）

#### F-05: 設定永続化
- タイマー長（作業 / 短休憩 / 長休憩 / 長休憩までのセッション数）
- 音量（水音 / カコン音 / マスター）
- ミュート状態
- ログイン時自動起動の有無
- Dock アイコン表示の有無（LSUIElement）
- 永続化先: `tauri-plugin-store`（JSON 1ファイル）

#### F-06: セッション履歴
- 1 セッションごとに 1 レコード保存
- スキーマ:
  ```sql
  CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type TEXT NOT NULL,           -- 'work' | 'short_break' | 'long_break'
    started_at TEXT NOT NULL,     -- ISO 8601
    completed_at TEXT,             -- NULL if interrupted
    was_completed INTEGER NOT NULL -- 0/1
  );
  ```
- 永続化先: `tauri-plugin-sql` (SQLite, `~/Library/Application Support/com.naoya.tokiodoshi/sessions.db`)

### 2.2 MVP に含めない機能（v1.1 以降）

- ❌ 履歴の可視化画面（週次・月次グラフ）
- ❌ タグ・プロジェクト分類
- ❌ Slack ステータス自動更新連携
- ❌ macOS Focus Mode との連携
- ❌ macOS システム通知
- ❌ iCloud / Notion 同期
- ❌ ショートカットキー (Cmd+Shift+T で開始 等)
- ❌ Dock bounce
- ❌ 複数プロファイル

### 2.3 将来検討（v2.0+）

- macOS Focus Mode との双方向連携
- Health データ連携（休憩時に呼吸を促す）
- ウィジェット対応
- iPad / iPhone 版

---

## 3. 非機能要件

### 3.1 パフォーマンス

| 指標 | 目標 |
|---|---|
| 起動時間 | 1.5 秒以内（コールドスタート） |
| メモリ使用量 | アイドル時 80MB 以下 |
| CPU 使用率 | アイドル時 1% 以下、アニメ中 5% 以下 |
| タイマー精度 | 1 秒あたり ±50ms 以内（macOS スリープ復帰後も維持） |

### 3.2 対応環境

- **OS**: macOS 12.0 (Monterey) 以降
- **アーキテクチャ**: Apple Silicon (arm64) / Intel (x86_64) Universal Binary
- **ディスプレイ**: Retina 対応（@2x SVG 描画）

### 3.3 アクセシビリティ

- VoiceOver 対応（タイマー状態を読み上げ可能に）
- ハイコントラストモード対応（システム設定追従）
- 色覚多様性配慮（色だけに依存しない状態表現）

### 3.4 セキュリティ・プライバシー

- 外部ネットワーク通信なし（オフライン完結）
- 個人情報の収集なし
- アナリティクス・テレメトリ送信なし
- 設定・履歴は全てローカル保存
- アプリは codesign + notarize 済みで配布

### 3.5 信頼性

- **クラッシュリカバリ**:
  - 進行中のセッションは `tauri-plugin-store` に `active_session` として保存（`started_at`, `type`, `planned_duration_seconds`, `end_at`）
  - 書き込み頻度: セッション開始時 1 回、状態遷移時のみ。tick ごとの書き込みは行わない（I/O 削減）
  - 起動時に `active_session` が存在し `end_at` を過ぎていれば、`was_completed=1` で `sessions` テーブルに確定
  - 起動時に `active_session` が存在し `end_at` 未到達なら、`was_completed=0` で確定（クラッシュ扱い）
- 設定ファイル破損時はデフォルトに自動フォールバック（読み込み失敗 → デフォルト書き戻し → ログ出力）
- システムスリープ・蓋閉じ → 復帰での継続動作（終端時刻基準の ticker により自動追従、§5.3 参照）

---

## 4. UI/UX 要件

### 4.1 美学的方向性

**ミニマル和モダン**
- 線画ベースの SVG、墨色＋畳色を基調
- 余白を多く取り、要素は最小限
- アニメーションは「物理的な余韻」を残す（Svelte `spring` を活用）
- Linear / Things / Bear Focus Timer のような craft 感

**テーマ追従の挙動**
- システムテーマ (`prefers-color-scheme`) に追従
- アプリ起動中に OS テーマが切り替わった瞬間にも即時反映する（CSS variables + `matchMedia` イベントリスナー）
- 設定で `system` / `light` / `dark` を明示選択可能

### 4.2 カラーパレット（暫定）

| 用途 | 値 | 備考 |
|---|---|---|
| 背景（畳色） | `#F5EFE0` | ライトモード基調 |
| 墨色（主線） | `#2B2B2B` | テキスト・SVG 線 |
| 水色 | `#7BA7BC` | 水滴・ループバー |
| アクセント（石） | `#8A8580` | 石・ヒット時に白光 |
| 背景（夜） | `#1A1A1A` | ダークモード基調 |

ダークモード対応は MVP に含む。システム設定に追従。

### 4.3 タイポグラフィ

- カウントダウン表示: `SF Mono`（macOS 標準等幅）または `JetBrains Mono`
- 設定 UI: `SF Pro Text`（macOS 標準）
- 数字は tabular figures（桁ずれ防止）

### 4.4 サイズ・ウィンドウ

| ウィンドウ | サイズ | 振る舞い |
|---|---|---|
| ししおどしメイン | 480 × 640 px | リサイズ可、最小 360 × 480 |
| 設定 | 480 × 560 px | 固定サイズ、モーダル風 |

メニューバーアイコンサイズは macOS 標準（22pt 高）。

### 4.5 振る舞い

- 起動時はウィンドウを開かない（メニューバー常駐のみ）
- メニューバーから「ウィンドウを開く」で表示
- ウィンドウを閉じてもアプリは終了しない（メニューから明示的に終了）
- セッション開始時にウィンドウを自動前面化（設定で OFF 可能）

---

## 5. 技術要件

### 5.1 採用スタック

| 層 | 採用 |
|---|---|
| シェル | Tauri v2 |
| フロント | SvelteKit + adapter-static (SPA mode) |
| 言語 (フロント) | TypeScript |
| Svelte リアクティビティ | Runes ($state / $derived) — 旧 stores は使わない |
| タイマーロジック | Rust（pure logic を `core/` に分離、Tauri 非依存） |
| 設定永続化 | `tauri-plugin-store` |
| 履歴永続化 | `tauri-plugin-sql` (SQLite) |
| 音 | Rust 側で `rodio` クレート (ウィンドウ閉時も再生継続) |
| アニメ | Svelte `tweened` / `spring` + SVG |

### 5.2 ディレクトリ構成

```
tokiodoshi/
├── src/                                    # SvelteKit フロント
│   ├── routes/
│   │   ├── +layout.svelte
│   │   ├── +layout.ts                      # ssr=false, prerender=false
│   │   ├── +page.svelte                    # ししおどしメイン
│   │   ├── settings/+page.svelte
│   │   └── history/+page.svelte            # v1.1+
│   └── lib/
│       ├── components/                     # ShishiOdoshi, BambooTube, WaterDrop, Stone
│       ├── stores/                         # timer.svelte.ts (runes)
│       ├── ipc/                            # invoke ラッパー (型付き)
│       ├── types/                          # Rust 型に対応する手書き TS 型
│       └── audio/                          # Web Audio player + assets
├── static/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs                         # 薄いパススルー
│   │   ├── lib.rs                          # builder + invoke_handler 集約
│   │   ├── state.rs                        # AppState (Mutex)
│   │   ├── error.rs                        # thiserror で AppError
│   │   ├── commands/                       # 機能別 Tauri コマンド
│   │   │   ├── timer.rs
│   │   │   ├── settings.rs
│   │   │   └── history.rs
│   │   ├── core/                           # Tauri 非依存のドメインロジック
│   │   │   ├── timer_machine.rs
│   │   │   ├── ticker.rs
│   │   │   └── persistence.rs
│   │   ├── models/                         # serde 型定義
│   │   ├── tray.rs                         # TrayIconBuilder
│   │   └── tray_menu.rs
│   ├── capabilities/default.json
│   ├── icons/
│   ├── tauri.conf.json
│   ├── Cargo.toml                          # [lib] tokiodoshi_lib, crate-type 3種
│   └── build.rs
├── svelte.config.js                        # adapter-static, fallback: 'index.html'
├── vite.config.ts                          # server.port = 1420
├── tsconfig.json
├── package.json
└── CREDITS.md                              # 音源クレジット
```

### 5.3 アーキテクチャ原則

1. **タイマー本体は Rust に持つ** — WebView のバックグラウンド間引きを避けるため
2. **`core/` は Tauri 非依存** — `cargo test` でユニットテスト可能に保つ
3. **`lib.rs` を集約点に** — モバイル化の余地を残す（公式規約準拠）
4. **TS 型は Rust 型から手書き同期** — ts-rs / specta は導入しない（型は数個のみ）
5. **Runes で統一** — stores と runes の混在を避ける
6. **IPC は `core` イベント + コマンド** — tick / state_changed / complete の3種
7. **ticker は「終端時刻基準」** — セッション開始時に `end_at = Instant::now() + duration` を保持し、tick 毎に `end_at - Instant::now()` で残時間を再計算する。`tokio::time::interval` の累積ドリフトとスリープ復帰時の挙動を一発で吸収する
8. **DB マイグレーションは Rust 側で宣言** — `tauri-plugin-sql` の `Migration` struct を `lib.rs` で builder に登録する。フロントから ad-hoc に DDL を打たない

### 5.4 IPC 設計

#### コマンド（フロント → Rust）

| コマンド | 引数 | 戻り値 |
|---|---|---|
| `timer_start` | - | `TimerState` |
| `timer_pause` | - | `TimerState` |
| `timer_reset` | - | `TimerState` |
| `timer_skip` | - | `TimerState` |
| `timer_get_state` | - | `TimerState` |
| `settings_get` | - | `Settings` |
| `settings_set` | `Settings` | `()` |
| `history_list` | `from: ISO, to: ISO` | `Vec<SessionRecord>` |

#### イベント（Rust → フロント）

| イベント | ペイロード | 頻度 |
|---|---|---|
| `timer:tick` | `{ remaining_seconds: u32 }` | 1秒ごと |
| `timer:state_changed` | `{ phase, session_count }` | 状態遷移時 |
| `timer:completed` | `{ type: 'work'/'short_break'/'long_break' }` | セッション完了時 |

### 5.5 capability（パーミッション）

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-close",
    "core:window:allow-hide",
    "core:window:allow-show",
    "core:window:allow-set-focus",
    "store:default",
    "sql:default",
    "sql:allow-execute",
    "sql:allow-select"
  ]
}
```

---

## 6. データ要件

### 6.1 設定スキーマ (`tauri-plugin-store`)

```typescript
interface Settings {
  durations: {
    work_seconds: number;        // default: 1500 (25min)
    short_break_seconds: number; // default: 300 (5min)
    long_break_seconds: number;  // default: 900 (15min)
    sessions_until_long_break: number; // default: 4
  };
  audio: {
    mode: 'silent' | 'kakon_only' | 'full'; // default: 'full'
    master_volume: number;       // 0.0 - 1.0, default: 0.7
    water_volume: number;        // 0.0 - 1.0, default: 0.3
    kakon_volume: number;        // 0.0 - 1.0, default: 0.6
  };
  behavior: {
    launch_at_login: boolean;    // default: false
    hide_dock_icon: boolean;     // default: false (LSUIElement)
    auto_show_window_on_start: boolean; // default: true
  };
  appearance: {
    theme: 'system' | 'light' | 'dark'; // default: 'system'
  };
}
```

**設定変更の反映タイミング**:

| カテゴリ | 反映タイミング | 理由 |
|---|---|---|
| `durations.*` | **次セッションから** | 進行中セッションの長さを途中で変えると体験が破綻する |
| `audio.*` | **即時** | 音量・モードの即時反映は UX 向上に直結 |
| `behavior.launch_at_login` | 即時（`launchctl` 反映） | OS 側設定への書き込み |
| `behavior.hide_dock_icon` | **要アプリ再起動** | `LSUIElement` は Info.plist 由来で動的反映が不完全。動的に `NSApp.setActivationPolicy(.accessory)` を呼ぶ実装を将来検討（MVP では再起動モーダル表示） |
| `behavior.auto_show_window_on_start` | 即時 | 次のセッション開始から効く |
| `appearance.theme` | 即時 | CSS variables 切替 |

### 6.2 履歴スキーマ (SQLite)

```sql
CREATE TABLE IF NOT EXISTS sessions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  type TEXT NOT NULL CHECK(type IN ('work', 'short_break', 'long_break')),
  started_at TEXT NOT NULL,        -- ISO 8601 UTC
  completed_at TEXT,                -- NULL if interrupted
  was_completed INTEGER NOT NULL CHECK(was_completed IN (0, 1)),
  planned_duration_seconds INTEGER NOT NULL
);

CREATE INDEX idx_sessions_started_at ON sessions(started_at);
```

---

## 7. 制約・前提

### 7.1 制約

- macOS 専用（Windows / Linux は対象外）
- オフライン完結（ネットワーク機能なし）
- 1人開発のため、サブスク・課金・サーバー機能は持たない

### 7.2 前提

- ユーザーはポモドーロテクニックの基本を知っている
- ユーザーは macOS の「メニューバー常駐アプリ」というメンタルモデルを持っている

### 7.3 配布チャネル

- **配布形式**: notarize 済み `.dmg`
- **配布チャネル**: GitHub Releases（プライマリ）
- **自動更新**: MVP では入れない。Sparkle / `tauri-plugin-updater` は v1.1+ で検討
- **コード署名**: Apple Developer ID（個人加入、$99/year）必須
- **CI**: GitHub Actions で macOS runner からの自動 codesign + notarize を将来構築。MVP は手動でも可

---

## 8. 段階的リリース計画

### Phase 0: セットアップ（0.5 日）
- `npm create tauri-app@latest tokiodoshi -- --template svelte-ts` で初期化
- 推奨ディレクトリ構造へリファクタ
- `tauri-plugin-store`, `tauri-plugin-sql` 追加
- macOS で `npm run tauri dev` が起動することを確認

### Phase 1: タイマーコア（1.5 日）
- `core/timer_machine.rs` の純 Rust ステートマシン
- `core/ticker.rs`: **終端時刻基準**で実装（`end_at: Instant` を保持し `end_at - Instant::now()` で残時間計算、`tokio::time::interval` は 1s 周期のドライブのみ）
- `commands/timer.rs` の薄い IPC 層
- `timer:tick` / `timer:state_changed` / `timer:completed` イベント発火
- **ユニットテスト戦略** (`cargo test`、`core/` は Tauri 非依存なので素のテストが書ける):
  - 状態遷移テーブル: `Idle → Work → ShortBreak → Work → ShortBreak → Work → ShortBreak → Work → LongBreak → Idle` のフルサイクル
  - スキップ: 各状態で `skip()` を呼んで次状態に進むこと
  - ポーズ: `Work` 中に pause → 残時間が固定 → resume で再開
  - リセット: 任意状態で `reset()` → `Idle`、進行中セッションは `was_completed=0` で記録
  - 長休憩境界: `sessions_until_long_break` のカウンタ動作
  - スリープ復帰相当: `end_at` を 30 秒前の `Instant` にセットしてから tick → 即座に `completed` 発火
  - 設定変更: 進行中セッションには反映されないこと、次セッションには反映されること

### Phase 2: メニューバー（1 日）
- `tray.rs` で TrayIconBuilder
- title に `MM:SS` 表示（**ゼロ埋め固定**、フォント指定不可前提）
- アイコン素材作成（template image 3 種: Idle / Work / Break、いずれも黒単色 SVG → PNG @2x）
- 状態に応じて `set_icon()` で切替
- メニュー（Start / Pause / Open / Settings / Quit）

### Phase 3: ししおどしアニメ（1.5 日）
- SVG コンポーネント設計（BambooTube / WaterDrop / Stone）
- Svelte `tweened` で水量増加、`spring` で傾き戻り
- 5 状態の遷移を組み込み

### Phase 4: 音（0.5 日）
- freesound.org から CC0 音源を選定（水音・カコン音）
- Rust 側に `audio` モジュール実装 (`rodio` で loop / one-shot / volume / 0.5s fade)
- 3 モード (`silent` / `kakon_only` / `full`) のディスパッチ
- `audio_set_mode` / `audio_set_volume` コマンド追加
- `CREDITS.md` 作成

### Phase 5: 設定 + 履歴（1 日）
- 設定画面（Svelte）
- `tauri-plugin-store` 読み書き
- SQLite スキーマ migration + 履歴記録

### Phase 6: 仕上げ（1.5-2 日）
- アプリアイコン `.icns` 作成（Dock 表示用、メニューバーアイコンとは別）
- ログイン時自動起動（`tauri-plugin-autostart` 推奨、`launchctl` 直叩きより堅牢）
- `LSUIElement` 切替（MVP は要アプリ再起動モーダル）
- Apple Developer ID 加入 + 証明書発行
- codesign + notarize（手動、GitHub Actions 化は v1.1+）
- DMG 作成 + GitHub Releases へのアップロード

**合計工数: 8-10 日（人間時間、レビュー反映後の現実的見積もり）**

---

## 9. 成功指標

### MVP リリース時点

- [ ] 1 セッション（25 分）を完走できる
- [ ] メニューバーに残り時間が正確に表示される
- [ ] セッション完了時に「カコン」音が 1 回だけ鳴る
- [ ] ししおどしの傾き〜戻りアニメが破綻なく動く
- [ ] 設定が再起動後も保持される
- [ ] CPU 使用率がアイドル時 1% 以下
- [ ] 通知を一切出さない

### 個人的な craft 基準

- 起動 → 1 セッション完走の体験が「気持ちいい」と感じられる
- 友人にスクショを見せて「これ何のアプリ？」と聞かれる
- 自分が日常使いし続けられる

---

## 10. 未確定事項（実装中に判断）

1. アプリアイコン `.icns` の絵柄 — ししおどしのシルエットか、抽象幾何か
2. カコン音の余韻長 — 0.5秒 / 1秒 / 1.5秒のどれが「心地よい」か実音で決める
3. 水滴の頻度 — 進行率比例 or 一定間隔
4. ダークモードのカラー詳細
5. 自動起動の実装方式（`launchctl` 直接 or プラグイン）

---

## 変更履歴

| 日付 | バージョン | 変更内容 | 担当 |
|---|---|---|---|
| 2026-06-01 | v0.1 | 初版作成 | Naoya |
| 2026-06-01 | v0.2 | 音を 3 モード (`silent`/`kakon_only`/`full`) に拡張。音再生を Rust 側 `rodio` に変更 (ウィンドウ閉時も継続)。フェード仕様明記。 | Naoya |
| 2026-06-01 | v0.3 | レビュー指摘 R-2〜R-5 / M-2〜M-8 を反映。ticker 終端時刻基準、クラッシュリカバリ仕様、設定反映タイミング表、テスト戦略、メニューバーアイコン仕様、ダークモード追従、配布チャネル、DB migration を追加。工数を 8-10 日に補正。 | Naoya |
