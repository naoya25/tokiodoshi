# トキオドシ

> ししおどしの「静と動」をポモドーロに重ねる、macOS メニューバー常駐型タイマー。

集中の25分、休憩の5分。
時が満ちると、竹筒がそっと傾き、石を打つ。**カコン。**

ノイズを足さず、時の流れを「音と動き」だけで知らせる、craft 重視のポモドーロアプリ。

---

## コンセプト

- **静と動のリズム** — 作業中は静かに水が満ちる。完了時に一度だけ「カコン」が鳴る
- **時を落とす道具** — 通知は出さない。Slack も Mail も鳴らさない。あなたの集中を守る
- **メニューバー常駐** — Dock を占有しない。必要なときだけ専用ウィンドウを開く

## 主な機能（MVP）

- ✅ メニューバーにカウントダウン表示 (`MM:SS`)
- ✅ ししおどしの SVG アニメーション（ミニマル和モダン）
- ✅ 3 つの音モード切替（完全無音 / カコン音のみ / 水音 + カコン音）
- ✅ 自由設定可能なタイマー（デフォルト 25分作業 / 5分休憩 / 15分長休憩）
- ✅ セッション履歴の永続化（SQLite）

## 技術スタック

- **シェル**: [Tauri v2](https://v2.tauri.app/)
- **フロント**: [SvelteKit](https://kit.svelte.dev/) (adapter-static / SPA mode) + TypeScript + Runes
- **タイマーコア**: Rust（バックグラウンドでも正確に走るステートマシン）
- **永続化**: `tauri-plugin-store` (設定) + `tauri-plugin-sql` (履歴)
- **音**: Rust 側 `rodio` クレート（ウィンドウ閉時も継続再生）
- **対応 OS**: macOS 12.0+（Apple Silicon / Intel）

詳細は [`docs/requirements.md`](docs/requirements.md) を参照。

## セットアップ（開発）

```bash
# 依存インストール
npm install

# 開発モード起動
npm run tauri dev

# macOS 向けビルド
npm run tauri build
```

### 前提

- Node.js 20+
- Rust 1.77+（`rustup` 推奨）
- macOS 12.0+
- Xcode Command Line Tools

## プロジェクト構造

```
tokiodoshi/
├── src/                    # SvelteKit フロントエンド
├── src-tauri/              # Rust バックエンド
│   ├── src/
│   │   ├── lib.rs          # builder + invoke_handler 集約
│   │   ├── commands/       # Tauri コマンド (機能別分割)
│   │   ├── core/           # ドメインロジック (Tauri 非依存)
│   │   ├── models/         # serde 型定義
│   │   └── tray.rs         # メニューバー
│   └── tauri.conf.json
├── docs/
│   └── requirements.md     # 要件定義書
└── README.md
```

## ライセンス

未定（個人プロジェクト）。

## 音源のクレジット

音源は [freesound.org](https://freesound.org/) の CC0 ライセンス素材を厳選して使用予定。
詳細は `CREDITS.md`（作成予定）に記載する。
