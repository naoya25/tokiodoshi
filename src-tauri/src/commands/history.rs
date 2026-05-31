//! History commands.
//!
//! `history_list(from, to)`:
//! - `from` / `to` は ISO 8601 (RFC 3339) 文字列。`new Date().toISOString()` 互換。
//! - `chrono::DateTime::parse_from_rfc3339` で UTC にパース。失敗は `AppError::NotFound`。
//! - persistence::list_sessions に渡し、`Vec<SessionRecord>` を返す。

use chrono::{DateTime, Utc};
use tauri::AppHandle;

use crate::core::persistence;
use crate::error::{AppError, AppResult};
use crate::models::SessionRecord;

#[tauri::command]
pub async fn history_list(
    from: String,
    to: String,
    app: AppHandle,
) -> AppResult<Vec<SessionRecord>> {
    let from_dt = parse_iso8601(&from, "from")?;
    let to_dt = parse_iso8601(&to, "to")?;
    persistence::list_sessions(&app, from_dt, to_dt).await
}

/// ISO 8601 (RFC 3339) 文字列を `DateTime<Utc>` にパースする。
/// 失敗は `AppError::NotFound("invalid date: <field>: <input>")` にマップ。
fn parse_iso8601(s: &str, field: &str) -> AppResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| AppError::NotFound(format!("invalid date ({field}): {s}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_iso8601_accepts_utc_z_format() {
        // `new Date().toISOString()` と同形式
        let dt = parse_iso8601("2026-06-01T10:00:00.000Z", "from").expect("valid");
        assert_eq!(dt.to_rfc3339(), "2026-06-01T10:00:00+00:00");
    }

    #[test]
    fn parse_iso8601_accepts_offset_format() {
        // 2026-06-01T19:00:00+09:00 == 2026-06-01T10:00:00Z
        let dt = parse_iso8601("2026-06-01T19:00:00+09:00", "to").expect("valid");
        let utc = parse_iso8601("2026-06-01T10:00:00Z", "to").expect("valid");
        // UTC に変換されているので同じタイムスタンプを指す
        assert_eq!(dt.timestamp(), utc.timestamp());
    }

    #[test]
    fn parse_iso8601_rejects_garbage() {
        let err = parse_iso8601("not-a-date", "from").unwrap_err();
        match err {
            AppError::NotFound(msg) => {
                assert!(msg.contains("invalid date"));
                assert!(msg.contains("from"));
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
    }
}
