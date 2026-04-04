//! Workflow DSL Engine
//!
//! Provides a declarative workflow system for automating multi-step tasks.
//! Workflows can be defined in YAML/JSON and include tools, agents, conditions,
//! and parallel execution.

use futures::future::{join_all, BoxFuture, FutureExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use theasus_agents::{AgentContext, AgentRegistry};
use theasus_core::Result;
use theasus_tools::ToolRegistry;
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Workflow Definition
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub version: String,
    pub steps: Vec<WorkflowStep>,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    #[serde(default)]
    pub inputs: Vec<WorkflowInput>,
    #[serde(default)]
    pub outputs: Vec<WorkflowOutput>,
}

impl Workflow {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            version: "1.0.0".to_string(),
            steps: vec![],
            triggers: vec![],
            inputs: vec![],
            outputs: vec![],
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn add_step(mut self, step: WorkflowStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn add_trigger(mut self, trigger: Trigger) -> Self {
        self.triggers.push(trigger);
        self
    }

    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).map_err(|e| {
            theasus_core::TheasusError::Other(format!("Invalid workflow YAML: {}", e))
        })
    }

    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| {
            theasus_core::TheasusError::Other(format!("Invalid workflow JSON: {}", e))
        })
    }

    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(self)
            .map_err(|e| theasus_core::TheasusError::Other(format!("YAML serialization failed: {}", e)))
    }
}

// ============================================================================
// Workflow Steps
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowStep {
    Tool {
        id: Option<String>,
        name: String,
        #[serde(default)]
        args: serde_json::Value,
        #[serde(default)]
        retry: Option<RetryConfig>,
    },
    Agent {
        id: Option<String>,
        name: String,
        query: String,
        #[serde(default)]
        timeout_secs: Option<u64>,
    },
    Condition {
        id: Option<String>,
        #[serde(rename = "if")]
        condition: String,
        then: Box<WorkflowStep>,
        #[serde(rename = "else")]
        else_step: Option<Box<WorkflowStep>>,
    },
    Parallel {
        id: Option<String>,
        steps: Vec<WorkflowStep>,
        #[serde(default)]
        fail_fast: bool,
    },
    Loop {
        id: Option<String>,
        items: String,
        item_var: String,
        step: Box<WorkflowStep>,
    },
    SetVariable {
        name: String,
        value: serde_json::Value,
    },
    Log {
        message: String,
        #[serde(default)]
        level: LogLevel,
    },
    Delay {
        seconds: u64,
    },
}

impl WorkflowStep {
    pub fn tool(name: impl Into<String>, args: serde_json::Value) -> Self {
        Self::Tool {
            id: None,
            name: name.into(),
            args,
            retry: None,
        }
    }

    pub fn agent(name: impl Into<String>, query: impl Into<String>) -> Self {
        Self::Agent {
            id: None,
            name: name.into(),
            query: query.into(),
            timeout_secs: None,
        }
    }

    pub fn parallel(steps: Vec<WorkflowStep>) -> Self {
        Self::Parallel {
            id: None,
            steps,
            fail_fast: false,
        }
    }

    pub fn condition(cond: impl Into<String>, then: WorkflowStep) -> Self {
        Self::Condition {
            id: None,
            condition: cond.into(),
            then: Box::new(then),
            else_step: None,
        }
    }

    pub fn get_id(&self) -> Option<&str> {
        match self {
            Self::Tool { id, .. } => id.as_deref(),
            Self::Agent { id, .. } => id.as_deref(),
            Self::Condition { id, .. } => id.as_deref(),
            Self::Parallel { id, .. } => id.as_deref(),
            Self::Loop { id, .. } => id.as_deref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    #[serde(default = "default_retry_delay")]
    pub delay_secs: u64,
    #[serde(default)]
    pub exponential_backoff: bool,
}

fn default_retry_delay() -> u64 {
    1
}

// ============================================================================
// Triggers
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Trigger {
    Manual,
    FileChange {
        patterns: Vec<String>,
    },
    Schedule {
        cron: String,
    },
    Webhook {
        path: String,
        method: String,
    },
    Event {
        name: String,
    },
}

// ============================================================================
// Inputs/Outputs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOutput {
    pub name: String,
    pub value: String,
}

// ============================================================================
// Execution Context
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct WorkflowContext {
    pub run_id: Uuid,
    pub variables: HashMap<String, serde_json::Value>,
    pub step_results: HashMap<String, StepResult>,
    pub cwd: PathBuf,
}

impl WorkflowContext {
    pub fn new() -> Self {
        Self {
            run_id: Uuid::new_v4(),
            variables: HashMap::new(),
            step_results: HashMap::new(),
            cwd: std::env::current_dir().unwrap_or_default(),
        }
    }

