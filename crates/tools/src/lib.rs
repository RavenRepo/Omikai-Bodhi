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

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self {
            success: false,
            output: msg.clone(),
            error: Some(msg),
        }
    }
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

    pub fn list_tools(&self) -> Vec<ToolDefinition> {
        self.list()
    }

    pub async fn execute(&self, name: &str, input: serde_json::Value) -> Result<ToolResult> {
        let tool = self.get(name).ok_or_else(|| {
            TheasusError::Other(format!("Tool not found: {}", name))
        })?;

        let context = ToolContext {
            cwd: std::env::current_dir().unwrap_or_default(),
            session_id: uuid::Uuid::new_v4(),
            user_id: None,
        };

        tool.execute(input, &context).await
    }

    pub async fn execute_with_context(
        &self,
        name: &str,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult> {
        let tool = self.get(name).ok_or_else(|| {
            TheasusError::Other(format!("Tool not found: {}", name))
        })?;

        tool.execute(input, context).await
    }

    pub fn to_llm_tools(&self) -> Vec<theasus_language_model::ToolDefinition> {
        self.tools
            .values()
            .map(|tool| {
                let def = tool.definition();
                theasus_language_model::ToolDefinition {
                    name: def.name,
                    description: def.description,
                    input_schema: def.input_schema,
                }
            })
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_context() -> ToolContext {
        ToolContext {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            session_id: uuid::Uuid::new_v4(),
            user_id: None,
        }
    }

    #[test]
    fn test_tool_registry_creation() {
        let registry = ToolRegistry::new();
        let tools = registry.list();
        
        assert!(!tools.is_empty(), "Registry should have default tools");
        
        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"bash"), "Should have bash tool");
        assert!(tool_names.contains(&"file_read"), "Should have file_read tool");
        assert!(tool_names.contains(&"grep"), "Should have grep tool");
        assert!(tool_names.contains(&"glob"), "Should have glob tool");
    }

    #[test]
    fn test_tool_definitions_have_schemas() {
        let registry = ToolRegistry::new();
        let tools = registry.list();
        
        for tool in &tools {
            assert!(!tool.name.is_empty(), "Tool should have a name");
            assert!(!tool.description.is_empty(), "Tool {} should have description", tool.name);
            assert!(tool.input_schema.is_object(), "Tool {} should have object schema", tool.name);
        }
    }

    #[tokio::test]
    async fn test_registry_execute_method() {
        let registry = ToolRegistry::new();
        
        let result = registry.execute("glob", serde_json::json!({
            "pattern": "*.toml"
        })).await;
        
        assert!(result.is_ok(), "Registry execute should work");
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let registry = ToolRegistry::new();
        
        let result = registry.execute("nonexistent_tool", serde_json::json!({})).await;
        
        assert!(result.is_err(), "Should error for nonexistent tool");
    }

    #[test]
    fn test_tool_result_builders() {
        let success = ToolResult::success("Output text");
        assert!(success.success);
        assert_eq!(success.output, "Output text");
        assert!(success.error.is_none());

        let error = ToolResult::error("Error message");
        assert!(!error.success);
        assert!(error.error.is_some());
    }
}
