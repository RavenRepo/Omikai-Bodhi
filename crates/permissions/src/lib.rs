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

#[derive(Debug)]
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

#[derive(Debug, Serialize, Deserialize)]
struct PermissionRulesFile {
    always_allow: HashMap<PermissionRuleSource, Vec<String>>,
    always_deny: HashMap<PermissionRuleSource, Vec<String>>,
    always_ask: HashMap<PermissionRuleSource, Vec<String>>,
}

impl PermissionManager {
    pub fn save(&self, path: &std::path::Path) -> Result<(), PermissionError> {
        let rules = PermissionRulesFile {
            always_allow: self.always_allow.clone(),
            always_deny: self.always_deny.clone(),
            always_ask: self.always_ask.clone(),
        };
        let json = serde_json::to_string_pretty(&rules)
            .map_err(|e| PermissionError::SerializationFailed(e.to_string()))?;
        std::fs::write(path, json).map_err(|e| PermissionError::IoError(e.to_string()))?;
        Ok(())
    }

    pub fn load(path: &std::path::Path) -> Result<Self, PermissionError> {
        let json =
            std::fs::read_to_string(path).map_err(|e| PermissionError::IoError(e.to_string()))?;
        let rules: PermissionRulesFile = serde_json::from_str(&json)
            .map_err(|e| PermissionError::SerializationFailed(e.to_string()))?;
        Ok(Self {
            mode: PermissionMode::default(),
            always_allow: rules.always_allow,
            always_deny: rules.always_deny,
            always_ask: rules.always_ask,
        })
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

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_permission_mode_default() {
        let mode = PermissionMode::default();
        assert_eq!(mode, PermissionMode::Default);
    }

    #[test]
    fn test_permission_check_allow() {
        let mut manager = PermissionManager::new().with_mode(PermissionMode::Default);
        manager.allow_tool("bash");

        let context = ToolPermissionContext {
            tool_name: "bash".to_string(),
            input: serde_json::json!({}),
            cwd: PathBuf::from("/tmp"),
            session_id: uuid::Uuid::new_v4(),
        };

        let result = manager.check_permission("bash", &context);
        assert!(result.is_allowed());
    }

    #[test]
    fn test_permission_check_deny() {
        let mut manager = PermissionManager::new();
        manager.deny_tool("dangerous_tool");

        let context = ToolPermissionContext {
            tool_name: "dangerous_tool".to_string(),
            input: serde_json::json!({}),
            cwd: PathBuf::from("/tmp"),
            session_id: uuid::Uuid::new_v4(),
        };

        let result = manager.check_permission("dangerous_tool", &context);
        assert!(result.is_denied());
    }

    #[test]
    fn test_is_command_safe() {
        assert!(is_command_safe("ls -la"));
        assert!(is_command_safe("cat file.txt"));
        assert!(!is_command_safe("rm -rf /"));
        assert!(!is_command_safe("rm -rf"));
        assert!(!is_command_safe("dd if=/dev/zero"));
    }

    #[test]
    fn test_is_path_safe() {
        let cwd = std::env::current_dir().unwrap();
        assert!(is_path_safe(&cwd, &cwd));

        let parent = cwd.parent().map(|p| p.to_path_buf());
        if let Some(parent) = parent {
            assert!(!is_path_safe(&parent, &cwd));
        }
    }

    #[test]
    fn test_bypass_permissions_mode() {
        let manager = PermissionManager::new().with_mode(PermissionMode::BypassPermissions);
        let context = ToolPermissionContext {
            tool_name: "any_tool".to_string(),
            input: serde_json::json!({}),
            cwd: PathBuf::from("/tmp"),
            session_id: uuid::Uuid::new_v4(),
        };
        let result = manager.check_permission("any_tool", &context);
        assert!(result.is_allowed());
    }

    #[test]
    fn test_permission_result_helpers() {
        let allowed = PermissionResult::Allowed;
        assert!(allowed.is_allowed());
        assert!(!allowed.is_denied());
        assert!(!allowed.requires_ask());

        let denied = PermissionResult::Denied {
            reason: "test".to_string(),
        };
        assert!(!denied.is_allowed());
        assert!(denied.is_denied());
        assert!(!denied.requires_ask());

        let ask = PermissionResult::Ask {
            tool: "test".to_string(),
            context: "ctx".to_string(),
        };
        assert!(!ask.is_allowed());
        assert!(!ask.is_denied());
        assert!(ask.requires_ask());
    }
}
