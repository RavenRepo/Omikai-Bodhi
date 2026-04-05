//! Task management tool for tracking and managing background tasks.
//!
//! Provides operations to list, get status of, and cancel running background tasks.

use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use theasus_core::Result;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInput {
    /// Action to perform: list, get, or cancel
    pub action: TaskAction,
    /// Task ID (required for get and cancel actions)
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskAction {
    List,
    Get,
    Cancel,
}

/// Information about a managed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: TaskState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Shared task registry for tracking background tasks across tools.
pub struct TaskRegistry {
    tasks: Arc<RwLock<HashMap<Uuid, TaskInfo>>>,
    cancel_signals: Arc<RwLock<HashMap<Uuid, tokio::sync::watch::Sender<bool>>>>,
}

impl TaskRegistry {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            cancel_signals: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_task(
        &self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> (Uuid, tokio::sync::watch::Receiver<bool>) {
        let id = Uuid::new_v4();
        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

        let task = TaskInfo {
            id,
            name: name.into(),
            description: description.into(),
            status: TaskState::Pending,
            created_at: chrono::Utc::now(),
            completed_at: None,
            result: None,
            error: None,
        };

        self.tasks.write().await.insert(id, task);
        self.cancel_signals.write().await.insert(id, cancel_tx);

        (id, cancel_rx)
    }

    pub async fn update_status(&self, id: Uuid, status: TaskState) {
        if let Some(task) = self.tasks.write().await.get_mut(&id) {
            task.status = status.clone();
            if matches!(
                status,
                TaskState::Completed | TaskState::Failed | TaskState::Cancelled
            ) {
                task.completed_at = Some(chrono::Utc::now());
            }
        }
    }

    pub async fn complete_task(&self, id: Uuid, result: String) {
        if let Some(task) = self.tasks.write().await.get_mut(&id) {
            task.status = TaskState::Completed;
            task.completed_at = Some(chrono::Utc::now());
            task.result = Some(result);
        }
        self.cancel_signals.write().await.remove(&id);
    }

    pub async fn fail_task(&self, id: Uuid, error: String) {
        if let Some(task) = self.tasks.write().await.get_mut(&id) {
            task.status = TaskState::Failed;
            task.completed_at = Some(chrono::Utc::now());
            task.error = Some(error);
        }
        self.cancel_signals.write().await.remove(&id);
    }

    pub async fn cancel_task(&self, id: Uuid) -> bool {
        let signals = self.cancel_signals.read().await;
        if let Some(tx) = signals.get(&id) {
            let _ = tx.send(true);
            drop(signals);
            self.update_status(id, TaskState::Cancelled).await;
            true
        } else {
            false
        }
    }

    pub async fn get_task(&self, id: Uuid) -> Option<TaskInfo> {
        self.tasks.read().await.get(&id).cloned()
    }

    pub async fn list_tasks(&self) -> Vec<TaskInfo> {
        self.tasks.read().await.values().cloned().collect()
    }

    pub async fn list_running_tasks(&self) -> Vec<TaskInfo> {
        self.tasks
            .read()
            .await
            .values()
            .filter(|t| matches!(t.status, TaskState::Pending | TaskState::Running))
            .cloned()
            .collect()
    }
}

impl Default for TaskRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TaskTool {
    registry: Arc<TaskRegistry>,
}

impl TaskTool {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(TaskRegistry::new()),
        }
    }

    pub fn with_registry(registry: Arc<TaskRegistry>) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> Arc<TaskRegistry> {
        self.registry.clone()
    }
}

impl Default for TaskTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &str {
        "task"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "task".to_string(),
            description: "Manage background tasks. List running tasks, get task status and results, or cancel running tasks.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Action to perform",
                        "enum": ["list", "get", "cancel"]
                    },
                    "task_id": {
                        "type": "string",
                        "description": "Task ID (required for get and cancel actions)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, input: serde_json::Value, _context: &ToolContext) -> Result<ToolResult> {
        let task_input: TaskInput =
            serde_json::from_value(input).map_err(|e| theasus_core::TheasusError::Tool {
                tool: "task".to_string(),
                reason: format!("Invalid input: {}", e),
            })?;

        match task_input.action {
            TaskAction::List => {
                let tasks = self.registry.list_tasks().await;
                if tasks.is_empty() {
                    Ok(ToolResult::success("No tasks found."))
                } else {
                    let output = tasks
                        .iter()
                        .map(|t| {
                            format!(
                                "- {} ({}): {} [{:?}]",
                                t.id, t.name, t.description, t.status
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    Ok(ToolResult::success(format!(
                        "Tasks ({}):\n{}",
                        tasks.len(),
                        output
                    )))
                }
            }
            TaskAction::Get => {
                let task_id_str = task_input.task_id.ok_or_else(|| {
                    theasus_core::TheasusError::Tool {
                        tool: "task".to_string(),
                        reason: "task_id is required for get action".to_string(),
                    }
                })?;

                let task_id = Uuid::parse_str(&task_id_str).map_err(|e| {
                    theasus_core::TheasusError::Tool {
                        tool: "task".to_string(),
                        reason: format!("Invalid task_id: {}", e),
                    }
                })?;

                match self.registry.get_task(task_id).await {
                    Some(task) => {
                        let mut output = format!(
                            "Task: {}\nName: {}\nDescription: {}\nStatus: {:?}\nCreated: {}",
                            task.id, task.name, task.description, task.status, task.created_at
                        );

                        if let Some(completed) = task.completed_at {
                            output.push_str(&format!("\nCompleted: {}", completed));
                        }
                        if let Some(result) = &task.result {
                            output.push_str(&format!("\nResult: {}", result));
                        }
                        if let Some(error) = &task.error {
                            output.push_str(&format!("\nError: {}", error));
                        }

                        Ok(ToolResult::success(output))
                    }
                    None => Ok(ToolResult::error(format!("Task not found: {}", task_id))),
                }
            }
            TaskAction::Cancel => {
                let task_id_str = task_input.task_id.ok_or_else(|| {
                    theasus_core::TheasusError::Tool {
                        tool: "task".to_string(),
                        reason: "task_id is required for cancel action".to_string(),
                    }
                })?;

                let task_id = Uuid::parse_str(&task_id_str).map_err(|e| {
                    theasus_core::TheasusError::Tool {
                        tool: "task".to_string(),
                        reason: format!("Invalid task_id: {}", e),
                    }
                })?;

                if self.registry.cancel_task(task_id).await {
                    Ok(ToolResult::success(format!(
                        "Task {} cancellation requested.",
                        task_id
                    )))
                } else {
                    Ok(ToolResult::error(format!(
                        "Task {} not found or already completed.",
                        task_id
                    )))
                }
            }
        }
    }
}
