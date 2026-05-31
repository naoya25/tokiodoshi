# 音源クレジット

トキオドシで使用する音源の出典とライセンス。すべて **CC0 (Public Domain)** で、クレジット表記は法的には不要ですが、敬意と再現性のため記録します。

## water-loop.mp3 — 作業中ループ音

| 項目 | 内容 |
|---|---|
| タイトル | tsukubai loop (蹲踞ループ) |
| 作者 | TheWandermiles |
| 出典 | https://freesound.org/people/TheWandermiles/sounds/611000/ |
| ライセンス | [CC0 1.0 (Public Domain Dedication)](https://creativecommons.org/publicdomain/zero/1.0/) |
| 再生時間 | 約 232 秒 (3:52) |
| 取得日 | 2026-06-01 |
| 取得URL | https://cdn.freesound.org/previews/611/611000_1696610-hq.mp3 |

**選定理由**: 日本の茶室にある「蹲踞 (つくばい)」を録音した素材。和の静寂・瞑想性に直結し、明確な周期がない滴下音なのでループ用途として継ぎ目を任意点に取れる。

## kakon.mp3 — セッション完了時の1打音

| 項目 | 内容 |
|---|---|
| タイトル | Bamboo hit |
| 作者 | michorvath |
| 出典 | https://freesound.org/people/michorvath/sounds/386888/ |
| ライセンス | [CC0 1.0 (Public Domain Dedication)](https://creativecommons.org/publicdomain/zero/1.0/) |
| 再生時間 | 約 0.60 秒 |
| 取得日 | 2026-06-01 |
| 取得URL | https://cdn.freesound.org/previews/386/386888_3094998-hq.mp3 |

**選定理由**: 竹の棒で叩いた打音。木質感が強く乾いた音で 0.5-0.6 秒の余韻があり、「カコン」要件にほぼ一致。CC0 で改変・商用利用可。

## 差し替えガイド

音素材を差し替える場合:
1. CC0 ライセンスの素材を [freesound.org](https://freesound.org/) で探す (`license:"Creative Commons 0"` でフィルタ)
2. `src-tauri/assets/water-loop.mp3` または `kakon.mp3` を上書き
3. アプリ再起動 (現在は実行時ファイル読み込みのため、ビルド不要)
4. このファイルのエントリを更新
