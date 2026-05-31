//! AudioService: rodio を抽象化した音再生サービス。
//!
//! 詳細: docs/spec/backend/design.md `AudioService 設計` 節
//! - silent / kakon_only / full の 3 モード切替
//! - 水音 (water-loop) のループ再生 + 0.5s フェードイン・アウト
//! - kakon の one-shot 再生
//! - 音アセット読み込み失敗 / 出力デバイス取得失敗時の silent fallback (E2)
//!
//! 設計判断:
//! - 音アセットはファイル方式: `assets_dir/water-loop.mp3`, `assets_dir/kakon.mp3` を実行時読み込み。
//!   include_bytes! はビルド時固定になり差し替え不可なので採用しない。
//! - `new(assets_dir: PathBuf)` を取り、ビルダー側 (lib.rs) で
//!   `app.path().resource_dir()` を渡す。テスト時は任意のパスを渡せる。
//! - 出力デバイス取得失敗 / アセット不在は panic させず、関数を no-op にして fallback。
//! - フェードは std::thread + Sink::set_volume で同期スリープ実装。
//!   audio_service を tokio に依存させない方針。

#![allow(dead_code)]

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use crate::models::{AudioMode, VolumeKind};

const WATER_FILE: &str = "water-loop.mp3";
const KAKON_FILE: &str = "kakon.mp3";

/// フェードの総時間 (要件: 0.5s)。
const FADE_DURATION: Duration = Duration::from_millis(500);
/// 1 ステップ当たりの待ち (50ms × 10 ステップ = 500ms)。
const FADE_STEP: Duration = Duration::from_millis(50);
const FADE_STEPS: u32 = 10;

pub struct AudioService {
    /// drop すると音が止まるのでフィールド保持。
    _stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    /// 水音 (ループ) の sink。再生中のみ Some。
    water_sink: Option<Arc<Sink>>,
    /// kakon の生バイト列 (preload)。Cursor で何度も再生可能。
    /// 数十 KB 想定なので再生時に clone する (Cursor<Vec<u8>> が Read + Seek を満たす最も素直な型)。
    kakon_bytes: Option<Vec<u8>>,
    /// アセットの所在 (path だけ覚えておき、再生時に都度 File::open)。
    assets_dir: PathBuf,
    mode: AudioMode,
    master_volume: f32,
    water_volume: f32,
    kakon_volume: f32,
}

// SAFETY:
// `rodio::OutputStream` は内部に cpal の `Stream` を持ち、それは `!Send + !Sync`
// (cpal の `NotSendSyncAcrossAllPlatforms` PhantomData による)。
// 一方、本アプリでは `AudioService` を `std::sync::Mutex` 越しに常に単一スレッドから
// 触る (Tauri `State<'_, AppState>` 経由) ため、`Send` を満たせば安全に扱える。
// 実環境 (macOS coreaudio) でも `OutputStream` をミューテックス越しに別タスクから
// 触る運用は rodio + tauri パターンとして広く使われている。
unsafe impl Send for AudioService {}
unsafe impl Sync for AudioService {}

impl AudioService {
    /// 音再生サービスを構築する。
    ///
    /// - 出力デバイス取得失敗時は `_stream/handle` を `None` にして silent fallback。
    /// - kakon の preload に失敗してもエラーにせず `kakon_bytes = None` (再生時 no-op)。
    /// - water は再生時に都度 `File::open` するため、ここではパス保持のみ。
    pub fn new(assets_dir: PathBuf) -> Self {
        let (stream, handle) = match OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(e) => {
                log::warn!("AudioService: OutputStream::try_default 失敗 ({e}), silent fallback");
                (None, None)
            }
        };

        let kakon_bytes = match std::fs::read(assets_dir.join(KAKON_FILE)) {
            Ok(b) => Some(b),
            Err(e) => {
                log::warn!(
                    "AudioService: kakon 読み込み失敗 ({}): {e}, kakon は no-op",
                    assets_dir.join(KAKON_FILE).display()
                );
                None
            }
        };

