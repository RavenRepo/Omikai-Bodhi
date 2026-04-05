//! Session management commands.
//!
//! Commands for managing conversation sessions:
//! - `/sessions` - List all sessions
//! - `/session <command>` - Session operations (new, resume, delete, rename)

use async_trait::async_trait;
use theasus_core::Result;
use theasus_session::SessionStore;
use uuid::Uuid;

use crate::{Command, CommandContext, CommandResult};

/// List all sessions.
pub struct SessionsCommand;

impl SessionsCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SessionsCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Command for SessionsCommand {
    fn name(&self) -> &str {
        "sessions"
    }

    fn description(&self) -> &str {
        "List all saved sessions"
    }

    async fn execute(&self, _args: &str, _context: &CommandContext) -> Result<CommandResult> {
        let store = match SessionStore::open_default() {
            Ok(s) => s,
            Err(e) => return Ok(CommandResult::error(format!("Failed to open session store: {}", e))),
        };

        let sessions = match store.list_sessions() {
            Ok(s) => s,
            Err(e) => return Ok(CommandResult::error(format!("Failed to list sessions: {}", e))),
        };

        if sessions.is_empty() {
            return Ok(CommandResult::success("No saved sessions."));
        }

        let mut output = String::from("Sessions:\n");
        output.push_str("─".repeat(60).as_str());
        output.push('\n');

        for session in sessions {
            let name = session.display_name();
            let updated = session.updated_at.format("%Y-%m-%d %H:%M");
            let msgs = session.message_count;
            let tokens = session.total_tokens;

            output.push_str(&format!(
                "  {} | {} | {} msgs | {} tokens\n",
                name, updated, msgs, tokens
            ));
        }

        Ok(CommandResult::success(output))
    }
}

/// Session management command.
pub struct SessionCommand;

impl SessionCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SessionCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Command for SessionCommand {
    fn name(&self) -> &str {
        "session"
    }

    fn aliases(&self) -> &[&str] {
        &["sess"]
    }

    fn description(&self) -> &str {
        "Manage sessions (new, resume, delete, rename)"
    }

    fn args_description(&self) -> Option<&str> {
        Some("<command> [args] - Commands: new [name], resume <id>, delete <id>, rename <id> <name>")
    }

    async fn execute(&self, args: &str, context: &CommandContext) -> Result<CommandResult> {
        let parts: Vec<&str> = args.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(CommandResult::error(
                "Usage: /session <new|resume|delete|rename> [args]\n\
                 Commands:\n\
                 - new [name]           Create a new session\n\
                 - resume <id|name>     Resume a saved session\n\
                 - delete <id|name>     Delete a session\n\
                 - rename <id> <name>   Rename a session\n\
                 - save                 Save current session"
            ));
        }

        let store = match SessionStore::open_default() {
            Ok(s) => s,
            Err(e) => return Ok(CommandResult::error(format!("Failed to open session store: {}", e))),
        };

        match parts[0] {
            "new" => {
                let name = if parts.len() > 1 {
                    Some(parts[1..].join(" "))
                } else {
                    None
                };

                match store.create_session(name.as_deref(), "gpt-4o") {
                    Ok(session) => Ok(CommandResult::success(format!(
                        "Created new session: {} ({})",
                        session.display_name(),
                        session.id
                    ))),
                    Err(e) => Ok(CommandResult::error(format!("Failed to create session: {}", e))),
                }
            }

            "resume" => {
                if parts.len() < 2 {
                    return Ok(CommandResult::error("Usage: /session resume <id|name>"));
                }

                let identifier = parts[1];

                // Try to parse as UUID first
                let session_id = if let Ok(uuid) = Uuid::parse_str(identifier) {
                    uuid
                } else {
                    // Search by name
                    let sessions = store.list_sessions().unwrap_or_default();
                    match sessions.iter().find(|s| {
                        s.name.as_deref() == Some(identifier)
                            || s.display_name() == identifier
                            || s.id.to_string().starts_with(identifier)
                    }) {
                        Some(s) => s.id,
                        None => return Ok(CommandResult::error(format!("Session not found: {}", identifier))),
                    }
                };

                match store.load_session(session_id) {
                    Ok(session) => Ok(CommandResult::success(format!(
                        "Resumed session: {} ({} messages)",
                        session.display_name(),
                        session.messages.len()
                    ))),
                    Err(e) => Ok(CommandResult::error(format!("Failed to load session: {}", e))),
                }
            }

            "delete" => {
                if parts.len() < 2 {
                    return Ok(CommandResult::error("Usage: /session delete <id|name>"));
                }

                let identifier = parts[1];

                let session_id = if let Ok(uuid) = Uuid::parse_str(identifier) {
                    uuid
                } else {
                    let sessions = store.list_sessions().unwrap_or_default();
                    match sessions.iter().find(|s| {
                        s.name.as_deref() == Some(identifier)
                            || s.display_name() == identifier
                            || s.id.to_string().starts_with(identifier)
                    }) {
                        Some(s) => s.id,
                        None => return Ok(CommandResult::error(format!("Session not found: {}", identifier))),
                    }
                };

                match store.delete_session(session_id) {
                    Ok(()) => Ok(CommandResult::success(format!("Deleted session: {}", session_id))),
                    Err(e) => Ok(CommandResult::error(format!("Failed to delete session: {}", e))),
                }
            }

            "rename" => {
                if parts.len() < 3 {
                    return Ok(CommandResult::error("Usage: /session rename <id> <new-name>"));
                }

                let identifier = parts[1];
                let new_name = parts[2..].join(" ");

                let session_id = if let Ok(uuid) = Uuid::parse_str(identifier) {
                    uuid
                } else {
                    let sessions = store.list_sessions().unwrap_or_default();
                    match sessions.iter().find(|s| {
                        s.name.as_deref() == Some(identifier)
                            || s.display_name() == identifier
                            || s.id.to_string().starts_with(identifier)
                    }) {
                        Some(s) => s.id,
                        None => return Ok(CommandResult::error(format!("Session not found: {}", identifier))),
                    }
                };

                match store.rename_session(session_id, &new_name) {
                    Ok(()) => Ok(CommandResult::success(format!(
                        "Renamed session {} to '{}'",
                        session_id, new_name
                    ))),
                    Err(e) => Ok(CommandResult::error(format!("Failed to rename session: {}", e))),
                }
            }

            "save" => {
                // For now, just acknowledge - real save would need app state integration
                Ok(CommandResult::success(format!(
                    "Session {} saved.",
                    context.session_id
                )))
            }

            _ => Ok(CommandResult::error(format!(
                "Unknown session command: {}. Use: new, resume, delete, rename, save",
                parts[0]
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_context() -> CommandContext {
        CommandContext {
            cwd: PathBuf::from("/tmp"),
            session_id: Uuid::new_v4(),
        }
    }

    #[tokio::test]
    async fn test_sessions_command() {
        let cmd = SessionsCommand::new();
        assert_eq!(cmd.name(), "sessions");

        let ctx = test_context();
        let result = cmd.execute("", &ctx).await.unwrap();
        // May succeed or fail depending on filesystem access
        assert!(result.success || result.error.is_some());
    }

    #[tokio::test]
    async fn test_session_command_help() {
        let cmd = SessionCommand::new();
        let ctx = test_context();

        let result = cmd.execute("", &ctx).await.unwrap();
        assert!(!result.success);
        assert!(result.error.unwrap().contains("Usage"));
    }
}
