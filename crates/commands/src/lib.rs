use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use theasus_core::Result;

pub mod builtins;

pub use builtins::{HelpCommand, ClearCommand, ExitCommand, StatusCommand, ModelCommand, CompactCommand, ToolsCommand, AgentsCommand, ConfigCommand, EnvCommand, PwdCommand, HistoryCommand};

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
    fn aliases(&self) -> &[&str] { &[] }
    fn description(&self) -> &str;
    fn args_description(&self) -> Option<&str> { None }
    
    async fn execute(
        &self,
        args: &str,
        context: &CommandContext,
    ) -> Result<CommandResult>;
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