    pub fn with_input(mut self, name: impl Into<String>, value: serde_json::Value) -> Self {
        self.variables.insert(name.into(), value);
        self
    }

    pub fn get_var(&self, name: &str) -> Option<&serde_json::Value> {
        self.variables.get(name)
    }

    pub fn set_var(&mut self, name: impl Into<String>, value: serde_json::Value) {
        self.variables.insert(name.into(), value);
    }

    pub fn get_step_result(&self, step_id: &str) -> Option<&StepResult> {
        self.step_results.get(step_id)
    }

    fn evaluate_condition(&self, condition: &str) -> bool {
        // Simple expression evaluation
        // Format: "steps.step_id.success" or "vars.var_name == value"
        if condition.starts_with("steps.") {
            let parts: Vec<&str> = condition.split('.').collect();
            if parts.len() >= 3 {
                let step_id = parts[1];
                let field = parts[2];
                if let Some(result) = self.step_results.get(step_id) {
                    return match field {
                        "success" => result.success,
                        _ => false,
                    };
                }
            }
        } else if condition.starts_with("vars.") {
            let parts: Vec<&str> = condition.splitn(2, '.').collect();
            if parts.len() >= 2 {
                return self.variables.contains_key(parts[1]);
            }
        } else if condition == "true" {
            return true;
        } else if condition == "false" {
            return false;
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_id: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub duration_ms: u64,
    #[serde(default)]
    pub error: Option<String>,
}

// ============================================================================
// Workflow Execution Result
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub run_id: Uuid,
    pub workflow_name: String,
    pub success: bool,
    pub step_results: Vec<StepResult>,
    pub outputs: HashMap<String, serde_json::Value>,
    pub duration_ms: u64,
    #[serde(default)]
    pub error: Option<String>,
}

// ============================================================================
// Workflow Executor
// ============================================================================

pub struct WorkflowExecutor {
    tool_registry: Arc<ToolRegistry>,
    agent_registry: Arc<AgentRegistry>,
}

impl WorkflowExecutor {
    pub fn new(tool_registry: Arc<ToolRegistry>, agent_registry: Arc<AgentRegistry>) -> Self {
        Self {
            tool_registry,
            agent_registry,
        }
    }

    pub async fn execute(&self, workflow: &Workflow, mut context: WorkflowContext) -> Result<WorkflowResult> {
        let start = std::time::Instant::now();
        tracing::info!("Starting workflow: {} (run_id: {})", workflow.name, context.run_id);

        let mut step_results = vec![];
        let mut success = true;
        let mut last_error = None;

        for (idx, step) in workflow.steps.iter().enumerate() {
            let step_id = step.get_id().map(String::from).unwrap_or_else(|| format!("step_{}", idx));

            match self.execute_step(step, &mut context).await {
                Ok(result) => {
                    if !result.success {
                        success = false;
                        last_error = result.error.clone();
                    }
                    context.step_results.insert(step_id, result.clone());
                    step_results.push(result);
                }
                Err(e) => {
                    success = false;
                    last_error = Some(e.to_string());
                    step_results.push(StepResult {
                        step_id,
                        success: false,
                        output: serde_json::Value::Null,
                        duration_ms: 0,
                        error: Some(e.to_string()),
                    });
                    break;
                }
            }
        }

        let outputs: HashMap<String, serde_json::Value> = workflow
            .outputs
            .iter()
            .filter_map(|o| {
                context.get_var(&o.value).map(|v| (o.name.clone(), v.clone()))
            })
            .collect();

        Ok(WorkflowResult {
            run_id: context.run_id,
            workflow_name: workflow.name.clone(),
            success,
            step_results,
            outputs,
            duration_ms: start.elapsed().as_millis() as u64,
            error: last_error,
        })
    }

