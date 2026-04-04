use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use theasus_core::{Result, TheasusError};

pub mod bash;
pub mod file_read;
pub mod file_write;
pub mod glob;
pub mod grep;

pub use bash::BashTool;
pub use file_read::FileReadTool;
pub use file_write::FileWriteTool;
pub use glob::GlobTool;
pub use grep::GrepTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn definition(&self) -> ToolDefinition;

    async fn execute(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult>;
}

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub cwd: std::path::PathBuf,
    pub session_id: uuid::Uuid,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    pub fn register_defaults(&mut self) {
        self.register(BashTool::new());
        self.register(FileReadTool::new());
        self.register(FileWriteTool::new());
        self.register(GrepTool::new());
        self.register(GlobTool::new());
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn list(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|tool| tool.definition()).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {name}")]
    NotFound { name: String },

    #[error("Tool execution failed: {message}")]
    ExecutionFailed { message: String },

    #[error("Invalid input for tool {name}: {message}")]
    InvalidInput { name: String, message: String },

    #[error("Permission denied for tool {name}: {reason}")]
    PermissionDenied { name: String, reason: String },
}
