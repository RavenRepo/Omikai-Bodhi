use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobInput {
    pub pattern: String,
    pub path: Option<String>,
}

pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self {
        Self
    }

    fn walk_dir(dir: &PathBuf, pattern: &str, matches: &mut Vec<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::walk_dir(&path, pattern, matches);
                } else if let Some(p) = path.to_str() {
                    if pattern.contains('*') {
                        if let Ok(glob) = glob::Pattern::new(pattern) {
                            if glob.matches(p) {
                                matches.push(path.display().to_string());
                            }
                        }
                    } else if p.contains(pattern) {
                        matches.push(path.display().to_string());
                    }
                }
            }
        }
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "glob".to_string(),
            description: "Find files matching a glob pattern".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (e.g., '**/*.rs')"
                    },
                    "path": {
                        "type": "string",
                        "description": "Base path to search from (optional, defaults to cwd)"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> crate::Result<ToolResult> {
        let glob_input: GlobInput =
            serde_json::from_value(input).map_err(|e| crate::TheasusError::Tool {
                tool: "glob".to_string(),
                reason: format!("Invalid input: {}", e),
            })?;

        let base_path = glob_input
            .path
            .map(|p| {
                let path = PathBuf::from(&p);
                if path.is_absolute() {
                    path
                } else {
                    context.cwd.join(&p)
                }
            })
            .unwrap_or(context.cwd.clone());

        let mut matches = Vec::new();
        Self::walk_dir(&base_path, &glob_input.pattern, &mut matches);
        matches.sort();

        Ok(ToolResult {
            success: !matches.is_empty(),
            output: matches.join("\n"),
            error: if matches.is_empty() { Some("No files matched".to_string()) } else { None },
        })
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}
