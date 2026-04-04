//! Bridge & Remote Connection System
//!
//! Provides WebSocket-based bidirectional communication for remote agent execution,
//! tool sharing, and distributed workflows.

use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use theasus_core::Result;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{
    accept_async, connect_async,
    tungstenite::Message as WsMessage,
};
use uuid::Uuid;

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub session_id: Option<Uuid>,
    pub is_server: bool,
    pub reconnect: bool,
    pub max_reconnect_attempts: u32,
    pub reconnect_delay_ms: u64,
}

impl BridgeConfig {
    pub fn client(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            api_key: None,
            session_id: None,
            is_server: false,
            reconnect: true,
            max_reconnect_attempts: 5,
            reconnect_delay_ms: 1000,
        }
    }

    pub fn server(port: u16) -> Self {
        Self {
            url: format!("ws://127.0.0.1:{}", port),
            api_key: None,
            session_id: None,
            is_server: true,
            reconnect: false,
            max_reconnect_attempts: 0,
            reconnect_delay_ms: 0,
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn with_reconnect(mut self, enabled: bool, max_attempts: u32, delay_ms: u64) -> Self {
        self.reconnect = enabled;
        self.max_reconnect_attempts = max_attempts;
        self.reconnect_delay_ms = delay_ms;
        self
    }
}

// ============================================================================
// Message Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub message_type: BridgeMessageType,
    pub payload: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
}

impl BridgeMessage {
    pub fn new(message_type: BridgeMessageType, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: Uuid::nil(),
            message_type,
            payload,
            correlation_id: None,
            auth_token: None,
        }
    }

    pub fn ping() -> Self {
        Self::new(BridgeMessageType::Ping, serde_json::json!({}))
    }

    pub fn pong(correlation_id: Uuid) -> Self {
        let mut msg = Self::new(BridgeMessageType::Pong, serde_json::json!({}));
        msg.correlation_id = Some(correlation_id);
        msg
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(
            BridgeMessageType::Error,
            serde_json::json!({ "error": message.into() }),
        )
    }

    pub fn tool_call(name: &str, input: serde_json::Value) -> Self {
        Self::new(
            BridgeMessageType::ToolCall,
            serde_json::json!({
                "name": name,
                "input": input
            }),
        )
    }

    pub fn tool_result(correlation_id: Uuid, success: bool, output: &str) -> Self {
        let mut msg = Self::new(
            BridgeMessageType::ToolResult,
            serde_json::json!({
                "success": success,
                "output": output
            }),
        );
        msg.correlation_id = Some(correlation_id);
        msg
    }

    pub fn with_session(mut self, session_id: Uuid) -> Self {
        self.session_id = session_id;
        self
    }

    pub fn with_auth(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    pub fn to_ws_message(&self) -> Result<WsMessage> {
        let json = serde_json::to_string(self)?;
        Ok(WsMessage::Text(json))
    }

    pub fn from_ws_message(msg: WsMessage) -> Result<Self> {
        match msg {
            WsMessage::Text(text) => {
                let message: BridgeMessage = serde_json::from_str(&text)?;
                Ok(message)
            }
            WsMessage::Binary(data) => {
                let message: BridgeMessage = serde_json::from_slice(&data)?;
                Ok(message)
            }
            _ => Err(theasus_core::TheasusError::Other(
                "Invalid WebSocket message type".to_string(),
            )),
        }
    }
}

