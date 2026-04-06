use std::path::PathBuf;

use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use rusqlite::Connection;
use tracing::{debug, info};
use uuid::Uuid;

use crate::error::{Result, SessionError};
use crate::schema::{init_schema, migrate};
use crate::session::{Session, SessionMessage, SessionSummary};

/// SQLite-based session storage.
pub struct SessionStore {
    conn: Connection,
}

impl SessionStore {
    /// Open a session store at the given path.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        debug!("Opening session store at {:?}", path);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;

        init_schema(&conn)?;
        migrate(&conn)?;

        info!("Session store opened at {:?}", path);
        Ok(Self { conn })
    }

    /// Open the default session store location.
    pub fn open_default() -> Result<Self> {
        let path = Self::default_path().ok_or_else(|| {
            SessionError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine config directory",
            ))
        })?;
        Self::open(path)
    }

    /// Open an in-memory session store (for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        init_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Get the default store path.
    pub fn default_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "omikai", "bodhi").map(|dirs| dirs.data_dir().join("sessions.db"))
    }

    /// Create a new session.
    pub fn create_session(&self, name: Option<&str>, model: &str) -> Result<Session> {
        let session =
            if let Some(n) = name { Session::with_name(n, model) } else { Session::new(model) };

        self.conn.execute(
            "INSERT INTO sessions (id, name, model, created_at, updated_at, total_tokens)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                session.id.to_string(),
                &session.name,
                &session.model,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
                session.total_tokens,
            ),
        )?;

        info!("Created session: {}", session.display_name());
        Ok(session)
    }

    /// Save a session (update metadata and add new messages).
    pub fn save_session(&self, session: &Session) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET name = ?2, updated_at = ?3, total_tokens = ?4 WHERE id = ?1",
            (
                session.id.to_string(),
                &session.name,
                session.updated_at.to_rfc3339(),
                session.total_tokens,
            ),
        )?;

        // Insert messages that don't exist yet
        for msg in &session.messages {
            let exists: bool = self.conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM messages WHERE id = ?1)",
                [msg.id.to_string()],
                |row| row.get(0),
            )?;

            if !exists {
                self.conn.execute(
                    "INSERT INTO messages (id, session_id, role, content, tool_calls, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    (
                        msg.id.to_string(),
                        session.id.to_string(),
                        &msg.role,
                        &msg.content,
                        &msg.tool_calls,
                        msg.created_at.to_rfc3339(),
                    ),
                )?;
            }
        }

        debug!("Saved session: {}", session.display_name());
        Ok(())
    }

    /// Load a session by ID.
    pub fn load_session(&self, id: Uuid) -> Result<Session> {
        let id_str = id.to_string();

        let (name, model, created_at, updated_at, total_tokens): (
            Option<String>,
            String,
            String,
            String,
            i64,
        ) = self
            .conn
            .query_row(
                "SELECT name, model, created_at, updated_at, total_tokens
                 FROM sessions WHERE id = ?1",
                [&id_str],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .map_err(|_| SessionError::NotFound(id_str.clone()))?;

        let messages = self.load_messages(id)?;

        Ok(Session {
            id,
            name,
            model,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            total_tokens,
            messages,
        })
    }

    /// Load messages for a session.
    fn load_messages(&self, session_id: Uuid) -> Result<Vec<SessionMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, role, content, tool_calls, created_at
             FROM messages WHERE session_id = ?1 ORDER BY created_at ASC",
        )?;

        let messages = stmt
            .query_map([session_id.to_string()], |row| {
                let id_str: String = row.get(0)?;
                let role: String = row.get(1)?;
                let content: String = row.get(2)?;
                let tool_calls: Option<String> = row.get(3)?;
                let created_at_str: String = row.get(4)?;

                Ok(SessionMessage {
                    id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()),
                    role,
                    content,
                    tool_calls,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(messages)
    }

    /// Add a message to a session.
    pub fn add_message(&self, session_id: Uuid, message: &SessionMessage) -> Result<()> {
        self.conn.execute(
            "INSERT INTO messages (id, session_id, role, content, tool_calls, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                message.id.to_string(),
                session_id.to_string(),
                &message.role,
                &message.content,
                &message.tool_calls,
                message.created_at.to_rfc3339(),
            ),
        )?;

        // Update session timestamp
        self.conn.execute(
            "UPDATE sessions SET updated_at = ?2 WHERE id = ?1",
            (session_id.to_string(), Utc::now().to_rfc3339()),
        )?;

        Ok(())
    }

    /// List all sessions (summaries only).
    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.name, s.model, s.created_at, s.updated_at, s.total_tokens,
                    (SELECT COUNT(*) FROM messages m WHERE m.session_id = s.id) as message_count
             FROM sessions s ORDER BY s.updated_at DESC",
        )?;

        let summaries = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let name: Option<String> = row.get(1)?;
                let model: String = row.get(2)?;
                let created_at_str: String = row.get(3)?;
                let updated_at_str: String = row.get(4)?;
                let total_tokens: i64 = row.get(5)?;
                let message_count: i64 = row.get(6)?;

                Ok(SessionSummary {
                    id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::new_v4()),
                    name,
                    model,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    total_tokens,
                    message_count,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(summaries)
    }

    /// Delete a session and all its messages.
    pub fn delete_session(&self, id: Uuid) -> Result<()> {
        let rows = self.conn.execute("DELETE FROM sessions WHERE id = ?1", [id.to_string()])?;

        if rows == 0 {
            return Err(SessionError::NotFound(id.to_string()));
        }

        info!("Deleted session: {}", id);
        Ok(())
    }

    /// Delete sessions older than the given duration.
    pub fn cleanup_old_sessions(&self, older_than_days: i64) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(older_than_days);

        let rows = self
            .conn
            .execute("DELETE FROM sessions WHERE updated_at < ?1", [cutoff.to_rfc3339()])?;

        if rows > 0 {
            info!("Cleaned up {} old sessions", rows);
        }

        Ok(rows)
    }

    /// Update token count for a session.
    pub fn update_tokens(&self, session_id: Uuid, tokens: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET total_tokens = total_tokens + ?2, updated_at = ?3 WHERE id = ?1",
            (session_id.to_string(), tokens, Utc::now().to_rfc3339()),
        )?;
        Ok(())
    }

    /// Rename a session.
    pub fn rename_session(&self, id: Uuid, name: &str) -> Result<()> {
        let rows = self.conn.execute(
            "UPDATE sessions SET name = ?2, updated_at = ?3 WHERE id = ?1",
            (id.to_string(), name, Utc::now().to_rfc3339()),
        )?;

        if rows == 0 {
            return Err(SessionError::NotFound(id.to_string()));
        }

        Ok(())
    }

    /// Get session count.
    pub fn session_count(&self) -> Result<i64> {
        let count: i64 =
            self.conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_load_session() {
        let store = SessionStore::open_in_memory().unwrap();
        let session = store.create_session(Some("test"), "gpt-4o").unwrap();

        let loaded = store.load_session(session.id).unwrap();
        assert_eq!(loaded.name, Some("test".to_string()));
        assert_eq!(loaded.model, "gpt-4o");
    }

    #[test]
    fn test_add_messages() {
        let store = SessionStore::open_in_memory().unwrap();
        let session = store.create_session(None, "gpt-4o").unwrap();

        let msg1 = SessionMessage::user("Hello");
        let msg2 = SessionMessage::assistant("Hi there!");

        store.add_message(session.id, &msg1).unwrap();
        store.add_message(session.id, &msg2).unwrap();

        let loaded = store.load_session(session.id).unwrap();
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(loaded.messages[0].role, "user");
        assert_eq!(loaded.messages[1].role, "assistant");
    }

    #[test]
    fn test_list_sessions() {
        let store = SessionStore::open_in_memory().unwrap();
        store.create_session(Some("session1"), "gpt-4o").unwrap();
        store.create_session(Some("session2"), "claude-3").unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_delete_session() {
        let store = SessionStore::open_in_memory().unwrap();
        let session = store.create_session(None, "gpt-4o").unwrap();

        store.delete_session(session.id).unwrap();

        let result = store.load_session(session.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_session() {
        let store = SessionStore::open_in_memory().unwrap();
        let session = store.create_session(None, "gpt-4o").unwrap();

        store.rename_session(session.id, "new-name").unwrap();

        let loaded = store.load_session(session.id).unwrap();
        assert_eq!(loaded.name, Some("new-name".to_string()));
    }

    #[test]
    fn test_session_count() {
        let store = SessionStore::open_in_memory().unwrap();
        assert_eq!(store.session_count().unwrap(), 0);

        store.create_session(None, "gpt-4o").unwrap();
        store.create_session(None, "gpt-4o").unwrap();

        assert_eq!(store.session_count().unwrap(), 2);
    }
}