    fn execute_step<'a>(
        &'a self,
        step: &'a WorkflowStep,
        context: &'a mut WorkflowContext,
    ) -> BoxFuture<'a, Result<StepResult>> {
        async move {
            let start = std::time::Instant::now();
            let step_id = step.get_id().map(String::from).unwrap_or_else(|| Uuid::new_v4().to_string());

        match step {
            WorkflowStep::Tool { name, args, retry, .. } => {
                let max_attempts = retry.as_ref().map(|r| r.max_attempts).unwrap_or(1);
                let mut last_error = None;

                for attempt in 0..max_attempts {
                    if attempt > 0 {
                        let delay = retry.as_ref().map(|r| {
                            if r.exponential_backoff {
                                r.delay_secs * (1 << attempt.min(5))
                            } else {
                                r.delay_secs
                            }
                        }).unwrap_or(1);
                        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                    }

                    match self.tool_registry.execute(name, args.clone()).await {
                        Ok(result) => {
                            return Ok(StepResult {
                                step_id,
                                success: result.success,
                                output: serde_json::json!({"output": result.output}),
                                duration_ms: start.elapsed().as_millis() as u64,
                                error: result.error,
                            });
                        }
                        Err(e) => {
                            last_error = Some(e.to_string());
                        }
                    }
                }

                Ok(StepResult {
                    step_id,
                    success: false,
                    output: serde_json::Value::Null,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: last_error,
                })
            }

            WorkflowStep::Agent { name, query, timeout_secs, .. } => {
                let agent = self.agent_registry.get(name).ok_or_else(|| {
                    theasus_core::TheasusError::Other(format!("Agent not found: {}", name))
                })?;

                let agent_context = AgentContext::new(context.cwd.clone(), self.tool_registry.clone());

                let result = if let Some(timeout) = timeout_secs {
                    tokio::time::timeout(
                        std::time::Duration::from_secs(*timeout),
                        agent.execute(query, &agent_context),
                    )
                    .await
                    .map_err(|_| theasus_core::TheasusError::Other("Agent timeout".to_string()))?
                } else {
                    agent.execute(query, &agent_context).await
                };

                match result {
                    Ok(agent_result) => Ok(StepResult {
                        step_id,
                        success: agent_result.success,
                        output: serde_json::json!({"output": agent_result.output}),
                        duration_ms: start.elapsed().as_millis() as u64,
                        error: if agent_result.success { None } else { Some(agent_result.output) },
                    }),
                    Err(e) => Ok(StepResult {
                        step_id,
                        success: false,
                        output: serde_json::Value::Null,
                        duration_ms: start.elapsed().as_millis() as u64,
                        error: Some(e.to_string()),
                    }),
                }
            }

            WorkflowStep::Condition { condition, then, else_step, .. } => {
                let result = if context.evaluate_condition(condition) {
                    self.execute_step(then, context).await?
                } else if let Some(else_s) = else_step {
                    self.execute_step(else_s, context).await?
                } else {
                    StepResult {
                        step_id: step_id.clone(),
                        success: true,
                        output: serde_json::json!({"skipped": true}),
                        duration_ms: start.elapsed().as_millis() as u64,
                        error: None,
                    }
                };
                Ok(result)
            }

            WorkflowStep::Parallel { steps, fail_fast, .. } => {
                let futures: Vec<_> = steps.iter().map(|s| {
                    let mut ctx = context.clone();
                    async move { self.execute_step(s, &mut ctx).await }
                }).collect();

                let results = join_all(futures).await;
                let mut all_success = true;
                let mut outputs = vec![];

                for result in results {
                    match result {
                        Ok(r) => {
                            if !r.success {
                                all_success = false;
                                if *fail_fast {
                                    break;
                                }
                            }
                            outputs.push(r);
                        }
                        Err(e) => {
                            all_success = false;
                            outputs.push(StepResult {
                                step_id: "parallel_step".to_string(),
                                success: false,
                                output: serde_json::Value::Null,
                                duration_ms: 0,
                                error: Some(e.to_string()),
                            });
                            if *fail_fast {
                                break;
                            }
                        }
                    }
                }

                Ok(StepResult {
                    step_id,
                    success: all_success,
                    output: serde_json::to_value(&outputs).unwrap_or_default(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: None,
                })
            }

            WorkflowStep::Loop { items, item_var, step: loop_step, .. } => {
                let items_val = context.get_var(items).cloned().unwrap_or(serde_json::Value::Array(vec![]));
                let items_array = items_val.as_array().cloned().unwrap_or_default();

                let mut all_success = true;
                let mut outputs = vec![];

                for item in items_array {
                    context.set_var(item_var.clone(), item);
                    let result = self.execute_step(loop_step, context).await?;
                    if !result.success {
                        all_success = false;
                    }
                    outputs.push(result);
                }

                Ok(StepResult {
                    step_id,
                    success: all_success,
                    output: serde_json::to_value(&outputs).unwrap_or_default(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: None,
                })
            }

            WorkflowStep::SetVariable { name, value } => {
                context.set_var(name.clone(), value.clone());
                Ok(StepResult {
                    step_id,
                    success: true,
                    output: value.clone(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: None,
                })
            }

            WorkflowStep::Log { message, level } => {
                match level {
                    LogLevel::Debug => tracing::debug!("{}", message),
                    LogLevel::Info => tracing::info!("{}", message),
                    LogLevel::Warn => tracing::warn!("{}", message),
                    LogLevel::Error => tracing::error!("{}", message),
                }
                Ok(StepResult {
                    step_id,
                    success: true,
                    output: serde_json::json!({"message": message}),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: None,
                })
            }

            WorkflowStep::Delay { seconds } => {
                tokio::time::sleep(std::time::Duration::from_secs(*seconds)).await;
                Ok(StepResult {
                    step_id,
                    success: true,
                    output: serde_json::json!({"delayed_seconds": seconds}),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: None,
                })
            }
        }
        }.boxed()
    }
}

// ============================================================================
// Workflow Registry
// ============================================================================

pub struct WorkflowRegistry {
    workflows: Arc<RwLock<HashMap<String, Workflow>>>,
}

impl WorkflowRegistry {
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, workflow: Workflow) {
        self.workflows.write().await.insert(workflow.name.clone(), workflow);
    }

    pub async fn get(&self, name: &str) -> Option<Workflow> {
        self.workflows.read().await.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<String> {
        self.workflows.read().await.keys().cloned().collect()
    }

    pub async fn load_from_dir(&self, dir: &PathBuf) -> Result<usize> {
        let mut count = 0;

        if !dir.exists() {
            return Ok(0);
        }

        for entry in std::fs::read_dir(dir)?.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false) {
                let content = std::fs::read_to_string(&path)?;
                if let Ok(workflow) = Workflow::from_yaml(&content) {
                    self.register(workflow).await;
                    count += 1;
                }
            } else if path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = std::fs::read_to_string(&path)?;
                if let Ok(workflow) = Workflow::from_json(&content) {
                    self.register(workflow).await;
                    count += 1;
                }
            }
        }

