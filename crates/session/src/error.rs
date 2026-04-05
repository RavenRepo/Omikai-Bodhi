use thiserror::Error;

/// Session-related errors.
#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid session ID: {0}")]
    InvalidId(String),
}

pub type Result<T> = std::result::Result<T, SessionError>;
