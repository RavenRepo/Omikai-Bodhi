//! Agent tool for spawning sub-agents.
//!
//! Allows spawning specialized agents to handle sub-tasks, either synchronously
//! or in the background.

use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use theasus_agents::{AgentContext, AgentRegistry};
use theasus_core::Result;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    /// Name of the agent to spawn (e.g., "explore", "task", "code-review")
    pub agent_name: String,
    /// Query or task for the agent to execute
    pub query: String,
    /// Whether to run in background (returns task_id immediately)
    #[serde(default)]
    pub background: bool,
}

/// Tracks background agent tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundTask {
    pub id: Uuid,
    pub agent_name: String,
    pub query: String,
    pub status: BackgroundTaskStatus,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackgroundTaskStatus {
    Running,
    Completed,
    Failed,
}

pub struct AgentTool {
    registry: Arc<AgentRegistry>,
    background_tasks: Arc<RwLock<HashMap<Uuid, BackgroundTask>>>,
}

impl AgentTool {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(AgentRegistry::new()),
            background_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_registry(registry: Arc<AgentRegistry>) -> Self {
        Self {
            registry,
            background_tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_task(&self, task_id: Uuid) -> Option<BackgroundTask> {
        self.background_tasks.read().await.get(&task_id).cloned()
    }

    pub async fn list_tasks(&self) -> Vec<BackgroundTask> {
        self.background_tasks.read().await.values().cloned().collect()
    }
}

impl Default for AgentTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AgentTool {
    fn name(&self) -> &str {
        "agent"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "agent".to_string(),
            description: "Spawn a sub-agent to handle a specific task. Available agents: explore (codebase exploration), task (command execution), code-review (code analysis), general-purpose (complex tasks), plan (planning), verification (testing).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "agent_name": {
                        "type": "string",
                        "description": "Name of the agent to spawn",
                        "enum": ["explore", "task", "code-review", "general-purpose", "plan", "verification"]
                    },
                    "query": {
                        "type": "string",
                        "description": "The query or task for the agent to execute"
                    },
                    "background": {
                        "type": "boolean",
                        "description": "Run in background and return task_id immediately (default: false)",
                        "default": false
                    }
                },
                "required": ["agent_name", "query"]
            }),
        }
    }

    async fn execute(&self, input: serde_json::Value, context: &ToolContext) -> Result<ToolResult> {
        let agent_input: AgentInput =
            serde_json::from_value(input).map_err(|e| theasus_core::TheasusError::Tool {
                tool: "agent".to_string(),
                reason: format!("Invalid input: {}", e),
            })?;

        let agent = self.registry.get(&agent_input.agent_name).ok_or_else(|| {
            theasus_core::TheasusError::Tool {
                tool: "agent".to_string(),
                reason: format!(
                    "Agent not found: {}. Available agents: {:?}",
                    agent_input.agent_name,
                    self.registry.names()
                ),
            }
        })?;

        // Create agent context from tool context
        let tool_registry = Arc::new(crate::ToolRegistry::new());
        let agent_context = AgentContext::new(context.cwd.clone(), tool_registry);

        if agent_input.background {
            // Background execution - spawn task and return immediately
            let task_id = Uuid::new_v4();
            let task = BackgroundTask {
                id: task_id,
                agent_name: agent_input.agent_name.clone(),
                query: agent_input.query.clone(),
                status: BackgroundTaskStatus::Running,
                result: None,
                error: None,
            };

            self.background_tasks.write().await.insert(task_id, task);

            // Spawn background task
            let tasks = self.background_tasks.clone();
            let query = agent_input.query.clone();
            tokio::spawn(async move {
                let result = agent.execute(&query, &agent_context).await;

                let mut tasks = tasks.write().await;
                if let Some(task) = tasks.get_mut(&task_id) {
                    match result {
                        Ok(r) => {
                            task.status = BackgroundTaskStatus::Completed;
                            task.result = Some(r.output);
                        }
                        Err(e) => {
                            task.status = BackgroundTaskStatus::Failed;
                            task.error = Some(e.to_string());
                        }
                    }
                }
            });

            Ok(ToolResult::success(format!(
                "Agent '{}' started in background. Task ID: {}",
                agent_input.agent_name, task_id
            )))
        } else {
            // Synchronous execution
            match agent.execute(&agent_input.query, &agent_context).await {
                Ok(result) => {
                    if result.success {
                        Ok(ToolResult::success(result.output))
                    } else {
                        Ok(ToolResult::error(result.output))
                    }
                }
                Err(e) => Ok(ToolResult::error(format!("Agent execution failed: {}", e))),
            }
        }
    }
}
