use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashInput {
    pub command: String,
    pub timeout_secs: Option<u64>,
    pub background: Option<bool>,
    pub description: Option<String>,
}

pub struct BashTool;

impl BashTool {
    pub fn new() -> Self {
        Self
    }

    async fn execute_impl(
        &self,
        command: &str,
        cwd: &std::path::Path,
        timeout: Option<u64>,
        background: bool,
    ) -> crate::Result<ToolResult> {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command).current_dir(cwd);

        if background {
            cmd.stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        } else {
            cmd.stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
        }

        let output = if let Some(timeout) = timeout {
            match tokio::time::timeout(std::time::Duration::from_secs(timeout), cmd.output()).await
            {
                Ok(Ok(output)) => output,
                Ok(Err(e)) => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Command execution error: {}", e)),
                    });
                }
                Err(_) => {
                    return Ok(ToolResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Command timed out after {} seconds", timeout)),
                    });
                }
            }
        } else {
            cmd.output().await.map_err(|e| crate::TheasusError::Tool {
                tool: "bash".to_string(),
                reason: e.to_string(),
            })?
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(ToolResult {
            success: output.status.success(),
            output: stdout,
            error: if stderr.is_empty() {
                None
            } else {
                Some(stderr)
            },
        })
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "bash".to_string(),
            description: "Execute a shell command".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    },
                    "timeout_secs": {
                        "type": "number",
                        "description": "Timeout in seconds (optional)"
                    },
                    "background": {
                        "type": "boolean",
                        "description": "Run in background (optional)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> crate::Result<ToolResult> {
        let bash_input: BashInput =
            serde_json::from_value(input).map_err(|e| crate::TheasusError::Tool {
                tool: "bash".to_string(),
                reason: format!("Invalid input: {}", e),
            })?;

        self.execute_impl(
            &bash_input.command,
            &context.cwd,
            bash_input.timeout_secs,
            bash_input.background.unwrap_or(false),
        )
        .await
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}
