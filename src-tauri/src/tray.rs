//! macOS メニューバー (Tray) の実装。
//!
//! 仕様:
//! - docs/spec/backend/design.md `Tray 設計` 節
//! - docs/spec/backend/tasks.md B5.1
//! - docs/requirements.md v0.3 F-02
//!
//! 設計判断:
//! - `setup()` は `lib.rs::run().setup()` から呼び、`TrayIcon` を返す。
//!   呼び出し側で `.manage(TrayHandle(tray))` 等で保持し、`update_title` / `update_icon`
//!   から参照する想定 (lib.rs 集約タスクで対応)。
//! - アイコンは `icons/tray-{idle,work,break}.png` を読む。
//!   ファイル不在は許容し、`default_window_icon()` を fallback に使う (warn ログ)。
//! - macOS では `icon_as_template(true)` を指定し、メニューバーの light/dark に追従させる
//!   (要件 F-02: 黒の単色 template image)。
//! - `on_menu_event` のハンドラは Wave1-2 で commands 層が完成していない関数も呼ぶため、
//!   現時点では `AppState` の `TimerMachine` を直接触る形にする。commands と
//!   重複するロジック (start/pause/reset/skip → events を emit + 副作用) は、
//!   後続の lib.rs 集約タスクで共通関数 `execute_timer_action` に括り出す前提。
//!   今回は最低限「メニュー押下で machine が動き、Tick/StateChanged が emit される」
//!   ところまで実装する。副作用 (audio, persistence) は lib.rs 統合時に追加する。
//! - `format_title` / `phase_to_icon_name` は pure function に切り出してユニットテスト可能にする。

#![allow(dead_code)]

use std::path::PathBuf;

use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuEvent},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Manager,
};

use crate::models::{Phase, TimerEvent};
use crate::state::AppState;

/// アプリ起動時に呼ぶ。Tray アイコン + メニュー + on_menu_event を構築。
///
/// 戻り値の `TrayIcon` は呼び出し側で保持し、`update_title` / `update_icon` から
/// 参照する。Drop されると tray が消えるため、`AppState` か `.manage(...)` 経由で
/// プロセス寿命と同じだけ生かす必要がある。
pub fn setup(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let menu = MenuBuilder::new(app)
        .text("start", "開始")
        .text("pause", "一時停止")
        .text("reset", "リセット")
        .text("skip", "スキップ")
        .separator()
        .text("open_window", "ウィンドウを開く")
        .text("open_settings", "設定")
        .separator()
        .text("quit", "終了")
        .build()?;

    let mut builder = TrayIconBuilder::new()
        .title(format_title(0))
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(handle_menu_event);

    // 起動時アイコン: tray-idle.png があればそれ、無ければ default_window_icon を fallback
    if let Some(image) = load_phase_icon(app, Phase::Idle) {
        builder = builder.icon(image);
    } else if let Some(default_icon) = app.default_window_icon().cloned() {
        log::warn!(
            "tray: tray-idle.png が見つからないため default_window_icon にフォールバック"
        );
        builder = builder.icon(default_icon);
    } else {
        log::warn!("tray: アイコン素材が一切無いので icon 未指定で起動");
    }

    builder.build(app)
}

/// ticker や commands から呼ばれる title 更新。
/// 仕様: 残り秒数を `MM:SS` のゼロ埋め (要件 F-02)。
pub fn update_title(tray: &TrayIcon, remaining_seconds: u32) {
    if let Err(e) = tray.set_title(Some(format_title(remaining_seconds))) {
        log::warn!("tray.set_title 失敗: {e}");
    }
}

/// Phase に応じて tray アイコンを切り替える。
/// ファイル不在時は何もしない (warn ログ)。
pub fn update_icon(tray: &TrayIcon, phase: Phase, app: &AppHandle) {
    let Some(image) = load_phase_icon(app, phase) else {
        log::warn!(
            "tray: phase {:?} 用のアイコン素材が無いため切替スキップ",
            phase
        );
        return;
    };
    if let Err(e) = tray.set_icon(Some(image)) {
        log::warn!("tray.set_icon 失敗: {e}");
    }
}

// ---------- 内部実装 ----------

/// メニューイベントのディスパッチ。
/// AppState から `TimerMachine` を借りて操作する。
fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "start" => dispatch_timer_action(app, TimerAction::Start),
        "pause" => dispatch_timer_action(app, TimerAction::Pause),
        "reset" => dispatch_timer_action(app, TimerAction::Reset),
        "skip" => dispatch_timer_action(app, TimerAction::Skip),
        "open_window" => {
            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = window.show() {
                    log::warn!("main window show 失敗: {e}");
                }
                if let Err(e) = window.set_focus() {
                    log::warn!("main window set_focus 失敗: {e}");
                }
            } else {
                log::warn!("tray: main window が見つからない");
            }
        }
        "open_settings" => {
            // TODO(backend): 専用 settings ウィンドウは未実装。当面は main を開く。
            if let Some(window) = app.get_webview_window("settings") {
                let _ = window.show();
                let _ = window.set_focus();
            } else {
                log::info!(
                    "tray: settings ウィンドウは未実装。main を開くフォールバックは別タスクで対応"
                );
            }
        }
        "quit" => {
            app.exit(0);
        }
        other => {
            log::warn!("tray: 未知のメニューID '{other}'");
        }
    }
}

