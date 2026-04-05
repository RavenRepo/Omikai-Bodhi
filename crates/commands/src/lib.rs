use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use theasus_core::Result;

pub mod builtins;

pub use builtins::{
    AgentsCommand, ClearCommand, CompactCommand, ConfigCommand, EnvCommand, ExitCommand,
    HelpCommand, HistoryCommand, ModelCommand, PwdCommand, StatusCommand, ToolsCommand,
};

// Git commands
mod git;
pub use git::{BranchCommand, CommitCommand, DiffCommand, ReviewCommand};

// Advanced commands
mod advanced;
pub use advanced::{ExportCommand, McpCommand, MemoryCommand, PermissionsCommand, ResumeCommand};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

impl CommandResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(msg.into()),
        }
    }
}

pub struct CommandContext {
    pub cwd: std::path::PathBuf,
    pub session_id: uuid::Uuid,
}

#[async_trait]
pub trait Command: Send + Sync {
    fn name(&self) -> &str;
    fn aliases(&self) -> &[&str] {
        &[]
    }
    fn description(&self) -> &str;
    fn args_description(&self) -> Option<&str> {
        None
    }

    async fn execute(&self, args: &str, context: &CommandContext) -> Result<CommandResult>;
}

pub struct CommandRegistry {
    commands: HashMap<String, Arc<dyn Command>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
        };
        registry.register_builtins();
        registry
    }

    fn register_builtins(&mut self) {
        self.register(HelpCommand::new());
        if let Some(cmd) = self.get("help") {
            self.register_alias("h", cmd.clone());
            self.register_alias("?", cmd.clone());
        }

        self.register(ClearCommand::new());
        if let Some(cmd) = self.get("clear") {
            self.register_alias("c", cmd.clone());
        }

        self.register(ExitCommand::new());
        if let Some(cmd) = self.get("exit") {
            self.register_alias("e", cmd.clone());
            self.register_alias("quit", cmd.clone());
            self.register_alias("q", cmd.clone());
        }

        self.register(StatusCommand::new());
        self.register(ModelCommand::new());
        self.register(CompactCommand::new());
        self.register(ToolsCommand::new());
        self.register(AgentsCommand::new());
        self.register(ConfigCommand::new());
        self.register(EnvCommand::new());
        self.register(PwdCommand::new());
        self.register(HistoryCommand::new());

        // Git commands
        self.register(CommitCommand::new());
        self.register(DiffCommand::new());
        self.register(ReviewCommand::new());
        self.register(BranchCommand::new());
        if let Some(cmd) = self.get("branch") {
            self.register_alias("br", cmd.clone());
        }

        // Advanced commands
        self.register(McpCommand::new());
        self.register(PermissionsCommand::new());
        if let Some(cmd) = self.get("permissions") {
            self.register_alias("perms", cmd.clone());
        }
        self.register(ResumeCommand::new());
        self.register(ExportCommand::new());
        self.register(MemoryCommand::new());
        if let Some(cmd) = self.get("memory") {
            self.register_alias("mem", cmd.clone());
            self.register_alias("context", cmd.clone());
        }
    }

    pub fn register<C: Command + 'static>(&mut self, command: C) {
        let name = command.name().to_string();
        self.commands.insert(name, Arc::new(command));
    }

    pub fn register_alias(&mut self, alias: &str, command: Arc<dyn Command>) {
        self.commands.insert(alias.to_string(), command);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Command>> {
        // Strip the leading slash if present
        let name = name.trim_start_matches('/');
        self.commands.get(name).cloned()
    }

    pub fn list(&self) -> Vec<(&str, &str)> {
        self.commands
            .values()
            .map(|cmd| (cmd.name(), cmd.description()))
            .collect()
    }

    pub fn list_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.commands.keys().cloned().collect();
        names.sort();
        names.dedup();
        names
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Command not found: {0}")]
    NotFound(String),

    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Invalid arguments for command {command}: {message}")]
    InvalidArguments { command: String, message: String },
}

pub type CommandResultT<T> = std::result::Result<T, CommandError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_registry_creation() {
        let registry = CommandRegistry::new();
        assert!(registry.get("help").is_some());
        assert!(registry.get("clear").is_some());
        assert!(registry.get("exit").is_some());
    }

    #[test]
    fn test_command_aliases() {
        let registry = CommandRegistry::new();
        assert!(registry.get("h").is_some());
        assert!(registry.get("?").is_some());
        assert!(registry.get("q").is_some());
        assert!(registry.get("quit").is_some());
    }

    #[test]
    fn test_command_result_success() {
        let result = CommandResult::success("Operation completed");
        assert!(result.success);
        assert_eq!(result.output, "Operation completed");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_command_result_error() {
        let result = CommandResult::error("Something went wrong");
        assert!(!result.success);
        assert!(result.output.is_empty());
        assert_eq!(result.error, Some("Something went wrong".to_string()));
    }

    #[tokio::test]
    async fn test_help_command_execution() {
        let help_cmd = HelpCommand::new();
        assert_eq!(help_cmd.name(), "help");
        assert!(!help_cmd.description().is_empty());

        let context = CommandContext {
            cwd: std::path::PathBuf::from("."),
            session_id: uuid::Uuid::new_v4(),
        };

        let result = help_cmd.execute("", &context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_clear_command_execution() {
        let clear_cmd = ClearCommand::new();
        assert_eq!(clear_cmd.name(), "clear");

        let context = CommandContext {
            cwd: std::path::PathBuf::from("."),
            session_id: uuid::Uuid::new_v4(),
        };

        let result = clear_cmd.execute("", &context).await;
        assert!(result.is_ok());
    }
}
