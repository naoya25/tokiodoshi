//! 永続化レイヤ。
//!
//! 責務:
//! - settings / active_session: tauri-plugin-store (JSON, 1 ファイル)
//! - sessions 履歴: SQLite (sqlx 直接利用)
//!
//! ## なぜ sqlx を直接使うか
//! `tauri-plugin-sql` v2.4 の `DbPool::execute / select` は `pub(crate)` で外部から
//! 呼べない (commands.rs::wrapper.rs を確認済み)。Rust 側から型安全に行を扱うために
//! sqlx を直接持ち込み、DB ファイルパスだけプラグインと同じ規約
//! (`<app_config_dir>/sessions.db`) に合わせる。
//!
//! `migrations()` は `tauri_plugin_sql::Migration` の Vec を返すので、`lib.rs` 側で
//! Builder へ `.add_migrations("sqlite:sessions.db", migrations())` として登録すれば、
//! プラグイン経由 (フロントの `Database.load`) でも同じ schema が適用される。
//! ただし production の Rust パスは下記 `ensure_db_initialized` が独自に migration を
//! 流すので、プラグイン登録は念のためのバックアップでしかない。

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use tauri::{AppHandle, Manager};
use tauri_plugin_sql::{Migration, MigrationKind};
use tauri_plugin_store::StoreExt;
use tokio::sync::OnceCell;

use crate::error::{AppError, AppResult};
use crate::models::{ActiveSession, SessionKind, SessionRecord, Settings};

// ---------------------------------------------------------------------------
// 定数
// ---------------------------------------------------------------------------

/// settings.json のパス (tauri-plugin-store が `<app_config_dir>/settings.json` に解決する)。
const STORE_PATH: &str = "settings.json";
/// `Settings` を入れる key。
const KEY_SETTINGS: &str = "settings";
/// `ActiveSession` を入れる key。`null` で「進行中なし」を表現。
const KEY_ACTIVE_SESSION: &str = "active_session";

/// SQLite DB のファイル名。`<app_config_dir>/sessions.db` に置かれる
/// (tauri-plugin-sql と同じ規約)。
const DB_FILE_NAME: &str = "sessions.db";
/// `lib.rs` から Builder へ渡すときの URL。
pub const DB_URL: &str = "sqlite:sessions.db";

/// Migration v1 の SQL。テストでも同じ文字列を流すために定数化。
pub const MIGRATION_V1_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type TEXT NOT NULL CHECK(type IN ('work','short_break','long_break')),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    was_completed INTEGER NOT NULL CHECK(was_completed IN (0,1)),
    planned_duration_seconds INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);
"#;

// ---------------------------------------------------------------------------
// migrations() — lib.rs から tauri_plugin_sql::Builder に渡す
// ---------------------------------------------------------------------------

pub fn migrations() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        description: "create sessions table",
        sql: MIGRATION_V1_SQL,
        kind: MigrationKind::Up,
    }]
}

// ---------------------------------------------------------------------------
// 内部: SQLite プール (app 1 プロセスに 1 つ、OnceCell でキャッシュ)
// ---------------------------------------------------------------------------

/// `app.manage(SessionsDb::new())` で持たせるラッパ。
///
/// `Arc` で共有してロック不要。`OnceCell` で 1 回だけ接続を確立する。
#[derive(Default)]
pub struct SessionsDb {
    pool: OnceCell<Arc<SqlitePool>>,
}

impl SessionsDb {
    pub fn new() -> Self {
        Self::default()
    }
}

/// 与えられた AppHandle から DB の絶対パスを解決する
/// (`<app_config_dir>/sessions.db`)。
fn resolve_db_path(app: &AppHandle) -> AppResult<PathBuf> {
    let mut dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Sql(format!("app_config_dir resolve failed: {e}")))?;
    std::fs::create_dir_all(&dir)?;
    dir.push(DB_FILE_NAME);
    Ok(dir)
}

/// プールを取得 (初回は接続 + migration を流す)。
async fn get_or_init_pool(app: &AppHandle) -> AppResult<Arc<SqlitePool>> {
    // `app.try_state::<SessionsDb>()` を信頼。setup() で `.manage(SessionsDb::new())` する前提。
    let db = app
        .try_state::<SessionsDb>()
        .ok_or_else(|| AppError::Sql("SessionsDb not managed (call app.manage(SessionsDb::new()) in setup)".into()))?;

    let pool = db
        .pool
        .get_or_try_init(|| async {
            let path = resolve_db_path(app)?;
            let opts = SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true)
                .foreign_keys(true);
            let pool = SqlitePoolOptions::new()
                .max_connections(4)
                .connect_with(opts)
                .await
                .map_err(|e| AppError::Sql(format!("sqlite connect failed: {e}")))?;
            // Migration v1 を流す (CREATE TABLE IF NOT EXISTS なので冪等)
            sqlx::query(MIGRATION_V1_SQL)
                .execute(&pool)
                .await
                .map_err(|e| AppError::Sql(format!("migration v1 failed: {e}")))?;
            Ok::<Arc<SqlitePool>, AppError>(Arc::new(pool))
        })
        .await?;
    Ok(pool.clone())
}

