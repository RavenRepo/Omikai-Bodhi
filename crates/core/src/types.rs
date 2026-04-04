use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    System(SystemMessage),
    Progress(ProgressMessage),
    Attachment(AttachmentMessage),
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
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMessage {
    pub message: String,
    pub progress: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentMessage {
    pub id: Uuid,
    pub file_name: String,
    pub mime_type: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Image {
        url: String,
        detail: String,
    },
    ToolUse {
        tool: ToolCall,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_use_id: String,
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub messages: Vec<Message>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub messages: Vec<Message>,
    pub session_id: Uuid,
    pub cwd: std::path::PathBuf,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            session_id: Uuid::new_v4(),
            cwd: std::env::current_dir().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: String,
    pub max_budget_usd: Option<f64>,
    pub max_tokens: Option<u32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            api_key: None,
            base_url: "https://api.openai.com".to_string(),
            max_budget_usd: Some(10.0),
            max_tokens: Some(4096),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionMode {
    Default,
    AcceptEdits,
    BypassPermissions,
    DontAsk,
    Plan,
    Auto,
    Bubble,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionResult {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionContext {
    pub session_id: Uuid,
    pub cwd: std::path::PathBuf,
    pub mode: PermissionMode,
}
