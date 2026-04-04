use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::types::{ContentBlock, Message, ToolCall, Usage};
use crate::TheasusError;

#[derive(Debug, Clone)]
pub struct QueryEngineConfig {
    pub model: String,
    pub max_tokens: Option<usize>,
    pub temperature: f32,
    pub system_prompt: Option<String>,
    pub max_tool_calls: usize,
    pub max_iterations: usize,
}

impl Default for QueryEngineConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            max_tokens: Some(4096),
            temperature: 0.7,
            system_prompt: Some("You are Bodhi, an AI terminal assistant.".to_string()),
            max_tool_calls: 10,
            max_iterations: 10,
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

    pub fn add_tool_result(&mut self, tool_use_id: &str, result: &str) {
        self.messages.push(Message::User(crate::types::UserMessage {
            id: Uuid::new_v4(),
            content: vec![ContentBlock::ToolResult {
                tool_use_id: tool_use_id.to_string(),
                content: result.to_string(),
            }],
            timestamp: chrono::Utc::now(),
        }));
    }

    pub fn add_tool_call_message(&mut self, tool_calls: Vec<ToolCall>) {
        for tool_call in tool_calls {
            self.messages.push(Message::User(crate::types::UserMessage {
                id: Uuid::new_v4(),
                content: vec![ContentBlock::ToolUse {
                    tool: tool_call.clone(),
                }],
                timestamp: chrono::Utc::now(),
            }));
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub message: Option<Message>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}
