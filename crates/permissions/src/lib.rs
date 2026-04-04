use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PermissionMode {
    #[default]
    Default,
    AcceptEdits,
    BypassPermissions,
    DontAsk,
    Plan,
    Auto,
    Bubble,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionBehavior {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub name: String,
    pub source: PermissionRuleSource,
    pub behavior: PermissionBehavior,
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PermissionRuleSource {
    UserSettings,
    ProjectSettings,
    LocalSettings,
    FlagSettings,
    PolicySettings,
    CliArg,
    Command,
    Session,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionContext {
    pub tool_name: String,
    pub input: serde_json::Value,
    pub cwd: std::path::PathBuf,
    pub session_id: uuid::Uuid,
}

pub struct PermissionManager {
    pub mode: PermissionMode,
    pub always_allow: HashMap<PermissionRuleSource, Vec<String>>,
    pub always_deny: HashMap<PermissionRuleSource, Vec<String>>,
    pub always_ask: HashMap<PermissionRuleSource, Vec<String>>,
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            mode: PermissionMode::Default,
            always_allow: HashMap::new(),
            always_deny: HashMap::new(),
            always_ask: HashMap::new(),
        }
    }

    pub fn with_mode(mut self, mode: PermissionMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn add_rule(&mut self, rule: PermissionRule) {
        match rule.behavior {
            PermissionBehavior::Allow => {
                self.always_allow
                    .entry(rule.source)
                    .or_default()
                    .push(rule.name);
            }
            PermissionBehavior::Deny => {
                self.always_deny
                    .entry(rule.source)
                    .or_default()
                    .push(rule.name);
            }
            PermissionBehavior::Ask => {
                self.always_ask
                    .entry(rule.source)
                    .or_default()
                    .push(rule.name);
            }
        }
    }

    pub fn check_permission(
        &self,
        tool_name: &str,
        context: &ToolPermissionContext,
    ) -> PermissionResult {
        if self.mode == PermissionMode::BypassPermissions {
            return PermissionResult::Allowed;
        }

        for (source, tools) in &self.always_deny {
            if tools.contains(&tool_name.to_string()) {
                return PermissionResult::Denied {
                    reason: format!("Tool {} denied by {:?}", tool_name, source),
                };
            }
        }

        for tools in self.always_allow.values() {
            if tools.contains(&tool_name.to_string()) {
                return PermissionResult::Allowed;
            }
        }

        for tools in self.always_ask.values() {
            if tools.contains(&tool_name.to_string()) {
                return PermissionResult::Ask {
                    tool: tool_name.to_string(),
                    context: format!("Working directory: {}", context.cwd.display()),
                };
            }
        }

        match self.mode {
            PermissionMode::Default | PermissionMode::DontAsk => PermissionResult::Ask {
                tool: tool_name.to_string(),
                context: "Default permission check".to_string(),
            },
            PermissionMode::AcceptEdits | PermissionMode::Bubble => PermissionResult::Allowed,
            PermissionMode::BypassPermissions => PermissionResult::Allowed,
            PermissionMode::Plan => PermissionResult::Ask {
                tool: tool_name.to_string(),
                context: "Plan mode - confirmation needed".to_string(),
            },
            PermissionMode::Auto => PermissionResult::Allowed,
        }
    }

    pub fn allow_tool(&mut self, tool: impl Into<String>) {
        let tool = tool.into();
        self.always_allow
            .entry(PermissionRuleSource::UserSettings)
            .or_default()
            .push(tool);
    }

    pub fn deny_tool(&mut self, tool: impl Into<String>) {
        let tool = tool.into();
        self.always_deny
            .entry(PermissionRuleSource::UserSettings)
            .or_default()
            .push(tool);
    }

    pub fn ask_tool(&mut self, tool: impl Into<String>) {
        let tool = tool.into();
        self.always_ask
            .entry(PermissionRuleSource::UserSettings)
            .or_default()
            .push(tool);
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionResult {
    Allowed,
    Denied { reason: String },
    Ask { tool: String, context: String },
}

impl PermissionResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, PermissionResult::Allowed)
    }

    pub fn is_denied(&self) -> bool {
        matches!(self, PermissionResult::Denied { .. })
    }

    pub fn requires_ask(&self) -> bool {
        matches!(self, PermissionResult::Ask { .. })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    #[error("Permission denied: {0}")]
    Denied(String),

    #[error("Permission check failed: {0}")]
    CheckFailed(String),

    #[error("Invalid rule: {0}")]
    InvalidRule(String),
}

pub type PermissionResultT<T> = std::result::Result<T, PermissionError>;

pub fn is_command_safe(command: &str) -> bool {
    let dangerous_patterns = [
        "rm -rf",
        "rm /",
        "mkfs",
        "dd if=",
        ":(){:|:&};:",
        "wget",
        "curl |",
    ];

    let lower = command.to_lowercase();
    for pattern in dangerous_patterns {
        if lower.contains(pattern) {
            return false;
        }
    }
    true
}

pub fn is_path_safe(path: &std::path::Path, allowed_dir: &std::path::Path) -> bool {
    if let Ok(canonical) = path.canonicalize() {
        if let Ok(allowed) = allowed_dir.canonicalize() {
            return canonical.starts_with(allowed);
        }
    }
    false
}
