use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use theasus_core::Result;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCallToolRequest {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCallToolResponse {
    pub content: Vec<McpContent>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { resource: McpResource },
}

pub struct McpClient {
    pub server_config: McpServerConfig,
    pub session_id: Uuid,
}

impl McpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            server_config: config,
            session_id: Uuid::new_v4(),
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        tracing::info!("Connecting to MCP server: {}", self.server_config.name);
        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<McpToolDefinition>> {
        tracing::info!("Listing tools from MCP server: {}", self.server_config.name);
        Ok(vec![])
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpCallToolResponse> {
        tracing::info!(
            "Calling MCP tool: {} on server: {}",
            name,
            self.server_config.name
        );
        Ok(McpCallToolResponse {
            content: vec![],
            is_error: None,
        })
    }

    pub async fn list_resources(&self) -> Result<Vec<McpResource>> {
        tracing::info!(
            "Listing resources from MCP server: {}",
            self.server_config.name
        );
        Ok(vec![])
    }

    pub async fn read_resource(&self, uri: &str) -> Result<String> {
        tracing::info!(
            "Reading resource: {} from MCP server: {}",
            uri,
            self.server_config.name
        );
        Ok(String::new())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        tracing::info!("Disconnecting from MCP server: {}", self.server_config.name);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Failed to connect to MCP server: {0}")]
    ConnectionFailed(String),

    #[error("MCP server error: {0}")]
    ServerError(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Invalid response from MCP server")]
    InvalidResponse,
}

pub type McpResult<T> = std::result::Result<T, McpError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
}

pub struct McpClientManager {
    clients: std::collections::HashMap<String, McpClient>,
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            clients: std::collections::HashMap::new(),
        }
    }

    pub async fn add_server(&mut self, config: McpServerConfig) -> Result<()> {
        let mut client = McpClient::new(config.clone());
        client.connect().await?;
        self.clients.insert(config.name, client);
        Ok(())
    }

    pub fn get_client(&self, name: &str) -> Option<&McpClient> {
        self.clients.get(name)
    }

    pub fn get_client_mut(&mut self, name: &str) -> Option<&mut McpClient> {
        self.clients.get_mut(name)
    }

    pub fn list_servers(&self) -> Vec<String> {
        self.clients.keys().cloned().collect()
    }

    pub async fn remove_server(&mut self, name: &str) -> Result<()> {
        if let Some(mut client) = self.clients.remove(name) {
            client.disconnect().await?;
        }
        Ok(())
    }
}

impl Default for McpClientManager {
    fn default() -> Self {
        Self::new()
    }
}
