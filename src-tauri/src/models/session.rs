use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use super::timer_state::SessionKind;

/// 履歴 (sessions テーブル) の 1 行に対応。
/// フロント `SessionRecord` と同期。
///
/// `started_at` / `completed_at` は ISO 8601 UTC 文字列 (`2026-06-01T12:34:56.000Z` 形式)。
/// SQLite TEXT カラムに格納されるため `String` で持つ。
///
/// `type` は Rust の予約語なので raw identifier `r#type` を使う。
/// raw identifier はそのまま `type` としてシリアライズされるが、念のため
/// `#[serde(rename = "type")]` を明示し挙動を固定する。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SessionRecord {
    pub id: i64,
    #[serde(rename = "type")]
    pub r#type: SessionKind,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub was_completed: bool,
    pub planned_duration_seconds: u32,
}

/// クラッシュリカバリ用に store に保存する進行中セッション情報。
/// requirements F8 / M-3 で規定。
///
/// `started_at` / `end_at` は ISO 8601 UTC 文字列。
/// 起動時に `end_at` と現在時刻を比較して `was_completed` を判定する。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ActiveSession {
    #[serde(rename = "type")]
    pub r#type: SessionKind,
    pub started_at: String,
    pub planned_duration_seconds: u32,
    pub end_at: String,
}

/// `chrono::DateTime<Utc>` を ISO 8601 (RFC 3339) 文字列に変換するヘルパ。
/// ミリ秒精度・`Z` 終端で TS の `new Date().toISOString()` と互換になる。
pub fn iso8601(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_record_serialize_uses_type_key() {
        // フロント TS は `type` キーで読むので、Rust の raw identifier `r#type` フィールドが
        // serialize 時に `"type"` になることを担保する
        let r = SessionRecord {
            id: 7,
            r#type: SessionKind::Work,
            started_at: "2026-06-01T10:00:00.000Z".to_string(),
            completed_at: Some("2026-06-01T10:25:00.000Z".to_string()),
            was_completed: true,
            planned_duration_seconds: 1500,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"id\":7"));
        assert!(json.contains("\"type\":\"work\""));
        assert!(json.contains("\"started_at\":\"2026-06-01T10:00:00.000Z\""));
        assert!(json.contains("\"completed_at\":\"2026-06-01T10:25:00.000Z\""));
        assert!(json.contains("\"was_completed\":true"));
        assert!(json.contains("\"planned_duration_seconds\":1500"));

        // ActiveSession も同様に `type` キーが出ること
        let a = ActiveSession {
            r#type: SessionKind::ShortBreak,
            started_at: "2026-06-01T10:25:00.000Z".to_string(),
            planned_duration_seconds: 300,
            end_at: "2026-06-01T10:30:00.000Z".to_string(),
        };
        let aj = serde_json::to_string(&a).unwrap();
        assert!(aj.contains("\"type\":\"short_break\""));
        assert!(aj.contains("\"end_at\":\"2026-06-01T10:30:00.000Z\""));

        // round-trip
        let back: SessionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
        let back_a: ActiveSession = serde_json::from_str(&aj).unwrap();
        assert_eq!(back_a, a);

        // iso8601 ヘルパが TS の toISOString と同じ形を返すこと
        let dt = DateTime::parse_from_rfc3339("2026-06-01T12:34:56.789Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(iso8601(dt), "2026-06-01T12:34:56.789Z");
    }
}