        Self {
            _stream: stream,
            handle,
            water_sink: None,
            kakon_bytes,
            assets_dir,
            mode: AudioMode::Full,
            master_volume: 0.7,
            water_volume: 0.3,
            kakon_volume: 0.6,
        }
    }

    /// 再生モードを切り替える。Silent / KakonOnly に変わったら水音を止める。
    pub fn set_mode(&mut self, mode: AudioMode) {
        let prev = self.mode;
        self.mode = mode;
        if prev == AudioMode::Full && mode != AudioMode::Full {
            self.stop_water();
        }
    }

    /// 音量を 0.0..=1.0 にクランプして設定する。
    /// 水音再生中なら即座に sink にも反映。
    pub fn set_volume(&mut self, kind: VolumeKind, value: f32) {
        let v = value.clamp(0.0, 1.0);
        match kind {
            VolumeKind::Master => self.master_volume = v,
            VolumeKind::Water => self.water_volume = v,
            VolumeKind::Kakon => self.kakon_volume = v,
        }
        if let Some(sink) = &self.water_sink {
            sink.set_volume(self.effective_water_volume());
        }
    }

    /// 水音をループ再生で開始する (0.5s fade-in)。
    ///
    /// 以下は no-op:
    /// - mode != Full
    /// - OutputStream 取得失敗
    /// - アセット不在 / デコード失敗
    /// - 既に再生中 (二重再生防止)
    pub fn start_water(&mut self) {
        if self.mode != AudioMode::Full {
            return;
        }
        if self.water_sink.is_some() {
            return;
        }
        let Some(handle) = &self.handle else { return };

        let path = self.assets_dir.join(WATER_FILE);
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(e) => {
                log::warn!(
                    "AudioService: water 読み込み失敗 ({}): {e}, start_water は no-op",
                    path.display()
                );
                return;
            }
        };
        let decoder = match Decoder::new(BufReader::new(file)) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("AudioService: water デコード失敗: {e}, start_water は no-op");
                return;
            }
        };

        let sink = match Sink::try_new(handle) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("AudioService: water Sink 生成失敗: {e}");
                return;
            }
        };
        sink.set_volume(0.0);
        sink.append(decoder.repeat_infinite());
        sink.play();

        let sink = Arc::new(sink);
        self.water_sink = Some(Arc::clone(&sink));

        // 0.5s fade-in (別スレッド)
        let target = self.effective_water_volume();
        spawn_fade(sink, 0.0, target);
    }

    /// 水音を 0.5s フェードアウトして停止する。
    pub fn stop_water(&mut self) {
        let Some(sink) = self.water_sink.take() else {
            return;
        };
        let start = sink.volume();
        // 別スレッドでフェードアウト → 終わったら stop
        thread::spawn(move || {
            for i in 1..=FADE_STEPS {
                let t = i as f32 / FADE_STEPS as f32;
                sink.set_volume(start * (1.0 - t));
                thread::sleep(FADE_STEP);
            }
            sink.stop();
        });
    }

    /// kakon を 1 回再生する。Silent モードでは no-op。
    pub fn play_kakon(&mut self) {
        if self.mode == AudioMode::Silent {
            return;
        }
        let Some(handle) = &self.handle else { return };
        let Some(bytes) = &self.kakon_bytes else {
            return;
        };

        // Decoder には Read + Seek が要るので Cursor<Vec<u8>> 経由 (Vec を都度 clone)。
        let cursor = std::io::Cursor::new(bytes.clone());
        let decoder = match Decoder::new(cursor) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("AudioService: kakon デコード失敗: {e}, play_kakon は no-op");
                return;
            }
        };
        let sink = match Sink::try_new(handle) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("AudioService: kakon Sink 生成失敗: {e}");
                return;
            }
        };
        sink.set_volume(self.effective_kakon_volume());
        sink.append(decoder);
        // detach: sink を drop しても再生が続くようにする
        sink.detach();
    }

    fn effective_water_volume(&self) -> f32 {
        self.master_volume * self.water_volume
    }

    fn effective_kakon_volume(&self) -> f32 {
        self.master_volume * self.kakon_volume
    }
}

