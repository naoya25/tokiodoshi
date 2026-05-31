use serde::{Deserialize, Serialize};

use super::timer_state::SessionKind;

/// 履歴 (sessions テーブル) の 1 行に対応。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: SessionKind,
    pub started_at: String,        // ISO 8601
    pub completed_at: Option<String>,
    pub was_completed: bool,
    pub planned_duration_seconds: u32,
}

/// クラッシュリカバリ用に store に保存する進行中セッション情報。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSession {
    #[serde(rename = "type")]
    pub kind: SessionKind,
    pub started_at: String,         // ISO 8601 UTC
    pub planned_duration_seconds: u32,
    pub end_at: String,             // ISO 8601 UTC, 完了予定時刻
}
