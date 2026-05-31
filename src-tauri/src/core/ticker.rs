//! 250ms 毎に TimerMachine.poll() を呼ぶ tokio タスク。
//! 終端時刻 (SystemTime) 基準で macOS スリープ復帰にも追従する。
//!
//! TODO(backend): docs/spec/backend/design.md `Ticker 設計` 節 を参照

#![allow(dead_code)]

use tauri::AppHandle;

pub fn spawn(_app: AppHandle) {
    // TODO(backend):
    // tokio::spawn(async move {
    //     let mut interval = tokio::time::interval(Duration::from_millis(250));
    //     loop {
    //         interval.tick().await;
    //         let events = { /* machine.poll(SystemTime::now()) */ };
    //         for ev in events { emit_event(&app, ev).await; }
    //     }
    // });
}
