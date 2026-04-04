use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWriteInput {
    pub path: String,
    pub content: String,
    pub append: Option<bool>,
}

pub struct FileWriteTool;

impl FileWriteTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "file_write".to_string(),
            description: "Write content to a file".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    },
                    "append": {
                        "type": "boolean",
                        "description": "Append to file instead of overwriting (optional)"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> crate::Result<ToolResult> {
        let file_input: FileWriteInput = serde_json::from_value(input).map_err(|e| {
            crate::TheasusError::Tool {
                tool: "file_write".to_string(),
                reason: format!("Invalid input: {}", e),
            }
        })?;

        let file_path = PathBuf::from(&file_input.path);
        let full_path = if file_path.is_absolute() {
            file_path
        } else {
            context.cwd.join(&file_path)
        };

        let result = if file_input.append.unwrap_or(false) {
            tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&full_path)
                .await
        } else {
            tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&full_path)
                .await
        };

        match result {
            Ok(mut file) => {
                use tokio::io::AsyncWriteExt;
                file.write_all(file_input.content.as_bytes()).await.map_err(|e| {
                    crate::TheasusError::Tool {
                        tool: "file_write".to_string(),
                        reason: format!("Failed to write file: {}", e),
                    }
                })?;

                Ok(ToolResult {
                    success: true,
                    output: format!("Written to {}", full_path.display()),
                    error: None,
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to open file: {}", e)),
            }),
        }
    }
}

impl Default for FileWriteTool {
    fn default() -> Self {
        Self::new()
    }
}