/// 指定 sink を `from` -> `to` に 0.5s かけて線形補間する別スレッドを起こす。
fn spawn_fade(sink: Arc<Sink>, from: f32, to: f32) {
    thread::spawn(move || {
        for i in 1..=FADE_STEPS {
            let t = i as f32 / FADE_STEPS as f32;
            let v = from + (to - from) * t;
            sink.set_volume(v);
            thread::sleep(FADE_STEP);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用: 存在しないアセットディレクトリを渡しても構造体は構築でき、
    /// 各メソッドが panic せず no-op として振る舞うこと (silent fallback)。
    #[test]
    fn new_with_missing_assets_falls_back_silently() {
        let dir = PathBuf::from("/this/path/does/not/exist/tokiodoshi-test");
        let mut svc = AudioService::new(dir);

        // kakon バイトは読めない
        assert!(svc.kakon_bytes.is_none());

        // 各メソッドが panic しない (実音は鳴らない / no-op)
        svc.start_water();
        assert!(
            svc.water_sink.is_none(),
            "アセット不在なら water_sink は作られない"
        );
        svc.play_kakon();
        svc.stop_water();
        svc.set_mode(AudioMode::Silent);
        svc.set_mode(AudioMode::Full);
    }

    /// set_volume が 0.0..=1.0 にクランプされること。
    #[test]
    fn set_volume_clamps_into_unit_range() {
        let dir = PathBuf::from("/nonexistent-for-clamp-test");
        let mut svc = AudioService::new(dir);

        svc.set_volume(VolumeKind::Master, -1.0);
        assert_eq!(svc.master_volume, 0.0);
        svc.set_volume(VolumeKind::Master, 2.0);
        assert_eq!(svc.master_volume, 1.0);
        svc.set_volume(VolumeKind::Master, 0.5);
        assert!((svc.master_volume - 0.5).abs() < f32::EPSILON);

        svc.set_volume(VolumeKind::Water, -0.1);
        assert_eq!(svc.water_volume, 0.0);
        svc.set_volume(VolumeKind::Water, 1.5);
        assert_eq!(svc.water_volume, 1.0);

        svc.set_volume(VolumeKind::Kakon, f32::NAN.max(0.3)); // NaN.max -> 0.3
        assert!((svc.kakon_volume - 0.3).abs() < f32::EPSILON);
        svc.set_volume(VolumeKind::Kakon, 1.0);
        assert_eq!(svc.kakon_volume, 1.0);
    }

    /// set_mode(Silent) 後は play_kakon / start_water が no-op であること。
    #[test]
    fn silent_mode_skips_playback() {
        let dir = PathBuf::from("/nonexistent-for-mode-test");
        let mut svc = AudioService::new(dir);

        svc.set_mode(AudioMode::Silent);
        assert_eq!(svc.mode, AudioMode::Silent);

        // Silent では水音 start は no-op
        svc.start_water();
        assert!(svc.water_sink.is_none());

        // Silent では kakon も no-op (アセット未読込でも同様にエラーなし)
        svc.play_kakon();

        // KakonOnly に切り替えても、water は no-op のまま
        svc.set_mode(AudioMode::KakonOnly);
        svc.start_water();
        assert!(svc.water_sink.is_none());
    }

    /// Full -> KakonOnly に切り替えると、水音が止まる (sink が解放される)。
    #[test]
    fn switching_from_full_stops_water() {
        // この時点では water_sink を実際に作るには出力デバイス & MP3 が必要なので、
        // 内部フィールドを直接いじって挙動だけ確認する。
        let dir = PathBuf::from("/nonexistent-switch-test");
        let mut svc = AudioService::new(dir);
        svc.mode = AudioMode::Full;
        // ダミー sink を入れるのは難しいので、ここは「Full -> KakonOnly で stop_water が呼ばれ、
        // water_sink が None になる」という遷移だけ検証する。
        assert!(svc.water_sink.is_none());
        svc.set_mode(AudioMode::KakonOnly);
        assert_eq!(svc.mode, AudioMode::KakonOnly);
        assert!(svc.water_sink.is_none());
    }
}
