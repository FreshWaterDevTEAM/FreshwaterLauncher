use thiserror::Error;

pub type Result<T> = std::result::Result<T, FwlError>;

#[derive(Debug, Error)]
pub enum FwlError {
    #[error("{0}")]
    Message(String),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Auth error: {0}")]
    Auth(String),
    #[error("Download error: {0}")]
    Download(String),
    #[error("Launch error: {0}")]
    Launch(String),
    #[error("Sync error: {0}")]
    Sync(String),
    #[error("Store error: {0}")]
    Store(String),
}

impl FwlError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Message(s.into())
    }
}
