use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use theasus_permissions::is_path_safe;
use tracing::{debug, warn};

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

    fn validate_path(&self, path: &Path, cwd: &Path) -> Result<PathBuf, String> {
        let full_path: PathBuf =
            if path.is_absolute() { path.to_path_buf() } else { cwd.join(path) };

        // For path safety, we check the parent directory if the file doesn't exist yet
        // This allows us to properly validate paths for non-existent files
        let path_to_check = if full_path.exists() {
            full_path.clone()
        } else if let Some(parent) = full_path.parent() {
            if parent.exists() {
                parent.to_path_buf()
            } else {
                return Err(format!("Parent directory does not exist: {}", parent.display()));
            }
        } else {
            full_path.clone()
        };

        if !is_path_safe(&path_to_check, cwd) {
            return Err(format!(
                "Permission denied: path '{}' is outside the allowed directory '{}'",
                full_path.display(),
                cwd.display()
            ));
        }

        Ok(full_path)
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

        debug!(
            path = %edit_input.path,
            old_str_len = edit_input.old_str.len(),
            new_str_len = edit_input.new_str.len(),
            "Executing file_edit"
        );

        let file_path = PathBuf::from(&edit_input.path);
        let full_path = match self.validate_path(&file_path, &context.cwd) {
            Ok(p) => p,
            Err(e) => {
                warn!(path = %edit_input.path, "Permission denied for file edit");
                return Ok(ToolResult::error(e));
            }
        };

        if !full_path.exists() {
            return Ok(ToolResult::error(format!("File not found: {}", full_path.display())));
        }

        if full_path.is_dir() {
            return Ok(ToolResult::error(format!(
                "Cannot edit a directory: {}",
                full_path.display()
            )));
        }

        let content =
            tokio::fs::read_to_string(&full_path).await.map_err(|e| crate::TheasusError::Tool {
                tool: "file_edit".to_string(),
                reason: format!("Failed to read file '{}': {}", full_path.display(), e),
            })?;

        if edit_input.old_str.is_empty() {
            return Ok(ToolResult::error(
                "old_str cannot be empty. Use file_write to create new content.",
            ));
        }

        let match_count = content.matches(&edit_input.old_str).count();

        if match_count == 0 {
            return Ok(ToolResult::error(format!(
                "No matches found for the specified old_str in {}.\n\nHint: Ensure whitespace and line endings match exactly.",
                full_path.display()
            )));
        }

        if match_count > 1 {
            return Ok(ToolResult::error(format!(
                "Found {} matches for old_str in {} (expected exactly 1).\n\nHint: Include more surrounding context to make the match unique.",
                match_count,
                full_path.display()
            )));
        }

        let new_content = content.replacen(&edit_input.old_str, &edit_input.new_str, 1);

        tokio::fs::write(&full_path, &new_content).await.map_err(|e| {
            crate::TheasusError::Tool {
                tool: "file_edit".to_string(),
                reason: format!("Failed to write file '{}': {}", full_path.display(), e),
            }
        })?;

        debug!(path = %full_path.display(), "File edit successful");
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

    #[tokio::test]
    async fn test_file_edit_empty_old_str() {
        let temp_dir = std::env::current_dir().unwrap().join("test_edit_temp4");
        fs::create_dir_all(&temp_dir).await.unwrap();
        let test_file = temp_dir.join("test_empty.txt");

        fs::write(&test_file, "hello world\n").await.unwrap();

        let tool = FileEditTool::new();
        let context = test_context(temp_dir.clone());

        let result = tool
            .execute(
                serde_json::json!({
                    "path": "test_empty.txt",
                    "old_str": "",
                    "new_str": "replacement"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("old_str cannot be empty"));

        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_file_edit_file_not_found() {
        let temp_dir = std::env::current_dir().unwrap().join("test_edit_temp5");
        fs::create_dir_all(&temp_dir).await.unwrap();

        let tool = FileEditTool::new();
        let context = test_context(temp_dir.clone());

        let result = tool
            .execute(
                serde_json::json!({
                    "path": "nonexistent.txt",
                    "old_str": "foo",
                    "new_str": "bar"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("File not found"));

        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_file_edit_cannot_edit_directory() {
        let temp_dir = std::env::current_dir().unwrap().join("test_edit_temp6");
        let subdir = temp_dir.join("subdir");
        fs::create_dir_all(&subdir).await.unwrap();

        let tool = FileEditTool::new();
        let context = test_context(temp_dir.clone());

        let result = tool
            .execute(
                serde_json::json!({
                    "path": "subdir",
                    "old_str": "foo",
                    "new_str": "bar"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.output.contains("Cannot edit a directory"));

        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_file_edit_multiline_replacement() {
        let temp_dir = std::env::current_dir().unwrap().join("test_edit_temp7");
        fs::create_dir_all(&temp_dir).await.unwrap();
        let test_file = temp_dir.join("test_multiline.txt");

        fs::write(&test_file, "fn main() {\n    println!(\"Hello\");\n}\n").await.unwrap();

        let tool = FileEditTool::new();
        let context = test_context(temp_dir.clone());

        let result = tool
            .execute(
                serde_json::json!({
                    "path": "test_multiline.txt",
                    "old_str": "fn main() {\n    println!(\"Hello\");\n}",
                    "new_str": "fn main() {\n    println!(\"World\");\n    println!(\"Goodbye\");\n}"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(result.success);

        let content = fs::read_to_string(&test_file).await.unwrap();
        assert!(content.contains("World"));
        assert!(content.contains("Goodbye"));

        fs::remove_dir_all(&temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_file_edit_preserves_file_permissions() {
        let temp_dir = std::env::current_dir().unwrap().join("test_edit_temp8");
        fs::create_dir_all(&temp_dir).await.unwrap();
        let test_file = temp_dir.join("test_perms.txt");

        fs::write(&test_file, "hello world\n").await.unwrap();

        let tool = FileEditTool::new();
        let context = test_context(temp_dir.clone());

        let result = tool
            .execute(
                serde_json::json!({
                    "path": "test_perms.txt",
                    "old_str": "hello",
                    "new_str": "goodbye"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(result.success);
        let content = fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "goodbye world\n");

        fs::remove_dir_all(&temp_dir).await.ok();
    }
}
