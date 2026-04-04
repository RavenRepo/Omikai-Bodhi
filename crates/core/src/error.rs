use thiserror::Error;

#[derive(Error, Debug)]
pub enum TheasusError {
    #[error("API error: {0}")]
    Api(String),

    #[error("Tool '{tool}' failed: {reason}")]
    Tool { tool: String, reason: String },

    #[error("Permission denied for '{tool}': {reason}")]
    Permission { tool: String, reason: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, TheasusError>;
