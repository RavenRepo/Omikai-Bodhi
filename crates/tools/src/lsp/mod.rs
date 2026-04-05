//! LSP tool for language server operations.
//!
//! Provides code intelligence features like go-to-definition, find references,
//! and hover information. This is a stub for future LSP integration.

use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use theasus_core::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspInput {
    /// Action to perform: definition, references, or hover
    pub action: LspAction,
    /// Path to the file
    pub file: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LspAction {
    Definition,
    References,
    Hover,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspHoverInfo {
    pub contents: String,
    pub range: Option<LspRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRange {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

pub struct LspTool {
    connected: bool,
}

impl LspTool {
    pub fn new() -> Self {
        Self { connected: false }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

impl Default for LspTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str {
        "lsp"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "lsp".to_string(),
            description: "Language server operations for code intelligence. Get definitions, references, and hover information for symbols.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Action to perform",
                        "enum": ["definition", "references", "hover"]
                    },
                    "file": {
                        "type": "string",
                        "description": "Path to the file"
                    },
                    "line": {
                        "type": "integer",
                        "description": "Line number (1-indexed)",
                        "minimum": 1
                    },
                    "column": {
                        "type": "integer",
                        "description": "Column number (1-indexed)",
                        "minimum": 1
                    }
                },
                "required": ["action", "file", "line", "column"]
            }),
        }
    }

    async fn execute(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let lsp_input: LspInput =
            serde_json::from_value(input).map_err(|e| theasus_core::TheasusError::Tool {
                tool: "lsp".to_string(),
                reason: format!("Invalid input: {}", e),
            })?;

        // Verify the file exists
        let file_path = if std::path::Path::new(&lsp_input.file).is_absolute() {
            std::path::PathBuf::from(&lsp_input.file)
        } else {
            context.cwd.join(&lsp_input.file)
        };

        if !file_path.exists() {
            return Ok(ToolResult::error(format!(
                "File not found: {}",
                lsp_input.file
            )));
        }

        // LSP is not connected - return placeholder response
        if !self.connected {
            let action_name = match lsp_input.action {
                LspAction::Definition => "Go to definition",
                LspAction::References => "Find references",
                LspAction::Hover => "Hover information",
            };

            return Ok(ToolResult::error(format!(
                "LSP not connected. {} requested for {}:{}:{}\n\n\
                 To use LSP features, a language server must be configured and running.\n\
                 Future versions will support automatic language server detection and startup.\n\n\
                 For now, you can use grep/glob tools to search for definitions and references.",
                action_name, lsp_input.file, lsp_input.line, lsp_input.column
            )));
        }

        // Placeholder for actual LSP implementation
        match lsp_input.action {
            LspAction::Definition => Ok(ToolResult::success(format!(
                "Definition lookup not yet implemented for {}:{}:{}",
                lsp_input.file, lsp_input.line, lsp_input.column
            ))),
            LspAction::References => Ok(ToolResult::success(format!(
                "References lookup not yet implemented for {}:{}:{}",
                lsp_input.file, lsp_input.line, lsp_input.column
            ))),
            LspAction::Hover => Ok(ToolResult::success(format!(
                "Hover information not yet implemented for {}:{}:{}",
                lsp_input.file, lsp_input.line, lsp_input.column
            ))),
        }
    }
}