impl Default for BridgeMessage {
    fn default() -> Self {
        Self::ping()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeMessageType {
    Ping,
    Pong,
    Auth,
    AuthResponse,
    Message,
    Query,
    QueryResponse,
    ToolCall,
    ToolResult,
    AgentTask,
    AgentResult,
    Error,
}

// ============================================================================
// Connection State
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

// ============================================================================
// Bridge Connection (Client)
// ============================================================================

pub struct BridgeConnection {
    pub config: BridgeConfig,
    pub session_id: Uuid,
    pub state: ConnectionState,
    tx: Option<mpsc::Sender<BridgeMessage>>,
    rx: Option<mpsc::Receiver<BridgeMessage>>,
    reconnect_attempts: u32,
}

impl BridgeConnection {
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            session_id: config.session_id.unwrap_or_else(Uuid::new_v4),
            config,
            state: ConnectionState::Disconnected,
            tx: None,
            rx: None,
            reconnect_attempts: 0,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        self.state = ConnectionState::Connecting;
        tracing::info!("Connecting to bridge: {}", self.config.url);

        let url = &self.config.url;
        let (ws_stream, _) = connect_async(url).await.map_err(|e| {
            self.state = ConnectionState::Failed;
            theasus_core::TheasusError::Other(format!("WebSocket connect failed: {}", e))
        })?;

        let (mut write, mut read) = ws_stream.split();

        // Create channels for message passing
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<BridgeMessage>(100);
        let (inbound_tx, inbound_rx) = mpsc::channel::<BridgeMessage>(100);

        self.tx = Some(outbound_tx);
        self.rx = Some(inbound_rx);

        // Spawn writer task
        tokio::spawn(async move {
            while let Some(msg) = outbound_rx.recv().await {
                if let Ok(ws_msg) = msg.to_ws_message() {
                    if write.send(ws_msg).await.is_err() {
                        break;
                    }
                }
            }
        });

        // Spawn reader task
        tokio::spawn(async move {
            while let Some(result) = read.next().await {
                match result {
                    Ok(ws_msg) => {
                        if let Ok(msg) = BridgeMessage::from_ws_message(ws_msg) {
                            if inbound_tx.send(msg).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("WebSocket read error: {}", e);
                        break;
                    }
                }
            }
        });

        self.state = ConnectionState::Connected;
        self.reconnect_attempts = 0;
        tracing::info!("Connected to bridge: {}", self.config.url);

        // Send authentication if configured
        if let Some(api_key) = &self.config.api_key {
            let auth_msg = BridgeMessage::new(
                BridgeMessageType::Auth,
                serde_json::json!({ "api_key": api_key }),
            )
            .with_session(self.session_id);
            self.send(auth_msg).await?;
        }

        Ok(())
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        if !self.config.reconnect {
            return Err(theasus_core::TheasusError::Other(
                "Reconnection disabled".to_string(),
            ));
        }

        while self.reconnect_attempts < self.config.max_reconnect_attempts {
            self.reconnect_attempts += 1;
            self.state = ConnectionState::Reconnecting;

            let delay = self.config.reconnect_delay_ms * (1 << self.reconnect_attempts.min(5));
            tracing::info!(
                "Reconnecting in {}ms (attempt {}/{})",
                delay,
                self.reconnect_attempts,
                self.config.max_reconnect_attempts
            );

            tokio::time::sleep(Duration::from_millis(delay)).await;

            match self.connect().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!("Reconnection attempt failed: {}", e);
                }
            }
        }

        self.state = ConnectionState::Failed;
        Err(theasus_core::TheasusError::Other(
            "Max reconnection attempts exceeded".to_string(),
        ))
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        tracing::info!("Disconnecting from bridge");
        self.tx = None;
        self.rx = None;
        self.state = ConnectionState::Disconnected;
        Ok(())
    }

    pub async fn send(&mut self, message: BridgeMessage) -> Result<()> {
        if self.state != ConnectionState::Connected {
            return Err(theasus_core::TheasusError::Other(
                "Not connected".to_string(),
            ));
        }

        let tx = self.tx.as_ref().ok_or_else(|| {
            theasus_core::TheasusError::Other("No sender available".to_string())
        })?;

        tx.send(message).await.map_err(|e| {
            theasus_core::TheasusError::Other(format!("Send failed: {}", e))
        })?;

        Ok(())
    }

    pub async fn receive(&mut self) -> Result<BridgeMessage> {
        if self.state != ConnectionState::Connected {
            return Err(theasus_core::TheasusError::Other(
                "Not connected".to_string(),
            ));
        }

        let rx = self.rx.as_mut().ok_or_else(|| {
            theasus_core::TheasusError::Other("No receiver available".to_string())
        })?;

        rx.recv().await.ok_or_else(|| {
            theasus_core::TheasusError::Other("Channel closed".to_string())
        })
    }

    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }
}

