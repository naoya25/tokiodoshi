//! ドメインロジック (Tauri 非依存)。`cargo test` で単体テスト可能に保つ。
//!
//! TODO(backend): 全モジュールを実装する。詳細は docs/spec/backend/design.md
//! と docs/spec/backend/tasks.md Phase 2 を参照

pub mod audio_service;
pub mod persistence;
pub mod ticker;
pub mod timer_machine;
