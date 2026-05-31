//! AudioService: rodio を抽象化した音再生サービス。
//!
//! TODO(backend): docs/spec/backend/design.md `AudioService 設計` 節 を参照
//! - silent / kakon_only / full の 3 モード切替
//! - 水音 (water-loop) のループ再生 + 0.5s フェードイン・アウト
//! - kakon の one-shot 再生
//! - 音アセット読み込み失敗時の silent fallback (E2)

#![allow(dead_code)]

use crate::error::AppResult;
use crate::models::AudioMode;

pub struct AudioService {
    mode: AudioMode,
    master_volume: f32,
}

impl AudioService {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            mode: AudioMode::Full,
            master_volume: 0.7,
        })
    }

    pub fn set_mode(&mut self, mode: AudioMode) {
        self.mode = mode;
    }

    pub fn set_master_volume(&mut self, v: f32) {
        self.master_volume = v.clamp(0.0, 1.0);
    }

    pub fn start_water(&mut self) {
        // TODO(backend): mode=Full のとき rodio で water-loop.mp3 をループ再生 (0.5s fade-in)
    }

    pub fn stop_water(&mut self) {
        // TODO(backend): 0.5s fade-out で停止
    }

    pub fn play_kakon(&mut self) {
        // TODO(backend): mode in [KakonOnly, Full] のとき kakon.mp3 を再生
    }
}
