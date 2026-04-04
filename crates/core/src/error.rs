use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TheasusError {
    #[error("API error: {message} (status: {status})")]
    Api { status: u16, message: String },

    #[error("Rate limited, retry after {retry_after}s")]
    RateLimited { retry_after: u64 },

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Budget exceeded: ${spent:.2} of ${limit:.2}")]
    BudgetExceeded { spent: f64, limit: f64 },

    #[error("Tool '{tool}' failed: {reason}")]
    Tool { tool: String, reason: String },

    #[error("Tool '{tool}' not found")]
    ToolNotFound { tool: String },

    #[error("Permission denied for '{tool}': {reason}")]
    Permission { tool: String, reason: String },

    #[error("Permission mode '{mode}' not recognized")]
    InvalidPermissionMode { mode: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid configuration: {field} = {value}")]
    InvalidConfig { field: String, value: String },

    #[error("Session '{id}' not found")]
    SessionNotFound { id: String },

    #[error("Failed to save session: {0}")]
    SessionSaveFailed(String),

    #[error("Command '{command}' not found")]
    CommandNotFound { command: String },

    #[error("Invalid command arguments for '{command}': {reason}")]
    InvalidArguments { command: String, reason: String },

    #[error("Agent '{agent}' not found")]
    AgentNotFound { agent: String },

    #[error("MCP server '{server}' connection failed: {reason}")]
    McpConnectionFailed { server: String, reason: String },

    #[error("Invalid path: {path}")]
    InvalidPath { path: PathBuf },

    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("File already exists: {path}")]
    FileExists { path: PathBuf },

    #[error("Security violation: {reason}")]
    SecurityViolation { reason: String },

    #[error("LLM provider '{provider}' not supported")]
    UnsupportedProvider { provider: String },

    #[error("Streaming error: {0}")]
    StreamError(String),

    #[error("Timeout after {timeout}s")]
    Timeout { timeout: u64 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, TheasusError>;

pub trait ContextExt<T, E> {
    fn context<C: Into<String>>(self, context: C) -> Result<T>;
    fn with_context<C: Into<String>, F: FnOnce() -> C>(self, context: F) -> Result<T>;
}

impl<T, E> ContextExt<T, E> for std::result::Result<T, E>
where
    E: std::fmt::Debug,
{
    fn context<C: Into<String>>(self, context: C) -> Result<T> {
        self.map_err(|e| TheasusError::Other(format!("{:?}: {}", e, context.into())))
    }

    fn with_context<C: Into<String>, F: FnOnce() -> C>(self, context: F) -> Result<T> {
        self.map_err(|e| TheasusError::Other(format!("{:?}: {}", e, context().into())))
    }
}
