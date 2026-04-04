use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use theasus_core::Result;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub session_id: Option<Uuid>,
}

pub struct BridgeConnection {
    pub config: BridgeConfig,
    pub session_id: Uuid,
    pub connected: bool,
}

impl BridgeConnection {
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            session_id: config.session_id.unwrap_or_else(Uuid::new_v4),
            config,
            connected: false,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        tracing::info!("Connecting to bridge: {}", self.config.url);
        self.connected = true;
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        tracing::info!("Disconnecting from bridge");
        self.connected = false;
        Ok(())
    }

    pub async fn send_message(&self, message: BridgeMessage) -> Result<()> {
        if !self.connected {
            return Err(theasus_core::TheasusError::Other("Not connected".to_string()));
        }
        tracing::debug!("Sending message: {:?}", message);
        Ok(())
    }

    pub async fn receive_message(&mut self) -> Result<BridgeMessage> {
        if !self.connected {
            return Err(theasus_core::TheasusError::Other("Not connected".to_string()));
        }
        Ok(BridgeMessage::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub message_type: BridgeMessageType,
    pub payload: serde_json::Value,
}

impl Default for BridgeMessage {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: Uuid::nil(),
            message_type: BridgeMessageType::Ping,
            payload: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BridgeMessageType {
    Ping,
    Pong,
    Message,
    ToolCall,
    ToolResult,
    Error,
}

pub trait BridgeTransport: Send + Sync {
    async fn connect(&mut self, url: &str) -> Result<()>;
    async fn send(&mut self, message: BridgeMessage) -> Result<()>;
    async fn receive(&mut self) -> Result<BridgeMessage>;
    async fn disconnect(&mut self) -> Result<()>;
}

pub struct BridgeManager {
    connections: std::collections::HashMap<Uuid, BridgeConnection>,
}

impl BridgeManager {
    pub fn new() -> Self {
        Self {
            connections: std::collections::HashMap::new(),
        }
    }

    pub async fn connect(&mut self, config: BridgeConfig) -> Result<Uuid> {
        let session_id = config.session_id.unwrap_or_else(Uuid::new_v4);
        let mut connection = BridgeConnection::new(config);
        connection.connect().await?;
        self.connections.insert(session_id, connection);
        Ok(session_id)
    }

    pub async fn disconnect(&mut self, session_id: Uuid) -> Result<()> {
        if let Some(mut connection) = self.connections.remove(&session_id) {
            connection.disconnect().await?;
        }
        Ok(())
    }

    pub fn get_connection(&self, session_id: Uuid) -> Option<&BridgeConnection> {
        self.connections.get(&session_id)
    }

    pub fn list_sessions(&self) -> Vec<Uuid> {
        self.connections.keys().cloned().collect()
    }
}

impl Default for BridgeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("Failed to connect: {0}")]
    ConnectionFailed(String),
    
    #[error("Not connected")]
    NotConnected,
    
    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),
    
    #[error("Send failed: {0}")]
    SendFailed(String),
    
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),
    
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
}

pub type BridgeResult<T> = std::result::Result<T, BridgeError>;
