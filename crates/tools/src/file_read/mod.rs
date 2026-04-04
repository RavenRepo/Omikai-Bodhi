use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadInput {
    pub path: String,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

pub struct FileReadTool;

impl FileReadTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "file_read".to_string(),
            description: "Read file contents".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    },
                    "offset": {
                        "type": "number",
                        "description": "Line offset to start reading from (optional)"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of lines to read (optional)"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> crate::Result<ToolResult> {
        let file_input: FileReadInput = serde_json::from_value(input).map_err(|e| {
            crate::TheasusError::Tool {
                tool: "file_read".to_string(),
                reason: format!("Invalid input: {}", e),
            }
        })?;

        let file_path = PathBuf::from(&file_input.path);
        let full_path = if file_path.is_absolute() {
            file_path
        } else {
            context.cwd.join(&file_path)
        };

        let content = tokio::fs::read_to_string(&full_path).await.map_err(|e| {
            crate::TheasusError::Tool {
                tool: "file_read".to_string(),
                reason: format!("Failed to read file: {}", e),
            }
        })?;

        let lines: Vec<&str> = content.lines().collect();
        let offset = file_input.offset.unwrap_or(0);
        let limit = file_input.limit.unwrap_or(lines.len());

        let selected: String = lines
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ToolResult {
            success: true,
            output: selected,
            error: None,
        })
    }
}

impl Default for FileReadTool {
    fn default() -> Self {
        Self::new()
    }
}
