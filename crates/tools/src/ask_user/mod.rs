use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskUserInput {
    pub question: String,
    pub choices: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskUserOutput {
    pub requires_user_input: bool,
    pub question: String,
    pub choices: Option<Vec<String>>,
}

pub struct AskUserTool;

impl AskUserTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for AskUserTool {
    fn name(&self) -> &str {
        "ask_user"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "ask_user".to_string(),
            description: "Ask the user a question and wait for their response".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The question to ask the user"
                    },
                    "choices": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional list of choices for the user to select from"
                    }
                },
                "required": ["question"]
            }),
        }
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> crate::Result<ToolResult> {
        let ask_input: AskUserInput =
            serde_json::from_value(input).map_err(|e| crate::TheasusError::Tool {
                tool: "ask_user".to_string(),
                reason: format!("Invalid input: {}", e),
            })?;

        let output = AskUserOutput {
            requires_user_input: true,
            question: ask_input.question,
            choices: ask_input.choices,
        };

        let output_json =
            serde_json::to_string_pretty(&output).map_err(|e| crate::TheasusError::Tool {
                tool: "ask_user".to_string(),
                reason: format!("Failed to serialize output: {}", e),
            })?;

        Ok(ToolResult::success(output_json))
    }
}

impl Default for AskUserTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_context() -> ToolContext {
        ToolContext { cwd: PathBuf::from("."), session_id: uuid::Uuid::new_v4(), user_id: None }
    }

    #[tokio::test]
    async fn test_ask_user_simple_question() {
        let tool = AskUserTool::new();
        let context = test_context();

        let result = tool
            .execute(
                serde_json::json!({
                    "question": "What is your favorite color?"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(result.success);
        let output: AskUserOutput = serde_json::from_str(&result.output).unwrap();
        assert!(output.requires_user_input);
        assert_eq!(output.question, "What is your favorite color?");
        assert!(output.choices.is_none());
    }

    #[tokio::test]
    async fn test_ask_user_with_choices() {
        let tool = AskUserTool::new();
        let context = test_context();

        let result = tool
            .execute(
                serde_json::json!({
                    "question": "Select your preferred theme:",
                    "choices": ["dark", "light", "system"]
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(result.success);
        let output: AskUserOutput = serde_json::from_str(&result.output).unwrap();
        assert!(output.requires_user_input);
        assert_eq!(output.question, "Select your preferred theme:");
        assert_eq!(
            output.choices,
            Some(vec!["dark".to_string(), "light".to_string(), "system".to_string()])
        );
    }
}
