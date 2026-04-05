//! MCP tool for calling tools on MCP servers.
//!
//! Connects to configured MCP servers and proxies tool calls to them.

use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use theasus_core::Result;
use theasus_mcp::{McpClientManager, McpServerConfig};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInput {
    /// Name of the MCP server to connect to
    pub server_name: String,
    /// Name of the tool to call on the server
    pub tool_name: String,
    /// Arguments to pass to the tool
    #[serde(default)]
    pub arguments: serde_json::Value,
}

pub struct McpTool {
    manager: Arc<RwLock<McpClientManager>>,
}

impl McpTool {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(McpClientManager::new())),
        }
    }

    pub fn with_manager(manager: Arc<RwLock<McpClientManager>>) -> Self {
        Self { manager }
    }

    pub async fn add_server(&self, config: McpServerConfig) -> Result<()> {
        self.manager.write().await.add_server(config).await
    }

    pub async fn remove_server(&self, name: &str) -> Result<()> {
        self.manager.write().await.remove_server(name).await
    }

    pub async fn list_servers(&self) -> Vec<String> {
        self.manager.read().await.list_servers()
    }

    pub async fn list_tools(&self) -> Result<Vec<(String, theasus_mcp::McpToolDefinition)>> {
        self.manager.read().await.list_all_tools().await
    }
}

impl Default for McpTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        "mcp"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "mcp".to_string(),
            description: "Call a tool on an MCP (Model Context Protocol) server. Use this to interact with external tool providers.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "server_name": {
                        "type": "string",
                        "description": "Name of the MCP server to connect to"
                    },
                    "tool_name": {
                        "type": "string",
                        "description": "Name of the tool to call on the server"
                    },
                    "arguments": {
                        "type": "object",
                        "description": "Arguments to pass to the tool",
                        "default": {}
                    }
                },
                "required": ["server_name", "tool_name"]
            }),
        }
    }

    async fn execute(&self, input: serde_json::Value, _context: &ToolContext) -> Result<ToolResult> {
        let mcp_input: McpInput =
            serde_json::from_value(input).map_err(|e| theasus_core::TheasusError::Tool {
                tool: "mcp".to_string(),
                reason: format!("Invalid input: {}", e),
            })?;

        let manager = self.manager.read().await;
        let servers = manager.list_servers();

        if servers.is_empty() {
            return Ok(ToolResult::error(
                "No MCP servers configured. Add a server first using the MCP configuration.",
            ));
        }

        if !servers.contains(&mcp_input.server_name) {
            return Ok(ToolResult::error(format!(
                "MCP server '{}' not found. Available servers: {:?}",
                mcp_input.server_name, servers
            )));
        }

        let arguments = if mcp_input.arguments.is_null() {
            None
        } else {
            Some(mcp_input.arguments)
        };

        match manager
            .call_tool(&mcp_input.server_name, &mcp_input.tool_name, arguments)
            .await
        {
            Ok(response) => {
                if response.is_error.unwrap_or(false) {
                    let error_text = response
                        .content
                        .iter()
                        .filter_map(|c| c.as_text())
                        .collect::<Vec<_>>()
                        .join("\n");
                    Ok(ToolResult::error(format!(
                        "MCP tool error: {}",
                        error_text
                    )))
                } else {
                    let output = response
                        .content
                        .iter()
                        .filter_map(|c| c.as_text())
                        .collect::<Vec<_>>()
                        .join("\n");
                    Ok(ToolResult::success(output))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("MCP call failed: {}", e))),
        }
    }
}
