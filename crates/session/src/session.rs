use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A persisted conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub name: Option<String>,
    pub model: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_tokens: i64,
    pub messages: Vec<SessionMessage>,
}

impl Session {
    /// Create a new session with the given model.
    pub fn new(model: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: None,
            model: model.into(),
            created_at: now,
            updated_at: now,
            total_tokens: 0,
            messages: Vec::new(),
        }
    }

    /// Create a new session with a name.
    pub fn with_name(name: impl Into<String>, model: impl Into<String>) -> Self {
        let mut session = Self::new(model);
        session.name = Some(name.into());
        session
    }

    /// Add a message to the session.
    pub fn add_message(&mut self, message: SessionMessage) {
        self.updated_at = Utc::now();
        self.messages.push(message);
    }

    /// Get message count.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get a display name (name or truncated ID).
    pub fn display_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| format!("session-{}", &self.id.to_string()[..8]))
    }
}

/// A message within a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl SessionMessage {
    /// Create a new message.
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role: role.into(),
            content: content.into(),
            tool_calls: None,
            created_at: Utc::now(),
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new("user", content)
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new("assistant", content)
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new("system", content)
    }

    /// Set tool calls JSON.
    pub fn with_tool_calls(mut self, tool_calls: impl Into<String>) -> Self {
        self.tool_calls = Some(tool_calls.into());
        self
    }
}

/// Summary of a session (without full message content).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: Uuid,
    pub name: Option<String>,
    pub model: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub total_tokens: i64,
    pub message_count: i64,
}

impl SessionSummary {
    /// Get a display name.
    pub fn display_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| format!("session-{}", &self.id.to_string()[..8]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new("gpt-4o");
        assert!(session.name.is_none());
        assert_eq!(session.model, "gpt-4o");
        assert_eq!(session.messages.len(), 0);
    }

    #[test]
    fn test_session_with_name() {
        let session = Session::with_name("test-session", "gpt-4o");
        assert_eq!(session.name, Some("test-session".to_string()));
    }

    #[test]
    fn test_message_roles() {
        let user = SessionMessage::user("hello");
        assert_eq!(user.role, "user");

        let assistant = SessionMessage::assistant("hi there");
        assert_eq!(assistant.role, "assistant");

        let system = SessionMessage::system("you are helpful");
        assert_eq!(system.role, "system");
    }

    #[test]
    fn test_display_name() {
        let mut session = Session::new("gpt-4o");
        assert!(session.display_name().starts_with("session-"));

        session.name = Some("my-chat".to_string());
        assert_eq!(session.display_name(), "my-chat");
    }
}
