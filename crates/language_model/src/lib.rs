use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    System(SystemMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub id: Uuid,
    pub content: Vec<ContentBlock>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub id: Uuid,
    pub content: Vec<ContentBlock>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub model: String,
    pub stop_reason: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    pub id: Uuid,
    pub content: Vec<ContentBlock>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

#[async_trait]
pub trait LanguageModel: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn max_tokens(&self) -> Option<u32>;

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<Box<dyn Stream<Item = Result<CompletionChunk>> + Send>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system: Option<String>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub message: Message,
    pub usage: Usage,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionChunk {
    pub delta: ContentBlock,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_request_builder() {
        let request = CompletionRequest {
            model: "gpt-4o".to_string(),
            messages: vec![],
            max_tokens: Some(1000),
            temperature: Some(0.7),
            system: Some("You are a helpful assistant.".to_string()),
            tools: None,
            stream: false,
        };

        assert_eq!(request.model, "gpt-4o");
        assert_eq!(request.max_tokens, Some(1000));
        assert!(!request.stream);
    }

    #[test]
    fn test_message_serialization() {
        let user_msg = Message::User(UserMessage {
            id: Uuid::new_v4(),
            content: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
            timestamp: Utc::now(),
        });

        let json = serde_json::to_string(&user_msg).expect("Serialization failed");
        let deserialized: Message =
            serde_json::from_str(&json).expect("Deserialization failed");

        if let Message::User(msg) = deserialized {
            assert_eq!(msg.content.len(), 1);
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_usage_default() {
        let usage = Usage::default();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn test_tool_definition_serialization() {
        let tool = ToolDefinition {
            name: "bash".to_string(),
            description: "Execute bash commands".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" }
                }
            }),
        };

        let json = serde_json::to_string(&tool).expect("Serialization failed");
        let deserialized: ToolDefinition =
            serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(deserialized.name, "bash");
        assert_eq!(deserialized.description, "Execute bash commands");
    }

    #[test]
    fn test_content_block_variants() {
        let text = ContentBlock::Text {
            text: "Hello".to_string(),
        };
        let tool_use = ContentBlock::ToolUse {
            id: "123".to_string(),
            name: "bash".to_string(),
            input: serde_json::json!({"command": "ls"}),
        };
        let tool_result = ContentBlock::ToolResult {
            tool_use_id: "123".to_string(),
            content: "output".to_string(),
        };

        let _ = serde_json::to_string(&text).expect("Text serialization failed");
        let _ = serde_json::to_string(&tool_use).expect("ToolUse serialization failed");
        let _ = serde_json::to_string(&tool_result).expect("ToolResult serialization failed");
    }
}
