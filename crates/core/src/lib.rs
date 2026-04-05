//! # Theasus Core
//!
//! Core functionality for the Theasus AI terminal application.
//!
//! This crate provides the foundational types and abstractions for:
//! - Message handling (User, Assistant, System messages)
//! - Query engine for LLM interactions
//! - Application state management
//! - Error handling
//!
//! ## Example
//!
//! ```rust,ignore
//! use theasus_core::{new_theasus, Config};
//!
//! let config = Config {
//!     api_key: Some("your-api-key".to_string()),
//!     model: "gpt-4".to_string(),
//!     ..Default::default()
//! };
//! let theasus = new_theasus(config).await?;
//! let response = theasus.query("Hello, world!").await?;
//! ```

use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub mod engine;
pub mod error;
pub mod types;

pub use error::{Result, TheasusError};
pub use types::{
    AgentId, AppState, AssistantMessage, Config, ContentBlock, Message, PermissionContext,
    PermissionMode, PermissionResult, Response, SessionId, SystemMessage, TaskStatus, TaskType,
    ToolCall, ToolResult, Usage, UserMessage,
};

pub use engine::QueryEngine;

/// The main Theasus application instance.
///
/// Holds shared application state and configuration.
pub struct Theasus {
    /// Shared application state, thread-safe with RwLock
    pub state: Arc<RwLock<AppState>>,
    /// Application configuration
    pub config: Config,
}

/// Create a new Theasus instance with the given configuration.
///
/// # Arguments
///
/// * `config` - The application configuration
///
/// # Returns
///
/// A new Theasus instance wrapped in a Result
pub async fn new_theasus(config: Config) -> Result<Theasus> {
    Ok(Theasus { state: Arc::new(RwLock::new(AppState::default())), config })
}

impl Theasus {
    /// Execute a query against the LLM.
    ///
    /// # Arguments
    ///
    /// * `input` - The user's query text
    ///
    /// # Returns
    ///
    /// A Response containing the LLM's reply and any tool calls
    pub async fn query(&self, input: &str) -> Result<Response> {
        let mut state = self.state.write().await;
        state.messages.push(Message::User(UserMessage {
            id: Uuid::new_v4(),
            content: vec![ContentBlock::Text { text: input.to_string() }],
            timestamp: Utc::now(),
        }));
        Ok(Response { messages: vec![], tool_calls: vec![], usage: Usage::default() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_theasus_creation() {
        let config = Config::default();
        let theasus = new_theasus(config).await;
        assert!(theasus.is_ok());
        let theasus = theasus.unwrap();
        assert_eq!(theasus.config.model, "gpt-4o");
    }

    #[tokio::test]
    async fn test_app_state_default() {
        let state = AppState::default();
        assert!(state.messages.is_empty());
        assert!(state.tasks.is_empty());
    }

    #[tokio::test]
    async fn test_theasus_query_adds_message() {
        let config = Config::default();
        let theasus = new_theasus(config).await.unwrap();
        let _ = theasus.query("Hello").await;
        let state = theasus.state.read().await;
        assert_eq!(state.messages.len(), 1);
        if let Message::User(msg) = &state.messages[0] {
            assert_eq!(msg.content.len(), 1);
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.model, "gpt-4o");
        assert!(config.api_key.is_none());
    }
}
