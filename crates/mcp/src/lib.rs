//! MCP (Model Context Protocol) Client
//!
//! Implements the MCP specification for connecting to external tool servers.
//! Supports stdio transport for local servers and provides a clean interface
//! for tool discovery, execution, and resource management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use theasus_core::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, RwLock};
use uuid::Uuid;

// ============================================================================
// MCP Protocol Version
// ============================================================================

pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

impl McpServerConfig {
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args: vec![],
            env: HashMap::new(),
            timeout_ms: Some(30000),
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

// ============================================================================
// JSON-RPC Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcNotification {
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}

// ============================================================================
// MCP Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

impl McpToolDefinition {
    pub fn to_tool_definition(&self) -> theasus_tools::ToolDefinition {
        theasus_tools::ToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            input_schema: self.input_schema.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<McpPromptArgument>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCallToolRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCallToolResponse {
    pub content: Vec<McpContent>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum McpContent {
    Text { text: String },
    Image { data: String, #[serde(rename = "mimeType")] mime_type: String },
    Resource { resource: McpResource },
}

impl McpContent {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<McpToolsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<McpResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<McpPromptsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpToolsCapability {
    #[serde(rename = "listChanged", default)]
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpResourcesCapability {
    #[serde(default)]
    pub subscribe: bool,
    #[serde(rename = "listChanged", default)]
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpPromptsCapability {
    #[serde(rename = "listChanged", default)]
    pub list_changed: bool,
}

// ============================================================================
// MCP Client
// ============================================================================

type PendingRequests = Arc<RwLock<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>;

pub struct McpClient {
    pub config: McpServerConfig,
    pub session_id: Uuid,
    pub server_info: Option<McpServerInfo>,
    pub capabilities: Option<McpServerCapabilities>,
    request_id: AtomicU64,
    stdin_tx: Option<mpsc::Sender<String>>,
    pending: PendingRequests,
    #[allow(dead_code)]
    child: Option<Child>,
}

impl McpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            session_id: Uuid::new_v4(),
            server_info: None,
            capabilities: None,
            request_id: AtomicU64::new(1),
            stdin_tx: None,
            pending: Arc::new(RwLock::new(HashMap::new())),
            child: None,
        }
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    pub async fn connect(&mut self) -> Result<()> {
        tracing::info!("Starting MCP server: {} ({})", self.config.name, self.config.command);

        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| {
            theasus_core::TheasusError::Other("Failed to get stdin".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            theasus_core::TheasusError::Other("Failed to get stdout".to_string())
        })?;

        // Create channel for sending to stdin
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(100);
        self.stdin_tx = Some(stdin_tx);
        self.child = Some(child);

        // Spawn stdin writer
        let mut stdin = stdin;
        tokio::spawn(async move {
            while let Some(line) = stdin_rx.recv().await {
                if stdin.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                if stdin.write_all(b"\n").await.is_err() {
                    break;
                }
                if stdin.flush().await.is_err() {
                    break;
                }
            }
        });

        // Spawn stdout reader
        let pending = self.pending.clone();
        let mut reader = BufReader::new(stdout);
        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&line) {
                            if let Some(id) = response.id {
                                if let Some(tx) = pending.write().await.remove(&id) {
                                    let _ = tx.send(response);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("MCP stdout read error: {}", e);
                        break;
                    }
                }
            }
        });

        // Initialize the connection
        self.initialize().await?;

        Ok(())
    }

    async fn send_request(&self, method: &str, params: Option<serde_json::Value>) -> Result<JsonRpcResponse> {
        let id = self.next_id();
        let request = JsonRpcRequest::new(id, method, params);
        let request_str = serde_json::to_string(&request)?;

        let (tx, rx) = oneshot::channel();
        self.pending.write().await.insert(id, tx);

        let stdin_tx = self.stdin_tx.as_ref().ok_or_else(|| {
            theasus_core::TheasusError::Other("Not connected".to_string())
        })?;

        stdin_tx.send(request_str).await.map_err(|e| {
            theasus_core::TheasusError::Other(format!("Failed to send request: {}", e))
        })?;

        let timeout = self.config.timeout_ms.unwrap_or(30000);
        let response = tokio::time::timeout(
            std::time::Duration::from_millis(timeout),
            rx,
        )
        .await
        .map_err(|_| theasus_core::TheasusError::Other("Request timeout".to_string()))?
        .map_err(|_| theasus_core::TheasusError::Other("Request cancelled".to_string()))?;

        if let Some(err) = &response.error {
            return Err(theasus_core::TheasusError::Other(format!(
                "MCP error {}: {}",
                err.code, err.message
            )));
        }

        Ok(response)
    }

    async fn send_notification(&self, method: &str, params: Option<serde_json::Value>) -> Result<()> {
        let notification = JsonRpcNotification::new(method, params);
        let notification_str = serde_json::to_string(&notification)?;

        let stdin_tx = self.stdin_tx.as_ref().ok_or_else(|| {
            theasus_core::TheasusError::Other("Not connected".to_string())
        })?;

        stdin_tx.send(notification_str).await.map_err(|e| {
            theasus_core::TheasusError::Other(format!("Failed to send notification: {}", e))
        })?;

        Ok(())
    }

    async fn initialize(&mut self) -> Result<()> {
        let params = serde_json::json!({
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities": {
                "roots": { "listChanged": true }
            },
            "clientInfo": {
                "name": "bodhi",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        let response = self.send_request("initialize", Some(params)).await?;

        if let Some(result) = response.result {
            if let Some(server_info) = result.get("serverInfo") {
                self.server_info = serde_json::from_value(server_info.clone()).ok();
            }
            if let Some(capabilities) = result.get("capabilities") {
                self.capabilities = serde_json::from_value(capabilities.clone()).ok();
            }
        }

        // Send initialized notification
        self.send_notification("notifications/initialized", None).await?;

        tracing::info!(
            "MCP server initialized: {:?}",
            self.server_info.as_ref().map(|s| &s.name)
        );

        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<McpToolDefinition>> {
        let response = self.send_request("tools/list", None).await?;

        if let Some(result) = response.result {
            if let Some(tools) = result.get("tools") {
                let tools: Vec<McpToolDefinition> = serde_json::from_value(tools.clone())?;
                return Ok(tools);
            }
        }

        Ok(vec![])
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<McpCallToolResponse> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments.unwrap_or(serde_json::json!({}))
        });

        let response = self.send_request("tools/call", Some(params)).await?;

        if let Some(result) = response.result {
            let call_response: McpCallToolResponse = serde_json::from_value(result)?;
            return Ok(call_response);
        }

        Ok(McpCallToolResponse {
            content: vec![],
            is_error: Some(true),
        })
    }

    pub async fn list_resources(&self) -> Result<Vec<McpResource>> {
        let response = self.send_request("resources/list", None).await?;

        if let Some(result) = response.result {
            if let Some(resources) = result.get("resources") {
                let resources: Vec<McpResource> = serde_json::from_value(resources.clone())?;
                return Ok(resources);
            }
        }

        Ok(vec![])
    }

    pub async fn read_resource(&self, uri: &str) -> Result<Vec<McpContent>> {
        let params = serde_json::json!({ "uri": uri });
        let response = self.send_request("resources/read", Some(params)).await?;

        if let Some(result) = response.result {
            if let Some(contents) = result.get("contents") {
                let contents: Vec<McpContent> = serde_json::from_value(contents.clone())?;
                return Ok(contents);
            }
        }

        Ok(vec![])
    }

    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>> {
        let response = self.send_request("prompts/list", None).await?;

        if let Some(result) = response.result {
            if let Some(prompts) = result.get("prompts") {
                let prompts: Vec<McpPrompt> = serde_json::from_value(prompts.clone())?;
                return Ok(prompts);
            }
        }

        Ok(vec![])
    }

    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, String>>,
    ) -> Result<Vec<McpContent>> {
        let params = serde_json::json!({
            "name": name,
            "arguments": arguments.unwrap_or_default()
        });

        let response = self.send_request("prompts/get", Some(params)).await?;

        if let Some(result) = response.result {
            if let Some(messages) = result.get("messages") {
                // Extract content from messages
                if let Some(messages) = messages.as_array() {
                    let mut contents = vec![];
                    for msg in messages {
                        if let Some(content) = msg.get("content") {
                            if let Some(text) = content.get("text").and_then(|t| t.as_str()) {
                                contents.push(McpContent::text(text));
                            }
                        }
                    }
                    return Ok(contents);
                }
            }
        }

        Ok(vec![])
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        tracing::info!("Disconnecting from MCP server: {}", self.config.name);
        self.stdin_tx = None;
        self.pending.write().await.clear();
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.stdin_tx.is_some()
    }

    pub fn has_tools(&self) -> bool {
        self.capabilities
            .as_ref()
            .and_then(|c| c.tools.as_ref())
            .is_some()
    }

    pub fn has_resources(&self) -> bool {
        self.capabilities
            .as_ref()
            .and_then(|c| c.resources.as_ref())
            .is_some()
    }

    pub fn has_prompts(&self) -> bool {
        self.capabilities
            .as_ref()
            .and_then(|c| c.prompts.as_ref())
            .is_some()
    }
}

// ============================================================================
// MCP Client Manager
// ============================================================================

pub struct McpClientManager {
    clients: HashMap<String, McpClient>,
}

impl McpClientManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub async fn add_server(&mut self, config: McpServerConfig) -> Result<()> {
        let name = config.name.clone();
        let mut client = McpClient::new(config);
        client.connect().await?;
        self.clients.insert(name, client);
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

    pub async fn list_all_tools(&self) -> Result<Vec<(String, McpToolDefinition)>> {
        let mut all_tools = vec![];
        for (server_name, client) in &self.clients {
            if let Ok(tools) = client.list_tools().await {
                for tool in tools {
                    all_tools.push((server_name.clone(), tool));
                }
            }
        }
        Ok(all_tools)
    }

    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<McpCallToolResponse> {
        let client = self.clients.get(server_name).ok_or_else(|| {
            theasus_core::TheasusError::Other(format!("Server not found: {}", server_name))
        })?;
        client.call_tool(tool_name, arguments).await
    }

    pub async fn remove_server(&mut self, name: &str) -> Result<()> {
        if let Some(mut client) = self.clients.remove(name) {
            client.disconnect().await?;
        }
        Ok(())
    }

    pub async fn disconnect_all(&mut self) -> Result<()> {
        for client in self.clients.values_mut() {
            client.disconnect().await?;
        }
        self.clients.clear();
        Ok(())
    }
}

impl Default for McpClientManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Errors
// ============================================================================

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

    #[error("Prompt not found: {0}")]
    PromptNotFound(String),

    #[error("Invalid response from MCP server")]
    InvalidResponse,

    #[error("Request timeout")]
    Timeout,

    #[error("Protocol version mismatch")]
    ProtocolMismatch,
}

pub type McpResult<T> = std::result::Result<T, McpError>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_config() {
        let config = McpServerConfig::new("test", "echo")
            .with_args(vec!["hello".to_string()])
            .with_env("FOO", "bar");

        assert_eq!(config.name, "test");
        assert_eq!(config.command, "echo");
        assert_eq!(config.args, vec!["hello"]);
        assert_eq!(config.env.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_jsonrpc_request() {
        let request = JsonRpcRequest::new(1, "tools/list", None);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"tools/list\""));
    }

    #[test]
    fn test_mcp_content() {
        let content = McpContent::text("Hello, world!");
        assert_eq!(content.as_text(), Some("Hello, world!"));

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"text\""));
    }

    #[test]
    fn test_tool_definition_conversion() {
        let mcp_tool = McpToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        };

        let tool_def = mcp_tool.to_tool_definition();
        assert_eq!(tool_def.name, "test_tool");
        assert_eq!(tool_def.description, "A test tool");
    }
}