        Ok(count)
    }
}

impl Default for WorkflowRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    NotFound(String),

    #[error("Step failed: {step_id} - {message}")]
    StepFailed { step_id: String, message: String },

    #[error("Condition evaluation failed: {0}")]
    ConditionFailed(String),

    #[error("Workflow timeout")]
    Timeout,

    #[error("Invalid workflow definition: {0}")]
    InvalidDefinition(String),
}

pub type WorkflowResultT<T> = std::result::Result<T, WorkflowError>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_yaml_parsing() {
        let yaml = r#"
name: test-workflow
description: A test workflow
steps:
  - type: log
    message: "Hello, World!"
    level: info
  - type: set_variable
    name: result
    value: "success"
"#;
        let workflow = Workflow::from_yaml(yaml).unwrap();
        assert_eq!(workflow.name, "test-workflow");
        assert_eq!(workflow.steps.len(), 2);
    }

    #[test]
    fn test_workflow_builder() {
        let workflow = Workflow::new("my-workflow")
            .with_description("Test workflow")
            .add_step(WorkflowStep::Log {
                message: "Starting".to_string(),
                level: LogLevel::Info,
            });

        assert_eq!(workflow.name, "my-workflow");
        assert_eq!(workflow.steps.len(), 1);
    }

    #[test]
    fn test_workflow_context() {
        let mut context = WorkflowContext::new()
            .with_input("foo", serde_json::json!("bar"));

        assert_eq!(context.get_var("foo"), Some(&serde_json::json!("bar")));

        context.set_var("baz", serde_json::json!(42));
        assert_eq!(context.get_var("baz"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn test_condition_evaluation() {
        let mut context = WorkflowContext::new();
        context.set_var("enabled", serde_json::json!(true));
        context.step_results.insert(
            "step1".to_string(),
            StepResult {
                step_id: "step1".to_string(),
                success: true,
                output: serde_json::Value::Null,
                duration_ms: 0,
                error: None,
            },
        );

        assert!(context.evaluate_condition("true"));
        assert!(!context.evaluate_condition("false"));
        assert!(context.evaluate_condition("vars.enabled"));
        assert!(context.evaluate_condition("steps.step1.success"));
    }

    #[test]
    fn test_parallel_step() {
        let step = WorkflowStep::parallel(vec![
            WorkflowStep::Log { message: "A".to_string(), level: LogLevel::Info },
            WorkflowStep::Log { message: "B".to_string(), level: LogLevel::Info },
        ]);

        if let WorkflowStep::Parallel { steps, .. } = step {
            assert_eq!(steps.len(), 2);
        } else {
            panic!("Expected Parallel step");
        }
    }
}