// ============================================================================
// Bridge Server
// ============================================================================

pub struct BridgeServer {
    port: u16,
    api_keys: Vec<String>,
    sessions: Arc<RwLock<HashMap<Uuid, ServerSession>>>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

struct ServerSession {
    #[allow(dead_code)]
    session_id: Uuid,
    authenticated: bool,
    tx: mpsc::Sender<BridgeMessage>,
}

impl BridgeServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            api_keys: vec![],
            sessions: Arc::new(RwLock::new(HashMap::new())),
            handle: None,
        }
    }

    pub fn with_api_keys(mut self, keys: Vec<String>) -> Self {
        self.api_keys = keys;
        self
    }

    pub async fn start(&mut self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await.map_err(|e| {
            theasus_core::TheasusError::Other(format!("Failed to bind: {}", e))
        })?;

        tracing::info!("Bridge server listening on {}", addr);

        let sessions = self.sessions.clone();
        let api_keys = self.api_keys.clone();

        let handle = tokio::spawn(async move {
            while let Ok((stream, addr)) = listener.accept().await {
                tracing::info!("New bridge connection from: {}", addr);

                let ws_stream = match accept_async(stream).await {
                    Ok(ws) => ws,
                    Err(e) => {
                        tracing::error!("WebSocket handshake failed: {}", e);
                        continue;
                    }
                };

                let (mut write, mut read) = ws_stream.split();
                let session_id = Uuid::new_v4();
                let (tx, mut rx) = mpsc::channel::<BridgeMessage>(100);

                let session = ServerSession {
                    session_id,
                    authenticated: api_keys.is_empty(), // Auto-auth if no keys configured
                    tx: tx.clone(),
                };

                sessions.write().await.insert(session_id, session);

                let sessions_clone = sessions.clone();
                let api_keys_clone = api_keys.clone();

                // Writer task
                tokio::spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        if let Ok(ws_msg) = msg.to_ws_message() {
                            if write.send(ws_msg).await.is_err() {
                                break;
                            }
                        }
                    }
                });

                // Reader task
                tokio::spawn(async move {
                    while let Some(result) = read.next().await {
                        match result {
                            Ok(WsMessage::Text(text)) => {
                                if let Ok(msg) = serde_json::from_str::<BridgeMessage>(&text) {
                                    Self::handle_message(
                                        &sessions_clone,
                                        &api_keys_clone,
                                        session_id,
                                        msg,
                                    )
                                    .await;
                                }
                            }
                            Ok(WsMessage::Close(_)) => {
                                tracing::info!("Client {} disconnected", session_id);
                                sessions_clone.write().await.remove(&session_id);
                                break;
                            }
                            Err(e) => {
                                tracing::error!("WebSocket error: {}", e);
                                sessions_clone.write().await.remove(&session_id);
                                break;
                            }
                            _ => {}
                        }
                    }
                });
            }
        });

        self.handle = Some(handle);
        Ok(())
    }

    async fn handle_message(
        sessions: &Arc<RwLock<HashMap<Uuid, ServerSession>>>,
        api_keys: &[String],
        session_id: Uuid,
        msg: BridgeMessage,
    ) {
        let mut sessions_guard = sessions.write().await;
        let session = match sessions_guard.get_mut(&session_id) {
            Some(s) => s,
            None => return,
        };

        match msg.message_type {
            BridgeMessageType::Auth => {
                let api_key = msg.payload.get("api_key").and_then(|v| v.as_str());
                let authenticated = api_keys.is_empty()
                    || api_key.map(|k| api_keys.contains(&k.to_string())).unwrap_or(false);

                session.authenticated = authenticated;

                let response = BridgeMessage::new(
                    BridgeMessageType::AuthResponse,
                    serde_json::json!({
                        "success": authenticated,
                        "session_id": session_id
                    }),
                );

                let _ = session.tx.send(response).await;
            }
            BridgeMessageType::Ping => {
                if session.authenticated {
                    let response = BridgeMessage::pong(msg.id);
                    let _ = session.tx.send(response).await;
                }
            }
            BridgeMessageType::ToolCall => {
                if session.authenticated {
                    // TODO: Route to tool executor
                    let response = BridgeMessage::tool_result(
                        msg.id,
                        true,
                        "Tool execution placeholder",
                    );
                    let _ = session.tx.send(response).await;
                }
            }
            _ => {
                tracing::debug!("Received message type: {:?}", msg.message_type);
            }
        }
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        self.sessions.write().await.clear();
        Ok(())
    }

    pub async fn broadcast(&self, message: BridgeMessage) -> Result<()> {
        let sessions = self.sessions.read().await;
        for session in sessions.values() {
            if session.authenticated {
                let _ = session.tx.send(message.clone()).await;
            }
        }
        Ok(())
    }

    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }
}

