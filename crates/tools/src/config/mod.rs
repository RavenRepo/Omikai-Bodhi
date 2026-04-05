use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use theasus_settings::{PermissionMode, Settings, Theme};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInput {
    pub action: ConfigAction,
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigAction {
    Get,
    Set,
}

pub struct ConfigTool;

impl ConfigTool {
    pub fn new() -> Self {
        Self
    }

    fn parse_theme(value: &str) -> Option<Theme> {
        match value.to_lowercase().as_str() {
            "dark" => Some(Theme::Dark),
            "light" => Some(Theme::Light),
            "system" => Some(Theme::System),
            _ => None,
        }
    }

    fn parse_permission_mode(value: &str) -> Option<PermissionMode> {
        match value.to_lowercase().as_str() {
            "default" => Some(PermissionMode::Default),
            "accept_edits" | "acceptedits" => Some(PermissionMode::AcceptEdits),
            "bypass_permissions" | "bypasspermissions" => Some(PermissionMode::BypassPermissions),
            "dont_ask" | "dontask" => Some(PermissionMode::DontAsk),
            "plan" => Some(PermissionMode::Plan),
            "auto" => Some(PermissionMode::Auto),
            "bubble" => Some(PermissionMode::Bubble),
            _ => None,
        }
    }

    fn theme_to_string(theme: &Theme) -> &'static str {
        match theme {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::System => "system",
        }
    }

    fn permission_mode_to_string(mode: &PermissionMode) -> &'static str {
        match mode {
            PermissionMode::Default => "default",
            PermissionMode::AcceptEdits => "accept_edits",
            PermissionMode::BypassPermissions => "bypass_permissions",
            PermissionMode::DontAsk => "dont_ask",
            PermissionMode::Plan => "plan",
            PermissionMode::Auto => "auto",
            PermissionMode::Bubble => "bubble",
        }
    }
}

#[async_trait]
impl Tool for ConfigTool {
    fn name(&self) -> &str {
        "config"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "config".to_string(),
            description: "Get or set configuration settings (model, theme, permission_mode)"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["get", "set"],
                        "description": "The action to perform: 'get' to read a setting, 'set' to write a setting"
                    },
                    "key": {
                        "type": "string",
                        "enum": ["model", "theme", "permission_mode"],
                        "description": "The configuration key to get or set"
                    },
                    "value": {
                        "type": "string",
                        "description": "The value to set (required for 'set' action). For theme: dark/light/system. For permission_mode: default/accept_edits/bypass_permissions/dont_ask/plan/auto/bubble"
                    }
                },
                "required": ["action", "key"]
            }),
        }
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> crate::Result<ToolResult> {
        let config_input: ConfigInput =
            serde_json::from_value(input).map_err(|e| crate::TheasusError::Tool {
                tool: "config".to_string(),
                reason: format!("Invalid input: {}", e),
            })?;

        let mut settings = Settings::load().map_err(|e| crate::TheasusError::Tool {
            tool: "config".to_string(),
            reason: format!("Failed to load settings: {}", e),
        })?;

        match config_input.action {
            ConfigAction::Get => {
                let value = match config_input.key.as_str() {
                    "model" => settings.model.clone(),
                    "theme" => Self::theme_to_string(&settings.theme).to_string(),
                    "permission_mode" => {
                        Self::permission_mode_to_string(&settings.permission_mode).to_string()
                    }
                    _ => {
                        return Ok(ToolResult::error(format!(
                            "Unknown configuration key: {}. Supported keys: model, theme, permission_mode",
                            config_input.key
                        )));
                    }
                };
                Ok(ToolResult::success(format!(
                    "{} = {}",
                    config_input.key, value
                )))
            }
            ConfigAction::Set => {
                let value = config_input.value.ok_or_else(|| crate::TheasusError::Tool {
                    tool: "config".to_string(),
                    reason: "Value is required for 'set' action".to_string(),
                })?;

                match config_input.key.as_str() {
                    "model" => {
                        settings.model = value.clone();
                    }
                    "theme" => {
                        let theme = Self::parse_theme(&value).ok_or_else(|| {
                            crate::TheasusError::Tool {
                                tool: "config".to_string(),
                                reason: format!(
                                    "Invalid theme value: {}. Valid values: dark, light, system",
                                    value
                                ),
                            }
                        })?;
                        settings.theme = theme;
                    }
                    "permission_mode" => {
                        let mode = Self::parse_permission_mode(&value).ok_or_else(|| {
                            crate::TheasusError::Tool {
                                tool: "config".to_string(),
                                reason: format!(
                                    "Invalid permission_mode value: {}. Valid values: default, accept_edits, bypass_permissions, dont_ask, plan, auto, bubble",
                                    value
                                ),
                            }
                        })?;
                        settings.permission_mode = mode;
                    }
                    _ => {
                        return Ok(ToolResult::error(format!(
                            "Unknown configuration key: {}. Supported keys: model, theme, permission_mode",
                            config_input.key
                        )));
                    }
                }

                settings.save().map_err(|e| crate::TheasusError::Tool {
                    tool: "config".to_string(),
                    reason: format!("Failed to save settings: {}", e),
                })?;

                Ok(ToolResult::success(format!(
                    "Set {} = {}",
                    config_input.key, value
                )))
            }
        }
    }
}

impl Default for ConfigTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[allow(dead_code)]
    fn test_context() -> ToolContext {
        ToolContext {
            cwd: PathBuf::from("."),
            session_id: uuid::Uuid::new_v4(),
            user_id: None,
        }
    }

    #[test]
    fn test_parse_theme() {
        assert_eq!(ConfigTool::parse_theme("dark"), Some(Theme::Dark));
        assert_eq!(ConfigTool::parse_theme("LIGHT"), Some(Theme::Light));
        assert_eq!(ConfigTool::parse_theme("System"), Some(Theme::System));
        assert_eq!(ConfigTool::parse_theme("invalid"), None);
    }

    #[test]
    fn test_parse_permission_mode() {
        assert_eq!(
            ConfigTool::parse_permission_mode("default"),
            Some(PermissionMode::Default)
        );
        assert_eq!(
            ConfigTool::parse_permission_mode("accept_edits"),
            Some(PermissionMode::AcceptEdits)
        );
        assert_eq!(
            ConfigTool::parse_permission_mode("bypass_permissions"),
            Some(PermissionMode::BypassPermissions)
        );
        assert_eq!(
            ConfigTool::parse_permission_mode("auto"),
            Some(PermissionMode::Auto)
        );
        assert_eq!(ConfigTool::parse_permission_mode("invalid"), None);
    }

    #[test]
    fn test_tool_definition() {
        let tool = ConfigTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "config");
        assert!(def.input_schema.is_object());
    }
}
