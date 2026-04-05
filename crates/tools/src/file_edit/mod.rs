use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEditInput {
    pub path: String,
    pub old_str: String,
    pub new_str: String,
}

pub struct FileEditTool;

impl FileEditTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "file_edit".to_string(),
            description: "Make surgical edits to a file by replacing exactly one occurrence of old_str with new_str".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to edit"
                    },
                    "old_str": {
                        "type": "string",
                        "description": "The exact string to find and replace (must match exactly one occurrence)"
                    },
                    "new_str": {
                        "type": "string",
                        "description": "The string to replace old_str with"
                    }
                },
                "required": ["path", "old_str", "new_str"]
            }),
        }
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> crate::Result<ToolResult> {
        let edit_input: FileEditInput =
            serde_json::from_value(input).map_err(|e| crate::TheasusError::Tool {
                tool: "file_edit".to_string(),
                reason: format!("Invalid input: {}", e),
            })?;

        let file_path = PathBuf::from(&edit_input.path);
        let full_path =
            if file_path.is_absolute() { file_path } else { context.cwd.join(&file_path) };

        let content =
            tokio::fs::read_to_string(&full_path).await.map_err(|e| crate::TheasusError::Tool {
                tool: "file_edit".to_string(),
                reason: format!("Failed to read file: {}", e),
            })?;

        let match_count = content.matches(&edit_input.old_str).count();

        if match_count == 0 {
            return Ok(ToolResult::error(format!(
                "No matches found for the specified old_str in {}",
                full_path.display()
            )));
        }

        if match_count > 1 {
            return Ok(ToolResult::error(format!(
                "Found {} matches for old_str in {} (expected exactly 1). Please provide more context to make the match unique.",
                match_count,
                full_path.display()
            )));
        }

        let new_content = content.replacen(&edit_input.old_str, &edit_input.new_str, 1);

        tokio::fs::write(&full_path, &new_content).await.map_err(|e| {
            crate::TheasusError::Tool {
                tool: "file_edit".to_string(),
                reason: format!("Failed to write file: {}", e),
            }
        })?;

        Ok(ToolResult::success(format!("Successfully edited {}", full_path.display())))
    }
}

impl Default for FileEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio::fs;

    fn test_context(cwd: PathBuf) -> ToolContext {
        ToolContext { cwd, session_id: uuid::Uuid::new_v4(), user_id: None }
    }

    #[tokio::test]
    async fn test_file_edit_single_match() {
        let temp_dir = std::env::current_dir().unwrap().join("test_edit_temp");
        fs::create_dir_all(&temp_dir).await.unwrap();
        let test_file = temp_dir.join("test_single.txt");

        fs::write(&test_file, "hello world\nfoo bar\n").await.unwrap();

        let tool = FileEditTool::new();
        let context = test_context(temp_dir.clone());

        let result = tool
            .execute(
                serde_json::json!({
                    "path": "test_single.txt",
                    "old_str": "foo bar",
                    "new_str": "baz qux"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(result.success);

        let content = fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "hello world\nbaz qux\n");

        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_file_edit_no_match() {
        let temp_dir = std::env::current_dir().unwrap().join("test_edit_temp2");
        fs::create_dir_all(&temp_dir).await.unwrap();
        let test_file = temp_dir.join("test_no_match.txt");

        fs::write(&test_file, "hello world\n").await.unwrap();

        let tool = FileEditTool::new();
        let context = test_context(temp_dir.clone());

        let result = tool
            .execute(
                serde_json::json!({
                    "path": "test_no_match.txt",
                    "old_str": "nonexistent",
                    "new_str": "replacement"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("No matches found"));

        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_file_edit_multiple_matches() {
        let temp_dir = std::env::current_dir().unwrap().join("test_edit_temp3");
        fs::create_dir_all(&temp_dir).await.unwrap();
        let test_file = temp_dir.join("test_multi.txt");

        fs::write(&test_file, "foo foo foo\n").await.unwrap();

        let tool = FileEditTool::new();
        let context = test_context(temp_dir.clone());

        let result = tool
            .execute(
                serde_json::json!({
                    "path": "test_multi.txt",
                    "old_str": "foo",
                    "new_str": "bar"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("3 matches"));

        fs::remove_dir_all(&temp_dir).await.ok();
    }
}
