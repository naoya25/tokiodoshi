//! トキオドシ — Tauri アプリのエントリポイント。
//!
//! ここは集約のみに留め、ロジックは `core/` (Tauri 非依存)
//! と `commands/` (薄い IPC 層) に分離する。
//! 詳細: `docs/spec/backend/design.md`
//!
//! ## run() の責務 (Wave 3 SubI)
//! 1. env_logger 初期化 + panic_hook 登録 (B6.1)
//! 2. tauri-plugin-store と tauri-plugin-sql を登録
//!    (sql は `persistence::migrations()` で sessions テーブルを作る)
//! 3. setup():
//!    a. `SessionsDb` を manage (Rust 側から sqlx で叩く DB プール)
//!    b. settings をロードして `TimerConfig` を派生
//!    c. assets_dir を解決して `AppState` を構築・manage
//!    d. AudioService に mode/volumes を反映
//!    e. クラッシュリカバリ (B7.1): `active_session` があれば end_at と now の
//!       比較で `was_completed` を判定し、`sessions` に INSERT → clear
//!    f. ticker::spawn / tray::setup
//! 4. invoke_handler に 10 コマンドを登録

mod commands;
mod core;
mod error;
mod models;
mod state;
mod tray;

use chrono::{DateTime, Utc};
use tauri::Manager;

/// `active_session.end_at` (ISO 8601) と現在時刻から `was_completed` を判定する。
///
/// - `now >= end_at` なら自然完了とみなし `true`
/// - `now <  end_at` ならクラッシュ扱いで `false`
/// - パース不能なら安全側 `false`
///
/// pure 関数として切り出し、`setup()` の async ブロックに依存せずユニットテスト可能にしている。
pub(crate) fn determine_was_completed(end_at_iso: &str, now: DateTime<Utc>) -> bool {
    match DateTime::parse_from_rfc3339(end_at_iso) {
        Ok(end) => now >= end.with_timezone(&Utc),
        Err(_) => false,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    // panic_hook:
    // panic 時に AppHandle へアクセスできず active_session の save は実現困難
    // (Mutex poison の可能性も高い)。MVP では log 記録のみ。
    // active_session は ticker / timer_start 側で逐次保存されているので、
    // 直前のフェーズ遷移時点までは復元可能。
    std::panic::set_hook(Box::new(|info| {
        log::error!("PANIC: {info}");
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(core::persistence::DB_URL, core::persistence::migrations())
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();

            // Rust 側からも sqlx で sessions.db を叩くためのプールラッパ
            // (persistence::get_or_init_pool が `try_state::<SessionsDb>()` で取得する)
            handle.manage(core::persistence::SessionsDb::new());

            // setup は同期。async の load_settings / クラッシュリカバリを block_on で実行する。
            // block_on は tauri::async_runtime のスレッドで動くので main runtime と衝突しない。
            let (settings, assets_dir) = tauri::async_runtime::block_on(async {
                let settings = core::persistence::load_settings(&handle).await;
                let assets_dir = handle
                    .path()
                    .resource_dir()
                    .map(|p| p.join("assets"))
                    .unwrap_or_else(|_| std::env::temp_dir().join("tokiodoshi-assets"));
                (settings, assets_dir)
            });

            // AppState 構築 (config は durations から派生) + manage
            let config = (&settings.durations).into();
            let app_state = state::AppState::new(config, assets_dir);

            // AudioService に永続化された mode / volumes を反映
            {
                let mut audio = match app_state.audio.lock() {
                    Ok(g) => g,
                    Err(poisoned) => {
                        log::warn!("setup: AudioService mutex poisoned, recovering");
                        poisoned.into_inner()
                    }
                };
                audio.set_mode(settings.audio.mode);
                audio.set_volume(models::VolumeKind::Master, settings.audio.master_volume);
                audio.set_volume(models::VolumeKind::Water, settings.audio.water_volume);
                audio.set_volume(models::VolumeKind::Kakon, settings.audio.kakon_volume);
            }

            handle.manage(app_state);

            // ---------- クラッシュリカバリ (B7.1) ----------
            tauri::async_runtime::block_on(async {
                if let Some(active) = core::persistence::load_active_session(&handle).await {
                    let now = Utc::now();
                    let was_completed = determine_was_completed(&active.end_at, now);

                    // completed_at:
                    // - was_completed=true: end_at をそのまま使う (本来完了したはずの時刻)
                    // - was_completed=false: None (クラッシュなので未完了)
                    let completed_at = if was_completed {
                        DateTime::parse_from_rfc3339(&active.end_at)
                            .ok()
                            .map(|d| models::iso8601(d.with_timezone(&Utc)))
                    } else {
                        None
                    };

                    let record = models::SessionRecord {
                        id: 0, // INSERT 時に AUTOINCREMENT で採番
                        r#type: active.r#type,
                        started_at: active.started_at,
                        completed_at,
                        was_completed,
                        planned_duration_seconds: active.planned_duration_seconds,
                    };

                    if let Err(e) = core::persistence::record_session(&handle, &record).await {
                        log::warn!("crash recovery: record_session failed: {e}");
                    }
                    if let Err(e) = core::persistence::clear_active_session(&handle).await {
                        log::warn!("crash recovery: clear_active_session failed: {e}");
                    }
                    log::info!(
                        "crash recovery: previous session restored (was_completed={was_completed})"
                    );
                }
            });

            // ---------- ticker spawn ----------
            core::ticker::spawn(handle.clone());

            // ---------- tray setup ----------
            // TrayIcon は drop されると消えるので manage で寿命を保持する
            match tray::setup(&handle) {
                Ok(tray) => {
                    handle.manage(tray);
                }
                Err(e) => {
                    log::warn!("tray::setup failed (non-fatal): {e}");
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::timer::timer_start,
            commands::timer::timer_pause,
            commands::timer::timer_reset,
            commands::timer::timer_skip,
            commands::timer::timer_get_state,
            commands::settings::settings_get,
            commands::settings::settings_set,
            commands::history::history_list,
            commands::audio::audio_set_mode,
            commands::audio::audio_set_volume,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    /// `end_at` が過去 → 正常完了扱い (true)
    #[test]
    fn determine_was_completed_returns_true_when_end_at_is_past() {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        // end_at が 10 分前
        let end_at = "2026-06-01T11:50:00.000Z";
        assert!(determine_was_completed(end_at, now));
    }

    /// `end_at` が未来 → クラッシュ扱い (false)
    #[test]
    fn determine_was_completed_returns_false_when_end_at_is_future() {
        let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        // end_at が 5 分後
        let end_at = "2026-06-01T12:05:00.000Z";
        assert!(!determine_was_completed(end_at, now));
    }

    /// `end_at` がパース不能 → 安全側で false (クラッシュ扱い)
    #[test]
    fn determine_was_completed_returns_false_on_parse_error() {
        let now = Utc::now();
        assert!(!determine_was_completed("not-an-iso8601", now));
        assert!(!determine_was_completed("", now));
        assert!(!determine_was_completed("2026-06-32T99:99:99Z", now));
    }
}
