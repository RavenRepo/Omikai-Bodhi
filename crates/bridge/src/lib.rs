//! Bridge & Remote Connection System
//!
//! Provides WebSocket-based bidirectional communication for remote agent execution,
//! tool sharing, and distributed workflows.

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use theasus_core::Result;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::{
    accept_async, connect_async,
    tungstenite::Message as WsMessage,
    MaybeTlsStream, WebSocketStream,
};
use uuid::Uuid;

// ============================================================================
// Transport Trait
// ============================================================================

#[async_trait]
pub trait Transport: Send + Sync {
    async fn connect(&mut self) -> std::result::Result<(), BridgeError>;
    async fn send(&self, message: BridgeMessage) -> std::result::Result<(), BridgeError>;
    async fn receive(&mut self) -> std::result::Result<BridgeMessage, BridgeError>;
    async fn disconnect(&mut self) -> std::result::Result<(), BridgeError>;
    fn is_connected(&self) -> bool;
}

// ============================================================================
// WebSocket Transport
// ============================================================================

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
type WsSink = futures::stream::SplitSink<WsStream, WsMessage>;
type WsSource = futures::stream::SplitStream<WsStream>;

pub struct WebSocketTransport {
    url: String,
    connected: bool,
    write: Option<Arc<Mutex<WsSink>>>,
    read: Option<Arc<Mutex<WsSource>>>,
}

impl WebSocketTransport {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            connected: false,
            write: None,
            read: None,
        }
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn connect(&mut self) -> std::result::Result<(), BridgeError> {
        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| BridgeError::ConnectionFailed(e.to_string()))?;

        let (write, read) = ws_stream.split();
        self.write = Some(Arc::new(Mutex::new(write)));
        self.read = Some(Arc::new(Mutex::new(read)));
        self.connected = true;

        tracing::info!("WebSocketTransport connected to {}", self.url);
        Ok(())
    }

    async fn send(&self, message: BridgeMessage) -> std::result::Result<(), BridgeError> {
        let write = self
            .write
            .as_ref()
            .ok_or(BridgeError::NotConnected)?;

        let ws_msg = message
            .to_ws_message()
            .map_err(|e| BridgeError::SendFailed(e.to_string()))?;

        write
            .lock()
            .await
            .send(ws_msg)
            .await
            .map_err(|e| BridgeError::SendFailed(e.to_string()))?;

        Ok(())
    }

    async fn receive(&mut self) -> std::result::Result<BridgeMessage, BridgeError> {
        let read = self
            .read
            .as_ref()
            .ok_or(BridgeError::NotConnected)?;

        let msg = read
            .lock()
            .await
            .next()
            .await
            .ok_or(BridgeError::ConnectionClosed)?
            .map_err(|e| BridgeError::ReceiveFailed(e.to_string()))?;

        BridgeMessage::from_ws_message(msg)
            .map_err(|e| BridgeError::InvalidMessage(e.to_string()))
    }

    async fn disconnect(&mut self) -> std::result::Result<(), BridgeError> {
        if let Some(write) = self.write.take() {
            let _ = write.lock().await.close().await;
        }
        self.read = None;
        self.connected = false;
        tracing::info!("WebSocketTransport disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

// ============================================================================
// JWT Authentication
// ============================================================================

#[derive(Debug, Clone)]
pub struct JwtAuth {
    pub secret: String,
    pub expiry_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JwtClaims {
    session_id: Uuid,
    exp: u64,
    iat: u64,
}

impl JwtAuth {
    pub fn new(secret: impl Into<String>, expiry_secs: u64) -> Self {
        Self {
            secret: secret.into(),
            expiry_secs,
        }
    }

    pub fn generate_token(&self, session_id: Uuid) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = JwtClaims {
            session_id,
            exp: now + self.expiry_secs,
            iat: now,
        };

        let claims_json = serde_json::to_string(&claims).unwrap();
        let header = r#"{"alg":"HS256","typ":"JWT"}"#;

        let header_b64 = base64_encode(header.as_bytes());
        let claims_b64 = base64_encode(claims_json.as_bytes());

        let signature_input = format!("{}.{}", header_b64, claims_b64);
        let signature = simple_hmac(&signature_input, &self.secret);
        let signature_b64 = base64_encode(&signature);

        format!("{}.{}.{}", header_b64, claims_b64, signature_b64)
    }

    pub fn validate_token(&self, token: &str) -> std::result::Result<Uuid, BridgeError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(BridgeError::InvalidToken);
        }

        let header_b64 = parts[0];
        let claims_b64 = parts[1];
        let signature_b64 = parts[2];

        // Verify signature
        let signature_input = format!("{}.{}", header_b64, claims_b64);
        let expected_signature = simple_hmac(&signature_input, &self.secret);
        let expected_b64 = base64_encode(&expected_signature);

        if signature_b64 != expected_b64 {
            return Err(BridgeError::InvalidToken);
        }

        // Decode claims
        let claims_bytes = base64_decode(claims_b64)
            .map_err(|_| BridgeError::InvalidToken)?;
        let claims: JwtClaims = serde_json::from_slice(&claims_bytes)
            .map_err(|_| BridgeError::InvalidToken)?;

        // Check expiry
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now >= claims.exp {
            return Err(BridgeError::TokenExpired);
        }

        Ok(claims.session_id)
    }
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).map(|&b| b as u32).unwrap_or(0);
        let b2 = chunk.get(2).map(|&b| b as u32).unwrap_or(0);

        let n = (b0 << 16) | (b1 << 8) | b2;

        result.push(ALPHABET[(n >> 18) as usize & 0x3F] as char);
        result.push(ALPHABET[(n >> 12) as usize & 0x3F] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[(n >> 6) as usize & 0x3F] as char);
        }
        if chunk.len() > 2 {
            result.push(ALPHABET[n as usize & 0x3F] as char);
        }
    }

    result
}