/// commands 層と重複するアクション種別。
/// 将来 `commands/mod.rs` に共通関数 `execute_timer_action(action, app)` を切り出す。
#[derive(Debug, Clone, Copy)]
enum TimerAction {
    Start,
    Pause,
    Reset,
    Skip,
}

/// `TimerMachine` を操作し、結果 events を emit する。
/// commands 側の本実装が入ったら、そちらの共通関数に置き換える。
fn dispatch_timer_action(app: &AppHandle, action: TimerAction) {
    let state = app.state::<AppState>();
    let now = std::time::SystemTime::now();
    let events: Vec<TimerEvent> = {
        let mut machine = match state.machine.lock() {
            Ok(m) => m,
            Err(poisoned) => {
                log::error!("tray: machine lock poisoned ({:?})", action);
                // poison は state.rs の方針 (unwrap fast-fail) に従い回復させない
                poisoned.into_inner()
            }
        };
        match action {
            TimerAction::Start => machine.start(now),
            TimerAction::Pause => machine.pause(now),
            TimerAction::Reset => machine.reset(),
            TimerAction::Skip => machine.skip(now),
        }
    };
    for ev in events {
        emit_timer_event(app, &ev);
    }
}

/// `TimerEvent` をフロント向けに emit。
/// ticker と同じ payload 仕様 (docs/spec/backend/design.md "Data Flow" 節)。
fn emit_timer_event(app: &AppHandle, event: &TimerEvent) {
    use tauri::Emitter;
    let result = match event {
        TimerEvent::Tick(remaining) => app.emit("timer:tick", remaining),
        TimerEvent::StateChanged { phase, count } => app.emit(
            "timer:state_changed",
            serde_json::json!({ "phase": phase, "count": count }),
        ),
        TimerEvent::Completed { kind } => app.emit("timer:completed", kind),
    };
    if let Err(e) = result {
        log::warn!("tray: emit 失敗 {:?}: {}", event, e);
    }
}

/// `src-tauri/icons/tray-{idle,work,break}.png` を読み込む。
/// ファイルが無ければ None (呼び出し側で fallback)。
fn load_phase_icon(app: &AppHandle, phase: Phase) -> Option<Image<'static>> {
    let file_name = phase_to_icon_name(phase);
    let path = resolve_icon_path(app, file_name)?;
    match Image::from_path(&path) {
        Ok(img) => Some(img),
        Err(e) => {
            log::warn!("tray: アイコン読み込み失敗 {path:?}: {e}");
            None
        }
    }
}

/// アイコンファイルパスを解決する。
/// 1. resource_dir / icons/<name>  (バンドル後)
/// 2. CARGO_MANIFEST_DIR / icons/<name>  (dev ビルド時)
fn resolve_icon_path(app: &AppHandle, file_name: &str) -> Option<PathBuf> {
    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join("icons").join(file_name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    // dev/test 用フォールバック (cargo run / cargo test 経由)
    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("icons")
        .join(file_name);
    if dev_path.exists() {
        return Some(dev_path);
    }
    None
}

/// 残り秒数 → `MM:SS` ゼロ埋め文字列。
/// 60 分以上 (3600 秒以上) は `MM` が 2 桁を超える点に注意。要件は 99:59 までしか
/// 想定していないが、念のためあふれても truncate せず素直に format する。
fn format_title(remaining_seconds: u32) -> String {
    let m = remaining_seconds / 60;
    let s = remaining_seconds % 60;
    format!("{:02}:{:02}", m, s)
}

/// Phase → アイコンファイル名のマッピング。
/// Idle / Paused は idle、ShortBreak / LongBreak は break、Work は work。
fn phase_to_icon_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Idle | Phase::Paused => "tray-idle.png",
        Phase::Work => "tray-work.png",
        Phase::ShortBreak | Phase::LongBreak => "tray-break.png",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // tray は Tauri::AppHandle が無いと build できないため、ここでは pure function だけテスト。
    // 実機での動作確認は手動 (DoD: 起動 → メニュー操作 → title が時刻更新)。

    #[test]
    fn format_title_zero() {
        assert_eq!(format_title(0), "00:00");
    }

    #[test]
    fn format_title_two_minutes_five_seconds() {
        assert_eq!(format_title(125), "02:05");
    }

    #[test]
    fn format_title_just_under_one_hour() {
        assert_eq!(format_title(3599), "59:59");
    }

    #[test]
    fn format_title_over_one_hour_does_not_truncate() {
        // 60 分以上は 2 桁を超えるが panic せず素直に出す
        assert_eq!(format_title(3600), "60:00");
    }

    #[test]
    fn phase_to_icon_name_work() {
        assert_eq!(phase_to_icon_name(Phase::Work), "tray-work.png");
    }

    #[test]
    fn phase_to_icon_name_idle_and_paused_share_icon() {
        assert_eq!(phase_to_icon_name(Phase::Idle), "tray-idle.png");
        assert_eq!(phase_to_icon_name(Phase::Paused), "tray-idle.png");
    }

    #[test]
    fn phase_to_icon_name_short_and_long_break_share_icon() {
        assert_eq!(phase_to_icon_name(Phase::ShortBreak), "tray-break.png");
        assert_eq!(phase_to_icon_name(Phase::LongBreak), "tray-break.png");
    }
}
