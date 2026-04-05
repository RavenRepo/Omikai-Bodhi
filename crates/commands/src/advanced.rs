use crate::{Command, CommandContext, CommandResult};
use async_trait::async_trait;
use std::fmt;
use std::fs;
use std::path::Path;

// /mcp command
pub struct McpCommand;

impl McpCommand {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for McpCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("McpCommand").finish()
    }
}

impl Clone for McpCommand {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for McpCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Command for McpCommand {
    fn name(&self) -> &str {
        "mcp"
    }

    fn description(&self) -> &str {
        "MCP server management"
    }

    fn args_description(&self) -> Option<&str> {
        Some("<list|connect <name>|disconnect <name>>")
    }

    async fn execute(
        &self,
        args: &str,
        _context: &CommandContext,
    ) -> theasus_core::Result<CommandResult> {
        let parts: Vec<&str> = args.split_whitespace().collect();

        match parts.first().copied() {
            None | Some("list") => {
                Ok(CommandResult::success(
                    "MCP Servers:\n  (none connected)\n\nUse `/mcp connect <name>` to connect a server",
                ))
            }
            Some("connect") => {
                if let Some(name) = parts.get(1) {
                    Ok(CommandResult::success(format!(
                        "Connecting to MCP server '{}'... (not yet implemented)",
                        name
                    )))
                } else {
                    Ok(CommandResult::error("Usage: /mcp connect <server_name>"))
                }
            }
            Some("disconnect") => {
                if let Some(name) = parts.get(1) {
                    Ok(CommandResult::success(format!(
                        "Disconnecting from MCP server '{}'... (not yet implemented)",
                        name
                    )))
                } else {
                    Ok(CommandResult::error("Usage: /mcp disconnect <server_name>"))
                }
            }
            Some(cmd) => Ok(CommandResult::error(format!(
                "Unknown subcommand '{}'. Use: list, connect, disconnect",
                cmd
            ))),
        }
    }
}

// /permissions command
pub struct PermissionsCommand;

impl PermissionsCommand {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for PermissionsCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PermissionsCommand").finish()
    }
}

impl Clone for PermissionsCommand {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for PermissionsCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Command for PermissionsCommand {
    fn name(&self) -> &str {
        "permissions"
    }

    fn aliases(&self) -> &[&str] {
        &["perms"]
    }

    fn description(&self) -> &str {
        "Manage tool permission rules"
    }

    fn args_description(&self) -> Option<&str> {
        Some("<list|allow <tool>|deny <tool>|ask <tool>>")
    }

    async fn execute(
        &self,
        args: &str,
        _context: &CommandContext,
    ) -> theasus_core::Result<CommandResult> {
        let parts: Vec<&str> = args.split_whitespace().collect();

        match parts.first().copied() {
            None | Some("list") => {
                Ok(CommandResult::success(
                    r#"Permission Rules:
  bash       - ask (prompts before execution)
  file_read  - allow (always allowed)
  file_write - ask (prompts before execution)
  grep       - allow (always allowed)
  glob       - allow (always allowed)

Use `/permissions allow|deny|ask <tool>` to change rules"#,
                ))
            }
            Some("allow") => {
                if let Some(tool) = parts.get(1) {
                    Ok(CommandResult::success(format!(
                        "Permission for '{}' set to: allow (not yet persisted)",
                        tool
                    )))
                } else {
                    Ok(CommandResult::error("Usage: /permissions allow <tool_name>"))
                }
            }
            Some("deny") => {
                if let Some(tool) = parts.get(1) {
                    Ok(CommandResult::success(format!(
                        "Permission for '{}' set to: deny (not yet persisted)",
                        tool
                    )))
                } else {
                    Ok(CommandResult::error("Usage: /permissions deny <tool_name>"))
                }
            }
            Some("ask") => {
                if let Some(tool) = parts.get(1) {
                    Ok(CommandResult::success(format!(
                        "Permission for '{}' set to: ask (not yet persisted)",
                        tool
                    )))
                } else {
                    Ok(CommandResult::error("Usage: /permissions ask <tool_name>"))
                }
            }
            Some(cmd) => Ok(CommandResult::error(format!(
                "Unknown subcommand '{}'. Use: list, allow, deny, ask",
                cmd
            ))),
        }
    }
}

// /resume command
pub struct ResumeCommand;

impl ResumeCommand {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for ResumeCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResumeCommand").finish()
    }
}

impl Clone for ResumeCommand {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for ResumeCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Command for ResumeCommand {
    fn name(&self) -> &str {
        "resume"
    }

    fn description(&self) -> &str {
        "Resume a previous session"
    }

    fn args_description(&self) -> Option<&str> {
        Some("[session_id] - Session ID to resume")
    }