// ---------------------------------------------------------------------------
// settings (tauri-plugin-store)
// ---------------------------------------------------------------------------

/// 破損 / 未保存時は `Default::default()` を返し、warn ログを出す (要件: store 破損耐性)。
pub async fn load_settings(app: &AppHandle) -> Settings {
    match app.store(STORE_PATH) {
        Ok(store) => match store.get(KEY_SETTINGS) {
            Some(v) => match serde_json::from_value::<Settings>(v) {
                Ok(s) => s,
                Err(e) => {
                    log::warn!(
                        "settings.json の deserialize 失敗、Default::default() を返す: {e}"
                    );
                    Settings::default()
                }
            },
            None => Settings::default(),
        },
        Err(e) => {
            log::warn!("store オープン失敗、Default::default() を返す: {e}");
            Settings::default()
        }
    }
}

pub async fn save_settings(app: &AppHandle, s: &Settings) -> AppResult<()> {
    let store = app
        .store(STORE_PATH)
        .map_err(|e| AppError::Store(format!("open store: {e}")))?;
    let value = serde_json::to_value(s)
        .map_err(|e| AppError::Store(format!("serialize Settings: {e}")))?;
    store.set(KEY_SETTINGS, value);
    store
        .save()
        .map_err(|e| AppError::Store(format!("save: {e}")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// active_session (tauri-plugin-store)
// ---------------------------------------------------------------------------

pub async fn save_active_session(app: &AppHandle, s: &ActiveSession) -> AppResult<()> {
    let store = app
        .store(STORE_PATH)
        .map_err(|e| AppError::Store(format!("open store: {e}")))?;
    let value = serde_json::to_value(s)
        .map_err(|e| AppError::Store(format!("serialize ActiveSession: {e}")))?;
    store.set(KEY_ACTIVE_SESSION, value);
    store
        .save()
        .map_err(|e| AppError::Store(format!("save: {e}")))?;
    Ok(())
}

/// 進行中セッションがない / 破損している場合は `None`。
pub async fn load_active_session(app: &AppHandle) -> Option<ActiveSession> {
    let store = match app.store(STORE_PATH) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("store オープン失敗 (active_session): {e}");
            return None;
        }
    };
    let raw = store.get(KEY_ACTIVE_SESSION)?;
    if raw.is_null() {
        return None;
    }
    match serde_json::from_value::<ActiveSession>(raw) {
        Ok(a) => Some(a),
        Err(e) => {
            log::warn!("active_session の deserialize 失敗、None を返す: {e}");
            None
        }
    }
}

pub async fn clear_active_session(app: &AppHandle) -> AppResult<()> {
    let store = app
        .store(STORE_PATH)
        .map_err(|e| AppError::Store(format!("open store: {e}")))?;
    // `null` を入れて「進行中なし」状態にする (delete でも良いが履歴的に明示)
    store.set(KEY_ACTIVE_SESSION, JsonValue::Null);
    store
        .save()
        .map_err(|e| AppError::Store(format!("save: {e}")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// sessions (SQLite via sqlx)
// ---------------------------------------------------------------------------

/// 1 行 INSERT し、`last_insert_rowid` を返す。
pub async fn record_session(app: &AppHandle, r: &SessionRecord) -> AppResult<i64> {
    let pool = get_or_init_pool(app).await?;
    let type_str = session_kind_str(r.r#type);
    let res = sqlx::query(
        r#"INSERT INTO sessions
           (type, started_at, completed_at, was_completed, planned_duration_seconds)
           VALUES (?1, ?2, ?3, ?4, ?5)"#,
    )
    .bind(type_str)
    .bind(&r.started_at)
    .bind(r.completed_at.as_deref())
    .bind(if r.was_completed { 1_i64 } else { 0_i64 })
    .bind(r.planned_duration_seconds as i64)
    .execute(pool.as_ref())
    .await
    .map_err(|e| AppError::Sql(format!("INSERT sessions: {e}")))?;
    Ok(res.last_insert_rowid())
}

/// `[from, to]` の範囲内の `started_at` を持つ行を `started_at ASC` で返す。
pub async fn list_sessions(
    app: &AppHandle,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> AppResult<Vec<SessionRecord>> {
    let pool = get_or_init_pool(app).await?;
    let from_s = crate::models::session::iso8601(from);
    let to_s = crate::models::session::iso8601(to);
    let rows = sqlx::query(
        r#"SELECT id, type, started_at, completed_at, was_completed, planned_duration_seconds
           FROM sessions
           WHERE started_at >= ?1 AND started_at <= ?2
           ORDER BY started_at ASC"#,
    )
    .bind(&from_s)
    .bind(&to_s)
    .fetch_all(pool.as_ref())
    .await
    .map_err(|e| AppError::Sql(format!("SELECT sessions: {e}")))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id: i64 = row.try_get("id").map_err(map_sqlx_err)?;
        let type_str: String = row.try_get("type").map_err(map_sqlx_err)?;
        let started_at: String = row.try_get("started_at").map_err(map_sqlx_err)?;
        let completed_at: Option<String> = row.try_get("completed_at").map_err(map_sqlx_err)?;
        let was_completed_i: i64 = row.try_get("was_completed").map_err(map_sqlx_err)?;
        let planned: i64 = row
            .try_get("planned_duration_seconds")
            .map_err(map_sqlx_err)?;
        out.push(SessionRecord {
            id,
            r#type: str_to_session_kind(&type_str)?,
            started_at,
            completed_at,
            was_completed: was_completed_i != 0,
            planned_duration_seconds: planned as u32,
        });
    }
    Ok(out)
}

fn map_sqlx_err(e: sqlx::Error) -> AppError {
    AppError::Sql(format!("row decode: {e}"))
}

fn session_kind_str(k: SessionKind) -> &'static str {
    match k {
        SessionKind::Work => "work",
        SessionKind::ShortBreak => "short_break",
        SessionKind::LongBreak => "long_break",
    }
}

fn str_to_session_kind(s: &str) -> AppResult<SessionKind> {
    match s {
        "work" => Ok(SessionKind::Work),
        "short_break" => Ok(SessionKind::ShortBreak),
        "long_break" => Ok(SessionKind::LongBreak),
        other => Err(AppError::Sql(format!("invalid session type: {other}"))),
    }
}

// ---------------------------------------------------------------------------
// Tests — in-memory SQLite (rusqlite) で migration + CRUD を確認
// ---------------------------------------------------------------------------
//
// AppHandle が絡む部分 (store / get_or_init_pool) はここではテストしない。
// SQL 契約だけを rusqlite で検証する (タスク指示通り)。
#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};

    /// rusqlite で migration v1 を流して、`sessions` テーブルがあることを確認するヘルパ。
    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open_in_memory");
        conn.execute_batch(MIGRATION_V1_SQL)
            .expect("run migration v1");
        conn
    }

    #[test]
    fn migration_v1_creates_sessions_table() {
        let conn = fresh_conn();
        // sqlite_master に sessions が存在する
        let name: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='sessions'",
                [],
                |r| r.get(0),
            )
            .expect("sessions table exists");
        assert_eq!(name, "sessions");

        // index も作られている
        let idx: String = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_sessions_started_at'",
                [],
                |r| r.get(0),
            )
            .expect("index exists");
        assert_eq!(idx, "idx_sessions_started_at");
    }

    #[test]
    fn insert_and_select_round_trip() {
        let conn = fresh_conn();
        conn.execute(
            "INSERT INTO sessions (type, started_at, completed_at, was_completed, planned_duration_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                "work",
                "2026-06-01T10:00:00.000Z",
                "2026-06-01T10:25:00.000Z",
                1_i64,
                1500_i64,
            ],
        )
        .expect("insert");

        let last_id: i64 = conn.last_insert_rowid();
        assert!(last_id >= 1, "last_insert_rowid is positive");

        let (kind, started, completed, was_completed, planned): (String, String, Option<String>, i64, i64) =
            conn.query_row(
                "SELECT type, started_at, completed_at, was_completed, planned_duration_seconds
                 FROM sessions WHERE id = ?1",
                params![last_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, i64>(4)?,
                    ))
                },
            )
            .expect("select");
        assert_eq!(kind, "work");
        assert_eq!(started, "2026-06-01T10:00:00.000Z");
        assert_eq!(completed.as_deref(), Some("2026-06-01T10:25:00.000Z"));
        assert_eq!(was_completed, 1);
        assert_eq!(planned, 1500);
    }

    #[test]
    fn range_query_returns_empty_when_out_of_range() {
        let conn = fresh_conn();
        conn.execute(
            "INSERT INTO sessions (type, started_at, completed_at, was_completed, planned_duration_seconds)
             VALUES ('work', '2026-06-01T10:00:00.000Z', NULL, 0, 1500)",
            [],
        )
        .expect("insert");

        // 範囲外の from / to
        let mut stmt = conn
            .prepare(
                "SELECT COUNT(*) FROM sessions
                 WHERE started_at >= ?1 AND started_at <= ?2",
            )
            .unwrap();
        let count: i64 = stmt
            .query_row(
                params!["2026-07-01T00:00:00.000Z", "2026-07-31T23:59:59.999Z"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "範囲外は 0 件");

        // 範囲内
        let count_in: i64 = stmt
            .query_row(
                params!["2026-06-01T00:00:00.000Z", "2026-06-01T23:59:59.999Z"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count_in, 1);
    }

    #[test]
    fn all_session_record_fields_round_trip() {
        // was_completed 0/1, completed_at NULL/値 の全パターンを 1 テストで網羅
        let conn = fresh_conn();
        let cases = [
            (
                "work",
                "2026-06-01T10:00:00.000Z",
                Some("2026-06-01T10:25:00.000Z"),
                true,
                1500_u32,
            ),
            (
                "short_break",
                "2026-06-01T10:25:00.000Z",
                None,
                false,
                300_u32,
            ),
            (
                "long_break",
                "2026-06-01T12:00:00.000Z",
                Some("2026-06-01T12:15:00.000Z"),
                true,
                900_u32,
            ),
        ];
        for (kind, started, completed, was_completed, planned) in cases.iter() {
            conn.execute(
                "INSERT INTO sessions (type, started_at, completed_at, was_completed, planned_duration_seconds)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    kind,
                    started,
                    *completed,
                    if *was_completed { 1_i64 } else { 0_i64 },
                    *planned as i64,
                ],
            )
            .expect("insert case");
        }

        let mut stmt = conn
            .prepare(
                "SELECT type, started_at, completed_at, was_completed, planned_duration_seconds
                 FROM sessions ORDER BY id ASC",
            )
            .unwrap();
        let rows: Vec<(String, String, Option<String>, i64, i64)> = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                ))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].0, "work");
        assert_eq!(rows[0].2.as_deref(), Some("2026-06-01T10:25:00.000Z"));
        assert_eq!(rows[0].3, 1);
        assert_eq!(rows[1].0, "short_break");
        assert_eq!(rows[1].2, None);
        assert_eq!(rows[1].3, 0);
        assert_eq!(rows[2].0, "long_break");
        assert_eq!(rows[2].4, 900);
    }

    #[test]
    fn check_constraint_rejects_invalid_type() {
        let conn = fresh_conn();
        let err = conn.execute(
            "INSERT INTO sessions (type, started_at, completed_at, was_completed, planned_duration_seconds)
             VALUES ('invalid', '2026-06-01T10:00:00.000Z', NULL, 0, 1500)",
            [],
        );
        assert!(err.is_err(), "CHECK 制約違反で Err を返す");
        let msg = format!("{:?}", err.unwrap_err());
        assert!(
            msg.to_lowercase().contains("check") || msg.to_lowercase().contains("constraint"),
            "エラーメッセージに 'check' or 'constraint' を含むこと: {msg}"
        );
    }

    #[test]
    fn check_constraint_rejects_invalid_was_completed() {
        // was_completed の CHECK 制約も効く
        let conn = fresh_conn();
        let err = conn.execute(
            "INSERT INTO sessions (type, started_at, completed_at, was_completed, planned_duration_seconds)
             VALUES ('work', '2026-06-01T10:00:00.000Z', NULL, 2, 1500)",
            [],
        );
        assert!(err.is_err(), "was_completed=2 は CHECK 違反");
    }

    #[test]
    fn migrations_fn_returns_v1() {
        // tauri-plugin-sql Builder へ渡す Migration が v1 で MIGRATION_V1_SQL と一致すること
        let v = migrations();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].version, 1);
        assert_eq!(v[0].sql, MIGRATION_V1_SQL);
    }

    #[test]
    fn session_kind_str_round_trip() {
        for k in [
            SessionKind::Work,
            SessionKind::ShortBreak,
            SessionKind::LongBreak,
        ] {
            let s = session_kind_str(k);
            let back = str_to_session_kind(s).expect("round trip");
            assert_eq!(back, k);
        }
        assert!(str_to_session_kind("bogus").is_err());
    }
}
