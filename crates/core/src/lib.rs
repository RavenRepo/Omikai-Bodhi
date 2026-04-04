use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub mod error;
pub mod engine;
pub mod types;

pub use error::{Result, TheasusError};
pub use types::{Message, UserMessage, AssistantMessage, SystemMessage, ContentBlock, ToolCall, ToolResult, Usage, AppState, Config, Response};

pub use engine::QueryEngine;

#[cfg(feature = "language_model")]
pub use theasus_language_model as language_model;

pub struct Theasus {
    pub state: Arc<RwLock<AppState>>,
    pub config: Config,
}

pub async fn new_theasus(config: Config) -> Result<Theasus> {
    Ok(Theasus {
        state: Arc::new(RwLock::new(AppState::default())),
        config,
    })
}

impl Theasus {
    pub async fn query(&self, input: &str) -> Result<Response> {
        let mut state = self.state.write().await;
        state.messages.push(Message::User(UserMessage {
            id: Uuid::new_v4(),
            content: vec![ContentBlock::Text {
                text: input.to_string(),
            }],
            timestamp: Utc::now(),
        }));
        Ok(Response {
            messages: vec![],
            tool_calls: vec![],
            usage: Usage::default(),
        })
    }
}