    async fn execute(
        &self,
        args: &str,
        _context: &CommandContext,
    ) -> theasus_core::Result<CommandResult> {
        let session_id = args.trim();

        if session_id.is_empty() {
            // List available sessions
            let sessions_dir = dirs::data_dir()
                .map(|p| p.join("bodhi").join("sessions"))
                .unwrap_or_else(|| std::path::PathBuf::from(".bodhi/sessions"));

            if !sessions_dir.exists() {
                return Ok(CommandResult::success("No saved sessions found"));
            }

            let mut sessions = Vec::new();
            if let Ok(entries) = fs::read_dir(&sessions_dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".json") {
                            sessions.push(name.trim_end_matches(".json").to_string());
                        }
                    }
                }
            }

            if sessions.is_empty() {
                Ok(CommandResult::success("No saved sessions found"))
            } else {
                sessions.sort();
                sessions.reverse();
                let list = sessions
                    .iter()
                    .take(10)
                    .map(|s| format!("  {}", s))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(CommandResult::success(format!(
                    "Available sessions (most recent first):\n{}\n\nUse `/resume <session_id>` to resume",
                    list
                )))
            }
        } else {
            Ok(CommandResult::success(format!(
                "Resuming session: {} (not yet implemented)",
                session_id
            )))
        }
    }
}

// /export command
pub struct ExportCommand;

impl ExportCommand {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for ExportCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExportCommand").finish()
    }
}

impl Clone for ExportCommand {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for ExportCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Command for ExportCommand {
    fn name(&self) -> &str {
        "export"
    }

    fn description(&self) -> &str {
        "Export conversation to file"
    }

    fn args_description(&self) -> Option<&str> {
        Some("<json|markdown> <path>")
    }

    async fn execute(
        &self,
        args: &str,
        context: &CommandContext,
    ) -> theasus_core::Result<CommandResult> {
        let parts: Vec<&str> = args.split_whitespace().collect();

        if parts.len() < 2 {
            return Ok(CommandResult::error(
                "Usage: /export <json|markdown> <path>\nExample: /export markdown conversation.md",
            ));
        }

        let format = parts[0].to_lowercase();
        let path_str = parts[1];

        // Resolve path relative to cwd
        let path = if Path::new(path_str).is_absolute() {
            std::path::PathBuf::from(path_str)
        } else {
            context.cwd.join(path_str)
        };

        match format.as_str() {
            "json" => {
                let content = serde_json::json!({
                    "session_id": context.session_id.to_string(),
                    "exported_at": chrono::Utc::now().to_rfc3339(),
                    "messages": [],
                    "note": "Conversation export not yet fully implemented"
                });

                match fs::write(&path, serde_json::to_string_pretty(&content).unwrap()) {
                    Ok(_) => Ok(CommandResult::success(format!(
                        "Exported conversation to: {}",
                        path.display()
                    ))),
                    Err(e) => Ok(CommandResult::error(format!("Failed to write file: {}", e))),
                }
            }
            "markdown" | "md" => {
                let content = format!(
                    "# Conversation Export\n\n**Session ID:** {}\n**Exported:** {}\n\n---\n\n_Conversation export not yet fully implemented_\n",
                    context.session_id,
                    chrono::Utc::now().to_rfc3339()
                );

                match fs::write(&path, content) {
                    Ok(_) => Ok(CommandResult::success(format!(
                        "Exported conversation to: {}",
                        path.display()
                    ))),
                    Err(e) => Ok(CommandResult::error(format!("Failed to write file: {}", e))),
                }
            }
            _ => Ok(CommandResult::error(format!(
                "Unknown format '{}'. Use: json, markdown",
                format
            ))),
        }
    }
}

// /memory command
pub struct MemoryCommand;

impl MemoryCommand {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for MemoryCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemoryCommand").finish()
    }
}

impl Clone for MemoryCommand {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for MemoryCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Command for MemoryCommand {
    fn name(&self) -> &str {
        "memory"
    }

    fn aliases(&self) -> &[&str] {
        &["mem", "context"]
    }

    fn description(&self) -> &str {
        "Show or clear context memory"
    }

    fn args_description(&self) -> Option<&str> {
        Some("[clear] - Clear context memory")
    }

    async fn execute(
        &self,
        args: &str,
        context: &CommandContext,
    ) -> theasus_core::Result<CommandResult> {
        let subcommand = args.trim().to_lowercase();

        if subcommand == "clear" {
            Ok(CommandResult::success(
                "Context memory cleared (not yet implemented)",
            ))
        } else {
            Ok(CommandResult::success(format!(
                r#"Context Memory Status:
  Session:        {}
  Messages:       0 (tracking not yet implemented)
  Token estimate: ~0 tokens
  Max context:    128,000 tokens

Use `/memory clear` to reset context"#,
                context.session_id
            )))
        }
    }
}
