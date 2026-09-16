use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("keychain error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("checksum mismatch for {file}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },
    #[error("launch error: {0}")]
    Launch(String),
    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Other(e.to_string())
    }
}

// Tauri commands must return errors that serialize to the frontend as strings.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