fn base64_decode(data: &str) -> std::result::Result<Vec<u8>, ()> {
    const DECODE_TABLE: [i8; 128] = [
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1,
        52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1,
        -1,  0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14,
        15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1, -1, 63,
        -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
        41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1,
    ];

    let bytes: Vec<u8> = data.bytes().collect();
    let mut result = Vec::new();

    for chunk in bytes.chunks(4) {
        let mut n: u32 = 0;
        let mut valid_bytes = 0;

        for &b in chunk {
            if b as usize >= 128 {
                return Err(());
            }
            let val = DECODE_TABLE[b as usize];
            if val < 0 {
                continue;
            }
            n = (n << 6) | (val as u32);
            valid_bytes += 1;
        }

        // Decode bytes based on how many base64 chars we had
        // 2 chars = 1 byte, 3 chars = 2 bytes, 4 chars = 3 bytes
        match valid_bytes {
            2 => {
                result.push((n >> 4) as u8);
            }
            3 => {
                result.push((n >> 10) as u8);
                result.push((n >> 2) as u8);
            }
            4 => {
                result.push((n >> 16) as u8);
                result.push((n >> 8) as u8);
                result.push(n as u8);
            }
            _ => {}
        }
    }

    Ok(result)
}

fn simple_hmac(message: &str, key: &str) -> Vec<u8> {
    // Simple HMAC-like hash for demonstration
    // In production, use a proper HMAC implementation
    let mut hash: u64 = 0;
    for (i, b) in message.bytes().enumerate() {
        hash = hash.wrapping_add((b as u64).wrapping_mul((i as u64).wrapping_add(1)));
    }
    for (i, b) in key.bytes().enumerate() {
        hash = hash.wrapping_mul(31).wrapping_add((b as u64).wrapping_mul((i as u64).wrapping_add(1)));
    }
    hash.to_le_bytes().to_vec()
}

// ============================================================================
// Attachment Support
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub mime_type: String,
    #[serde(with = "base64_serde")]
    pub data: Vec<u8>,
}