// ============================================================================
// Bridge Manager
// ============================================================================

pub struct BridgeManager {
    connections: HashMap<Uuid, BridgeConnection>,
    server: Option<BridgeServer>,
}

impl BridgeManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            server: None,
        }
    }

    pub async fn connect(&mut self, config: BridgeConfig) -> Result<Uuid> {
        let session_id = config.session_id.unwrap_or_else(Uuid::new_v4);
        let mut connection = BridgeConnection::new(config);
        connection.connect().await?;
        self.connections.insert(session_id, connection);
        Ok(session_id)
    }

    pub async fn start_server(&mut self, port: u16, api_keys: Vec<String>) -> Result<()> {
        let mut server = BridgeServer::new(port).with_api_keys(api_keys);
        server.start().await?;
        self.server = Some(server);
        Ok(())
    }

    pub async fn stop_server(&mut self) -> Result<()> {
        if let Some(mut server) = self.server.take() {
            server.stop().await?;
        }
        Ok(())
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

    pub fn get_connection_mut(&mut self, session_id: Uuid) -> Option<&mut BridgeConnection> {
        self.connections.get_mut(&session_id)
    }

    pub fn list_sessions(&self) -> Vec<Uuid> {
        self.connections.keys().cloned().collect()
    }

    pub async fn send(&mut self, session_id: Uuid, message: BridgeMessage) -> Result<()> {
        let connection = self.connections.get_mut(&session_id).ok_or_else(|| {
            theasus_core::TheasusError::Other(format!("Session not found: {}", session_id))
        })?;
        connection.send(message).await
    }

    pub async fn receive(&mut self, session_id: Uuid) -> Result<BridgeMessage> {
        let connection = self.connections.get_mut(&session_id).ok_or_else(|| {
            theasus_core::TheasusError::Other(format!("Session not found: {}", session_id))
        })?;
        connection.receive().await
    }
}

impl Default for BridgeManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("Failed to connect: {0}")]
    ConnectionFailed(String),

    #[error("Not connected")]
    NotConnected,

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Max reconnection attempts exceeded")]
    ReconnectionFailed,
}

pub type BridgeResult<T> = std::result::Result<T, BridgeError>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_message_serialization() {
        let msg = BridgeMessage::tool_call("bash", serde_json::json!({"command": "ls"}));
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: BridgeMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.message_type, BridgeMessageType::ToolCall);
    }

    #[test]
    fn test_bridge_config() {
        let config = BridgeConfig::client("ws://localhost:8080")
            .with_api_key("test-key")
            .with_reconnect(true, 3, 500);

        assert_eq!(config.url, "ws://localhost:8080");
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert!(config.reconnect);
        assert_eq!(config.max_reconnect_attempts, 3);
    }

    #[test]
    fn test_message_types() {
        let ping = BridgeMessage::ping();
        assert_eq!(ping.message_type, BridgeMessageType::Ping);

        let pong = BridgeMessage::pong(ping.id);
        assert_eq!(pong.correlation_id, Some(ping.id));

        let error = BridgeMessage::error("test error");
        assert_eq!(error.message_type, BridgeMessageType::Error);
    }
}
