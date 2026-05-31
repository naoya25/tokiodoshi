//! Audio commands.
//!
//! settings_set からも同じ AudioService が触られるが、フロントが
//! 「mode を切るためだけに Settings 全体を保存する」のは過剰なので、
//! 即時反映用の薄いコマンドを別途用意する (永続化はしない)。
//! 設定 UI 側では settings_set を使う想定。

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::models::{AudioMode, VolumeKind};
use crate::state::AppState;

#[tauri::command]
pub async fn audio_set_mode(mode: AudioMode, state: State<'_, AppState>) -> AppResult<()> {
    let mut audio = match state.audio.lock() {
        Ok(g) => g,
        Err(poisoned) => {
            log::warn!("audio_set_mode: AudioService mutex poisoned, recovering");
            poisoned.into_inner()
        }
    };
    audio.set_mode(mode);
    Ok(())
}

#[tauri::command]
pub async fn audio_set_volume(
    kind: String,
    value: f32,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let kind_enum = kind_str_to_volume_kind(&kind)?;
    let mut audio = match state.audio.lock() {
        Ok(g) => g,
        Err(poisoned) => {
            log::warn!("audio_set_volume: AudioService mutex poisoned, recovering");
            poisoned.into_inner()
        }
    };
    audio.set_volume(kind_enum, value);
    Ok(())
}

/// `"master" | "water" | "kakon"` 文字列を `VolumeKind` に変換する。
/// それ以外は `AppError::NotFound`。
///
/// なぜ AudioMode のように `#[serde(rename_all = "snake_case")]` enum で受けないか:
/// フロント側の型は `VolumeKind` のリテラル union (`"master" | "water" | "kakon"`) で、
/// Tauri invoke の payload は JSON。enum で受けると `{ kind: "master", value: 0.5 }` の
/// 形になるが、フロントが `invoke('audio_set_volume', { kind: 'master', value: 0.5 })` で
/// プリミティブとして渡せる方が DX が良い。明示的に String → enum を変換する。
fn kind_str_to_volume_kind(s: &str) -> AppResult<VolumeKind> {
    match s {
        "master" => Ok(VolumeKind::Master),
        "water" => Ok(VolumeKind::Water),
        "kakon" => Ok(VolumeKind::Kakon),
        other => Err(AppError::NotFound(format!("invalid kind: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_str_to_volume_kind_maps_known_values() {
        assert_eq!(kind_str_to_volume_kind("master").unwrap(), VolumeKind::Master);
        assert_eq!(kind_str_to_volume_kind("water").unwrap(), VolumeKind::Water);
        assert_eq!(kind_str_to_volume_kind("kakon").unwrap(), VolumeKind::Kakon);
    }

    #[test]
    fn kind_str_to_volume_kind_rejects_unknown_and_returns_not_found() {
        let err = kind_str_to_volume_kind("invalid").unwrap_err();
        match err {
            AppError::NotFound(msg) => {
                assert!(msg.contains("invalid kind"));
                assert!(msg.contains("invalid"));
            }
            other => panic!("expected NotFound, got {other:?}"),
        }

        // 大文字・空文字も拒否
        assert!(matches!(
            kind_str_to_volume_kind("Master").unwrap_err(),
            AppError::NotFound(_)
        ));
        assert!(matches!(
            kind_str_to_volume_kind("").unwrap_err(),
            AppError::NotFound(_)
        ));
    }
}