impl Attachment {
    pub fn new(name: impl Into<String>, mime_type: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            mime_type: mime_type.into(),
            data,
        }
    }

    pub fn text(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(name, "text/plain", content.into().into_bytes())
    }

    pub fn json(name: impl Into<String>, value: &serde_json::Value) -> Self {
        Self::new(
            name,
            "application/json",
            serde_json::to_vec(value).unwrap_or_default(),
        )
    }
}

mod base64_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&super::base64_encode(data))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        super::base64_decode(&s).map_err(|_| serde::de::Error::custom("invalid base64"))
    }
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,
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
            attachments: None,
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

    pub fn with_attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments = Some(attachments);
        self
    }

    pub fn add_attachment(&mut self, attachment: Attachment) {
        self.attachments
            .get_or_insert_with(Vec::new)
            .push(attachment);
    }

    pub fn attachment(name: impl Into<String>, mime_type: impl Into<String>, data: Vec<u8>) -> Self {
        let mut msg = Self::new(
            BridgeMessageType::Attachment,
            serde_json::json!({}),
        );
        msg.attachments = Some(vec![Attachment::new(name, mime_type, data)]);
        msg
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
    Attachment,
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

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token expired")]
    TokenExpired,

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

    // ========================================================================
    // Transport Trait Tests
    // ========================================================================

    #[test]
    fn test_websocket_transport_creation() {
        let transport = WebSocketTransport::new("ws://localhost:8080");
        assert!(!transport.is_connected());
        assert_eq!(transport.url, "ws://localhost:8080");
    }

    #[test]
    fn test_websocket_transport_not_connected() {
        let transport = WebSocketTransport::new("ws://localhost:8080");
        assert!(!transport.is_connected());
    }

    // ========================================================================
    // JWT Authentication Tests
    // ========================================================================

    #[test]
    fn test_jwt_token_generation() {
        let auth = JwtAuth::new("test-secret", 3600);
        let session_id = Uuid::new_v4();
        let token = auth.generate_token(session_id);

        assert!(!token.is_empty());
        assert!(token.contains('.'));

        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_jwt_token_validation_success() {
        let auth = JwtAuth::new("test-secret", 3600);
        let session_id = Uuid::new_v4();
        let token = auth.generate_token(session_id);

        let validated = auth.validate_token(&token).unwrap();
        assert_eq!(validated, session_id);
    }

    #[test]
    fn test_jwt_token_validation_invalid_format() {
        let auth = JwtAuth::new("test-secret", 3600);

        let result = auth.validate_token("invalid-token");
        assert!(matches!(result, Err(BridgeError::InvalidToken)));

        let result = auth.validate_token("only.two");
        assert!(matches!(result, Err(BridgeError::InvalidToken)));
    }

    #[test]
    fn test_jwt_token_validation_wrong_secret() {
        let auth1 = JwtAuth::new("secret-1", 3600);
        let auth2 = JwtAuth::new("secret-2", 3600);

        let session_id = Uuid::new_v4();
        let token = auth1.generate_token(session_id);

        let result = auth2.validate_token(&token);
        assert!(matches!(result, Err(BridgeError::InvalidToken)));
    }

    #[test]
    fn test_jwt_token_expired() {
        let auth = JwtAuth::new("test-secret", 0);
        let session_id = Uuid::new_v4();
        let token = auth.generate_token(session_id);

        std::thread::sleep(std::time::Duration::from_millis(10));

        let result = auth.validate_token(&token);
        assert!(matches!(result, Err(BridgeError::TokenExpired)));
    }

    #[test]
    fn test_jwt_different_sessions() {
        let auth = JwtAuth::new("test-secret", 3600);

        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();

        let token1 = auth.generate_token(session1);
        let token2 = auth.generate_token(session2);

        assert_ne!(token1, token2);

        assert_eq!(auth.validate_token(&token1).unwrap(), session1);
        assert_eq!(auth.validate_token(&token2).unwrap(), session2);
    }

    // ========================================================================
    // Attachment Tests
    // ========================================================================

    #[test]
    fn test_attachment_creation() {
        let attachment = Attachment::new("test.txt", "text/plain", b"hello".to_vec());

        assert_eq!(attachment.name, "test.txt");
        assert_eq!(attachment.mime_type, "text/plain");
        assert_eq!(attachment.data, b"hello".to_vec());
    }

    #[test]
    fn test_attachment_text() {
        let attachment = Attachment::text("readme.txt", "Hello, World!");

        assert_eq!(attachment.name, "readme.txt");
        assert_eq!(attachment.mime_type, "text/plain");
        assert_eq!(attachment.data, b"Hello, World!".to_vec());
    }

    #[test]
    fn test_attachment_json() {
        let value = serde_json::json!({"key": "value"});
        let attachment = Attachment::json("data.json", &value);

        assert_eq!(attachment.name, "data.json");
        assert_eq!(attachment.mime_type, "application/json");
        assert!(!attachment.data.is_empty());
    }

    #[test]
    fn test_attachment_serialization() {
        let attachment = Attachment::new("test.bin", "application/octet-stream", vec![1, 2, 3, 4]);

        let json = serde_json::to_string(&attachment).unwrap();
        let parsed: Attachment = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "test.bin");
        assert_eq!(parsed.mime_type, "application/octet-stream");
        assert_eq!(parsed.data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_message_with_attachments() {
        let mut msg = BridgeMessage::new(
            BridgeMessageType::Message,
            serde_json::json!({"content": "Hello"}),
        );

        msg.add_attachment(Attachment::text("file1.txt", "Content 1"));
        msg.add_attachment(Attachment::text("file2.txt", "Content 2"));

        assert!(msg.attachments.is_some());
        assert_eq!(msg.attachments.as_ref().unwrap().len(), 2);

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: BridgeMessage = serde_json::from_str(&json).unwrap();

        assert!(parsed.attachments.is_some());
        assert_eq!(parsed.attachments.as_ref().unwrap().len(), 2);
        assert_eq!(parsed.attachments.as_ref().unwrap()[0].name, "file1.txt");
    }

    #[test]
    fn test_message_with_attachments_builder() {
        let msg = BridgeMessage::new(
            BridgeMessageType::Message,
            serde_json::json!({"content": "Hello"}),
        )
        .with_attachments(vec![
            Attachment::text("a.txt", "A"),
            Attachment::text("b.txt", "B"),
        ]);

        assert_eq!(msg.attachments.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_attachment_message_type() {
        let msg = BridgeMessage::attachment("image.png", "image/png", vec![0x89, 0x50, 0x4E, 0x47]);

        assert_eq!(msg.message_type, BridgeMessageType::Attachment);
        assert!(msg.attachments.is_some());
        assert_eq!(msg.attachments.as_ref().unwrap()[0].name, "image.png");
        assert_eq!(msg.attachments.as_ref().unwrap()[0].mime_type, "image/png");
    }

    #[test]
    fn test_message_without_attachments_serialization() {
        let msg = BridgeMessage::ping();

        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("attachments"));

        let parsed: BridgeMessage = serde_json::from_str(&json).unwrap();
        assert!(parsed.attachments.is_none());
    }

    // ========================================================================
    // Base64 Encoding/Decoding Tests
    // ========================================================================

    #[test]
    fn test_base64_encode_decode() {
        let original = b"Hello, World!";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_base64_empty() {
        let original: &[u8] = b"";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();

        assert!(decoded.is_empty());
    }

    #[test]
    fn test_base64_various_lengths() {
        for len in 1..=20 {
            let original: Vec<u8> = (0..len).map(|i| i as u8).collect();
            let encoded = base64_encode(&original);
            let decoded = base64_decode(&encoded).unwrap();
            assert_eq!(decoded, original, "Failed for length {}", len);
        }
    }
}
