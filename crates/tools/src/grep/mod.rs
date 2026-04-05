use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepInput {
    pub pattern: String,
    pub path: Option<String>,
    pub files_with_matches: Option<bool>,
    pub line_number: Option<bool>,
    pub context: Option<usize>,
}

pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self {
        Self
    }

    async fn grep_file(
        &self,
        pattern: &Regex,
        path: &PathBuf,
        files_with_matches: bool,
        line_number: bool,
        context: usize,
    ) -> crate::Result<Vec<String>> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut results = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            if pattern.is_match(line) {
                let prefix =
                    if line_number { format!("{}: ", line_num + 1) } else { String::new() };

                if files_with_matches {
                    results.push(path.display().to_string());
                    break;
                } else {
                    let start = line_num.saturating_sub(context);
                    let end = (line_num + context + 1).min(content.lines().count());

                    if context > 0 {
                        results.push(format!("{}{}", prefix, line));
                        for i in start..line_num {
                            results.push(format!("  {}", content.lines().nth(i).unwrap_or("")));
                        }
                        for i in (line_num + 1)..end {
                            results.push(format!("  {}", content.lines().nth(i).unwrap_or("")));
                        }
                    } else {
                        results.push(format!("{}{}", prefix, line));
                    }
                }
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "grep".to_string(),
            description: "Search for patterns in files".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Path to search in (file or directory)"
                    },
                    "files_with_matches": {
                        "type": "boolean",
                        "description": "Only show filenames with matches"
                    },
                    "line_number": {
                        "type": "boolean",
                        "description": "Show line numbers"
                    },
                    "context": {
                        "type": "number",
                        "description": "Lines of context to show"
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
        let grep_input: GrepInput =
            serde_json::from_value(input).map_err(|e| crate::TheasusError::Tool {
                tool: "grep".to_string(),
                reason: format!("Invalid input: {}", e),
            })?;

        let regex = Regex::new(&grep_input.pattern).map_err(|e| crate::TheasusError::Tool {
            tool: "grep".to_string(),
            reason: format!("Invalid regex: {}", e),
        })?;

        let search_path = grep_input
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

        let files_with_matches = grep_input.files_with_matches.unwrap_or(false);
        let line_number = grep_input.line_number.unwrap_or(false);
        let context_lines = grep_input.context.unwrap_or(0);

        let mut all_results = Vec::new();

        if search_path.is_file() {
            let results = self
                .grep_file(&regex, &search_path, files_with_matches, line_number, context_lines)
                .await?;
            all_results.extend(results);
        } else if search_path.is_dir() {
            let mut entries = tokio::fs::read_dir(&search_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(results) = self
                        .grep_file(&regex, &path, files_with_matches, line_number, context_lines)
                        .await
                    {
                        if !results.is_empty() {
                            all_results.extend(results);
                        }
                    }
                }
            }
        }

        Ok(ToolResult {
            success: !all_results.is_empty(),
            output: all_results.join("\n"),
            error: if all_results.is_empty() { Some("No matches found".to_string()) } else { None },
        })
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}
