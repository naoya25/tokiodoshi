use thiserror::Error;

/// アプリ全体で使うエラー型。
/// フロントへは `to_string()` の結果を文字列として渡す (`serde::Serialize` を手動実装)。
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Io: {0}")]
    Io(#[from] std::io::Error),

    #[error("Store: {0}")]
    Store(String),

    #[error("Sql: {0}")]
    Sql(String),

    /// 音再生周りのエラー。
    /// 現在 `AudioService` は失敗を silent fallback で握りつぶす方針なので
    /// production コードからは未使用。将来 rodio エラーを正面から扱うときに使うため
    /// variant 自体は残しておく。
    #[allow(dead_code)]
    #[error("Audio: {0}")]
    Audio(String),

    #[error("NotFound: {0}")]
    NotFound(String),
}

impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.to_string().as_ref())
    }
}

/// `AppError` を返す `Result` の短縮形。
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_error_serialize_as_string() {
        let e = AppError::Store("missing key".to_string());
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(json, "\"Store: missing key\"");

        let e = AppError::Sql("syntax".to_string());
        assert_eq!(serde_json::to_string(&e).unwrap(), "\"Sql: syntax\"");

        let e = AppError::Audio("device".to_string());
        assert_eq!(serde_json::to_string(&e).unwrap(), "\"Audio: device\"");

        let e = AppError::NotFound("session 42".to_string());
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            "\"NotFound: session 42\""
        );

        // `Io` は `std::io::Error` から `From` で変換できる
        let io: std::io::Error =
            std::io::Error::new(std::io::ErrorKind::NotFound, "missing.toml");
        let e: AppError = io.into();
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.starts_with("\"Io: "));
        assert!(json.contains("missing.toml"));
    }
}
