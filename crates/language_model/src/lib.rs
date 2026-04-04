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
