use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::{ContentBlock, Message, ToolCall, Usage};

#[derive(Debug, Clone)]
pub struct QueryEngineConfig {
    pub model: String,
    pub max_tokens: Option<usize>,
    pub temperature: f32,
    pub system_prompt: Option<String>,
}

impl Default for QueryEngineConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            max_tokens: Some(4096),
            temperature: 0.7,
            system_prompt: Some("You are Bodhi, an AI terminal assistant.".to_string()),
        }
    }
}

pub struct QueryEngine {
    pub config: QueryEngineConfig,
    pub messages: Vec<Message>,
    pub session_id: Uuid,
    pub total_usage: Usage,
}

impl QueryEngine {
    pub fn new(config: QueryEngineConfig) -> Self {
        Self {
            config,
            messages: Vec::new(),
            session_id: Uuid::new_v4(),
            total_usage: Usage::default(),
        }
    }

    pub fn add_user_message(&mut self, content: &str) {
        self.messages.push(Message::User(crate::types::UserMessage {
            id: Uuid::new_v4(),
            content: vec![ContentBlock::Text {
                text: content.to_string(),
            }],
            timestamp: chrono::Utc::now(),
        }));
    }

    pub fn add_assistant_message(&mut self, content: &str) {
        self.messages
            .push(Message::Assistant(crate::types::AssistantMessage {
                id: Uuid::new_v4(),
                content: vec![ContentBlock::Text {
                    text: content.to_string(),
                }],
                tool_calls: vec![],
                usage: Usage::default(),
                model: self.config.model.clone(),
                stop_reason: Some("end_turn".to_string()),
                timestamp: chrono::Utc::now(),
            }));
    }

    pub fn compact_conversation(&mut self) {
        if self.messages.len() > 20 {
            let system_messages: Vec<Message> = self
                .messages
                .iter()
                .filter(|m| matches!(m, Message::System(_)))
                .cloned()
                .collect();

            let last_messages: Vec<Message> =
                self.messages.iter().rev().take(10).cloned().collect();

            self.messages = system_messages;
            self.messages.extend(last_messages.into_iter().rev());
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn clear_messages(&mut self) {
        self.messages.clear();
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn get_messages(&self) -> &Vec<Message> {
        &self.messages
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub message: Option<Message>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}
