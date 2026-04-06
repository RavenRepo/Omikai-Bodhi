//! # Theasus Tools
//!
//! Tool system for the Theasus AI terminal application.
//!
//! This crate provides a registry of tools that can be executed by the AI:
//! - **bash**: Execute shell commands
//! - **file_read**: Read file contents
//! - **file_write**: Write to files
//! - **glob**: Find files by pattern
//! - **grep**: Search file contents
//!
//! ## Features
//!
//! - **Caching**: Read-only tool results are cached with TTL-based expiration
//! - **Parallel Execution**: Independent tool calls can be executed in parallel
//!
//! ## Example
//!
//! ```rust,ignore
//! use theasus_tools::{ToolRegistry, ToolContext};
//!
//! let registry = ToolRegistry::new();
//! let context = ToolContext {
//!     cwd: std::env::current_dir()?,
//!     session_id: uuid::Uuid::new_v4(),
//!     user_id: None,
//! };
//!
//! let result = registry.execute("glob", serde_json::json!({
//!     "pattern": "**/*.rs"
//! })).await?;
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use theasus_core::{Result, TheasusError};

pub mod ask_user;
pub mod bash;
pub mod cache;
pub mod config;
pub mod file_edit;
pub mod file_read;
pub mod file_write;
pub mod glob;
pub mod grep;
pub mod web_fetch;

pub use ask_user::AskUserTool;
pub use bash::BashTool;
pub use cache::{CacheConfig, CacheStats, ToolCache};
pub use config::ConfigTool;
pub use file_edit::FileEditTool;
pub use file_read::FileReadTool;
pub use file_write::FileWriteTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use web_fetch::WebFetchTool;

/// Definition of a tool including its JSON schema for inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique name of the tool
    pub name: String,
    /// Human-readable description of what the tool does
    pub description: String,
    /// JSON Schema defining the expected input format
    pub input_schema: serde_json::Value,
}

/// Trait that all tools must implement.
///
/// Tools are async and can be executed with JSON input and a context.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the unique name of this tool.
    fn name(&self) -> &str;

    /// Returns the full definition including schema.
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool with the given input and context.
    async fn execute(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult>;
}

/// Context provided to tools during execution.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Current working directory
    pub cwd: std::path::PathBuf,
    /// Unique session identifier
    pub session_id: uuid::Uuid,
    /// Optional user identifier
    pub user_id: Option<String>,
}

/// Result returned from tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the tool executed successfully
    pub success: bool,
    /// Output from the tool
    pub output: String,
    /// Error message if the tool failed
    pub error: Option<String>,
}

impl ToolResult {
    /// Create a successful result with the given output.
    pub fn success(output: impl Into<String>) -> Self {
        Self { success: true, output: output.into(), error: None }
    }

    /// Create an error result with the given message.
    pub fn error(message: impl Into<String>) -> Self {
        let msg = message.into();
        Self { success: false, output: msg.clone(), error: Some(msg) }
    }
}

/// A tool call request for parallel execution.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub input: serde_json::Value,
}

impl ToolCall {
    /// Create a new tool call.
    pub fn new(name: impl Into<String>, input: serde_json::Value) -> Self {
        Self { name: name.into(), input }
    }
}

/// Result of a tool call including the tool name.
#[derive(Debug, Clone)]
pub struct ToolCallResult {
    pub name: String,
    pub result: ToolResult,
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    cache: Arc<ToolCache>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self { tools: HashMap::new(), cache: Arc::new(ToolCache::new()) };
        registry.register_defaults();
        registry
    }

    /// Create a registry with custom cache configuration.
    pub fn with_cache_config(config: CacheConfig) -> Self {
        let mut registry =
            Self { tools: HashMap::new(), cache: Arc::new(ToolCache::with_config(config)) };
        registry.register_defaults();
        registry
    }

    pub fn register_defaults(&mut self) {
        self.register(AskUserTool::new());
        self.register(BashTool::new());
        self.register(ConfigTool::new());
        self.register(FileEditTool::new());
        self.register(FileReadTool::new());
        self.register(FileWriteTool::new());
        self.register(GrepTool::new());
        self.register(GlobTool::new());
        self.register(WebFetchTool::new());
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

    /// Get cache statistics.
    pub fn cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }

    /// Clear the tool cache.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Invalidate cache after write operations.
    pub fn invalidate_cache_on_write(&self) {
        // After file writes, invalidate file_read and glob/grep caches
        self.cache.invalidate_tool("file_read");
        self.cache.invalidate_tool("glob");
        self.cache.invalidate_tool("grep");
    }

    pub async fn execute(&self, name: &str, input: serde_json::Value) -> Result<ToolResult> {
        // Verify tool exists first
        if self.get(name).is_none() {
            return Err(TheasusError::Other(format!("Tool not found: {}", name)));
        }

        let context = ToolContext {
            cwd: std::env::current_dir().unwrap_or_default(),
            session_id: uuid::Uuid::new_v4(),
            user_id: None,
        };

        self.execute_with_cache(name, input, &context).await
    }

    pub async fn execute_with_context(
        &self,
        name: &str,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult> {
        self.execute_with_cache(name, input, context).await
    }

    /// Execute a tool with caching for read-only tools.
    async fn execute_with_cache(
        &self,
        name: &str,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult> {
        // Check cache first
        if let Some(cached) = self.cache.get(name, &input) {
            tracing::debug!("Cache hit for tool: {}", name);
            return Ok(cached);
        }

        let tool = self
            .get(name)
            .ok_or_else(|| TheasusError::Other(format!("Tool not found: {}", name)))?;

        let result = tool.execute(input.clone(), context).await?;

        // Cache successful results for cacheable tools
        if result.success {
            self.cache.put(name, &input, result.clone());
        }

        // Invalidate caches on write operations
        if name == "file_write" || name == "file_edit" {
            self.invalidate_cache_on_write();
        }

        Ok(result)
    }

    /// Execute multiple tool calls in parallel.
    ///
    /// This is useful when you have multiple independent tool calls that can
    /// be executed concurrently.
    pub async fn execute_parallel(
        &self,
        calls: Vec<ToolCall>,
        context: &ToolContext,
    ) -> Vec<ToolCallResult> {
        use futures::future::join_all;

        let futures: Vec<_> = calls
            .into_iter()
            .map(|call| {
                let cache = self.cache.clone();
                let tool = self.get(&call.name);
                let context = context.clone();
                let name = call.name.clone();
                let input = call.input.clone();

                async move {
                    // Check cache first
                    if let Some(cached) = cache.get(&name, &input) {
                        return ToolCallResult { name, result: cached };
                    }

                    let result = match tool {
                        Some(t) => match t.execute(input.clone(), &context).await {
                            Ok(r) => {
                                if r.success {
                                    cache.put(&name, &input, r.clone());
                                }
                                r
                            }
                            Err(e) => ToolResult::error(e.to_string()),
                        },
                        None => ToolResult::error(format!("Tool not found: {}", name)),
                    };

                    ToolCallResult { name, result }
                }
            })
            .collect();

        join_all(futures).await
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

    #[allow(dead_code)]
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

        let result = registry
            .execute(
                "glob",
                serde_json::json!({
                    "pattern": "*.toml"
                }),
            )
            .await;

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
