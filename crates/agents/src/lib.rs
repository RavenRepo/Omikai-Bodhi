//! # Theasus Multi-Agent System
//!
//! Provides a framework for orchestrating multiple AI agents with different
//! specializations. Each agent has its own system prompt and tool access,
//! allowing for complex task decomposition and parallel execution.
//!
//! ## Features
//!
//! - **LlmAgent**: Base agent that runs an LLM loop with tool calling
//! - **AgentOrchestrator**: Coordinates multiple agents with dependencies
//! - **Built-in Agents**: GeneralPurpose, Explore, Plan, Task, CodeReview
//!
//! ## Example
//!
//! ```rust,ignore
//! use theasus_agents::{AgentRegistry, AgentContext, LlmAgent};
//! use std::sync::Arc;
//!
//! // Create context with tools
//! let tool_registry = Arc::new(theasus_tools::ToolRegistry::new());
//! let context = AgentContext::new(std::env::current_dir()?, tool_registry);
//!
//! // Execute an agent
//! let agent = LlmAgent::explore();
//! let result = agent.execute("Find all Rust files", &context).await?;
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use theasus_core::{Message, Result, ToolCall};
use theasus_tools::{ToolDefinition, ToolRegistry, ToolResult};
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// LLM Provider Abstraction
// ============================================================================

#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub messages: Vec<Message>,
    pub system: Option<String>,
    pub tools: Vec<ToolDefinition>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub stop_reason: Option<String>,
    pub usage: theasus_core::Usage,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse>;
}

// ============================================================================
// Agent Definition and Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(default)]
    pub max_turns: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
}

impl AgentDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>, system_prompt: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            system_prompt: system_prompt.into(),
            allowed_tools: None,
            max_turns: Some(10),
            temperature: None,
        }
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = Some(tools);
        self
    }

    pub fn with_max_turns(mut self, turns: u32) -> Self {
        self.max_turns = Some(turns);
        self
    }
}

// ============================================================================
// Agent Result
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub success: bool,
    pub output: String,
    pub messages: Vec<Message>,
    pub tool_calls: Vec<ToolCall>,
    pub turns_used: u32,
}

impl AgentResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            messages: vec![],
            tool_calls: vec![],
            turns_used: 0,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            output: msg.into(),
            messages: vec![],
            tool_calls: vec![],
            turns_used: 0,
        }
    }

    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    pub fn with_tool_calls(mut self, calls: Vec<ToolCall>) -> Self {
        self.tool_calls = calls;
        self
    }

    pub fn with_turns(mut self, turns: u32) -> Self {
        self.turns_used = turns;
        self
    }
}

// ============================================================================
// Agent Context
// ============================================================================

pub struct AgentContext {
    pub session_id: Uuid,
    pub cwd: std::path::PathBuf,
    pub tool_registry: Arc<ToolRegistry>,
    pub llm_provider: Option<Arc<dyn LlmProvider>>,
    pub extra: HashMap<String, serde_json::Value>,
}

impl AgentContext {
    pub fn new(cwd: std::path::PathBuf, tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            cwd,
            tool_registry,
            llm_provider: None,
            extra: HashMap::new(),
        }
    }

    pub fn with_llm(mut self, provider: Arc<dyn LlmProvider>) -> Self {
        self.llm_provider = Some(provider);
        self
    }

    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }

    pub fn get_tools(&self, filter: Option<&[String]>) -> Vec<ToolDefinition> {
        let all_tools = self.tool_registry.list_tools();
        match filter {
            Some(allowed) => all_tools
                .into_iter()
                .filter(|t| allowed.contains(&t.name))
                .collect(),
            None => all_tools,
        }
    }

    pub async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<ToolResult> {
        self.tool_registry.execute(name, args).await
    }
}

// ============================================================================
// Agent Trait
// ============================================================================

#[async_trait]
pub trait Agent: Send + Sync {
    fn definition(&self) -> AgentDefinition;

    async fn execute(&self, query: &str, context: &AgentContext) -> Result<AgentResult>;

    fn supports_streaming(&self) -> bool {
        false
    }
}

// ============================================================================
// Base Agent with LLM Loop
// ============================================================================

pub struct LlmAgent {
    definition: AgentDefinition,
}

impl LlmAgent {
    pub fn new(definition: AgentDefinition) -> Self {
        Self { definition }
    }

    async fn run_loop(&self, query: &str, context: &AgentContext) -> Result<AgentResult> {
        let provider = context.llm_provider.as_ref().ok_or_else(|| {
            theasus_core::TheasusError::Other("No LLM provider configured".to_string())
        })?;

        let tools = context.get_tools(self.definition.allowed_tools.as_deref());
        let max_turns = self.definition.max_turns.unwrap_or(10);
        let mut messages = vec![Message::user(query)];
        let mut all_tool_calls = vec![];
        let mut turns = 0;

        loop {
            turns += 1;
            if turns > max_turns {
                return Ok(AgentResult::error("Max turns exceeded")
                    .with_messages(messages)
                    .with_tool_calls(all_tool_calls)
                    .with_turns(turns));
            }

            let request = LlmRequest {
                messages: messages.clone(),
                system: Some(self.definition.system_prompt.clone()),
                tools: tools.clone(),
                temperature: self.definition.temperature,
                max_tokens: None,
                stop_sequences: None,
            };

            let response = provider.complete(request).await?;

            // Add assistant response to history
            messages.push(Message::assistant(&response.text));

            // Check if there are tool calls
            if response.tool_calls.is_empty() {
                // No tool calls - agent is done
                return Ok(AgentResult::success(&response.text)
                    .with_messages(messages)
                    .with_tool_calls(all_tool_calls)
                    .with_turns(turns));
            }

            // Process tool calls
            for tool_call in &response.tool_calls {
                all_tool_calls.push(tool_call.clone());

                let result = context
                    .execute_tool(&tool_call.name, tool_call.input.clone())
                    .await;

                let tool_result = match result {
                    Ok(r) => r,
                    Err(e) => ToolResult::error(format!("Tool error: {}", e)),
                };

                // Add tool result to messages
                messages.push(Message::tool_result(&tool_call.id, &tool_result.output));
            }
        }
    }
}

#[async_trait]
impl Agent for LlmAgent {
    fn definition(&self) -> AgentDefinition {
        self.definition.clone()
    }

    async fn execute(&self, query: &str, context: &AgentContext) -> Result<AgentResult> {
        self.run_loop(query, context).await
    }
}

// ============================================================================
// Built-in Agents
// ============================================================================

pub struct GeneralPurposeAgent(LlmAgent);

impl GeneralPurposeAgent {
    pub fn new() -> Self {
        Self(LlmAgent::new(AgentDefinition::new(
            "general-purpose",
            "Full-capability agent for complex multi-step tasks",
            r#"You are a helpful AI assistant with access to tools for completing tasks.

Guidelines:
- Use tools when appropriate to gather information or make changes
- Think step by step for complex tasks
- Verify your work before reporting completion
- Be concise in your responses while being thorough in your work"#,
        )))
    }
}

impl Default for GeneralPurposeAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for GeneralPurposeAgent {
    fn definition(&self) -> AgentDefinition {
        self.0.definition()
    }

    async fn execute(&self, query: &str, context: &AgentContext) -> Result<AgentResult> {
        self.0.execute(query, context).await
    }
}

pub struct ExploreAgent(LlmAgent);

impl ExploreAgent {
    pub fn new() -> Self {
        Self(LlmAgent::new(
            AgentDefinition::new(
                "explore",
                "Fast agent for exploring codebases and answering questions",
                r#"You are an exploration agent specialized in understanding codebases.

Your role:
- Use glob to find files matching patterns
- Use grep to search for code patterns and symbols
- Use view to examine file contents
- Synthesize findings into clear, concise answers

Guidelines:
- Answer multi-part questions thoroughly
- When searching, start broad then narrow down
- Report file paths and line numbers for findings
- Summarize rather than dumping raw output"#,
            )
            .with_tools(vec!["glob".into(), "grep".into(), "view".into()])
            .with_max_turns(5),
        ))
    }
}

impl Default for ExploreAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for ExploreAgent {
    fn definition(&self) -> AgentDefinition {
        self.0.definition()
    }

    async fn execute(&self, query: &str, context: &AgentContext) -> Result<AgentResult> {
        self.0.execute(query, context).await
    }
}

pub struct PlanAgent(LlmAgent);

impl PlanAgent {
    pub fn new() -> Self {
        Self(LlmAgent::new(
            AgentDefinition::new(
                "plan",
                "Agent for task breakdown and planning",
                r#"You are a planning agent that breaks down complex tasks into actionable steps.

Your role:
- Analyze the task requirements
- Explore the codebase to understand current state
- Create a structured implementation plan

Output format:
1. Brief problem statement
2. Proposed approach
3. Numbered list of concrete steps
4. Any risks or considerations

Guidelines:
- Be specific about files and functions to modify
- Include verification steps
- Note dependencies between tasks"#,
            )
            .with_tools(vec!["glob".into(), "grep".into(), "view".into()])
            .with_max_turns(5),
        ))
    }
}

impl Default for PlanAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for PlanAgent {
    fn definition(&self) -> AgentDefinition {
        self.0.definition()
    }

    async fn execute(&self, query: &str, context: &AgentContext) -> Result<AgentResult> {
        self.0.execute(query, context).await
    }
}

pub struct TaskAgent(LlmAgent);

impl TaskAgent {
    pub fn new() -> Self {
        Self(LlmAgent::new(
            AgentDefinition::new(
                "task",
                "Agent for executing commands with verbose output",
                r#"You are a task execution agent that runs commands and reports results.

Your role:
- Execute build, test, lint, and other development commands
- Return brief summary on success
- Return full output on failure for debugging

Guidelines:
- Report "All N tests passed" style summaries on success
- Include stack traces and error messages on failure
- Track command exit codes"#,
            )
            .with_tools(vec!["bash".into()])
            .with_max_turns(3),
        ))
    }
}

impl Default for TaskAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for TaskAgent {
    fn definition(&self) -> AgentDefinition {
        self.0.definition()
    }

    async fn execute(&self, query: &str, context: &AgentContext) -> Result<AgentResult> {
        self.0.execute(query, context).await
    }
}

pub struct CodeReviewAgent(LlmAgent);

impl CodeReviewAgent {
    pub fn new() -> Self {
        Self(LlmAgent::new(
            AgentDefinition::new(
                "code-review",
                "Agent for reviewing code with high signal-to-noise ratio",
                r#"You are a code review agent with extremely high standards.

Your role:
- Analyze staged/unstaged changes or branch diffs
- Only surface issues that genuinely matter
- Focus on bugs, security vulnerabilities, logic errors

What to report:
- Potential bugs or crashes
- Security vulnerabilities
- Logic errors
- Missing error handling
- Performance issues in hot paths

What NOT to comment on:
- Style or formatting (handled by linters)
- Minor naming suggestions
- "Consider" comments without substance

Guidelines:
- Be specific: file:line and exact issue
- Explain impact: why does this matter?
- Suggest fix when appropriate
- Will NOT modify code - review only"#,
            )
            .with_tools(vec!["bash".into(), "view".into()])
            .with_max_turns(5),
        ))
    }
}

impl Default for CodeReviewAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for CodeReviewAgent {
    fn definition(&self) -> AgentDefinition {
        self.0.definition()
    }

    async fn execute(&self, query: &str, context: &AgentContext) -> Result<AgentResult> {
        self.0.execute(query, context).await
    }
}

// ============================================================================
// Agent Registry
// ============================================================================

pub struct AgentRegistry {
    agents: HashMap<String, Arc<dyn Agent>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            agents: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    pub fn register_defaults(&mut self) {
        self.register(GeneralPurposeAgent::new());
        self.register(ExploreAgent::new());
        self.register(PlanAgent::new());
        self.register(TaskAgent::new());
        self.register(CodeReviewAgent::new());
    }

    pub fn register<A: Agent + 'static>(&mut self, agent: A) {
        self.agents
            .insert(agent.definition().name.clone(), Arc::new(agent));
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Agent>> {
        self.agents.get(name).cloned()
    }

    pub fn list(&self) -> Vec<AgentDefinition> {
        self.agents.values().map(|a| a.definition()).collect()
    }

    pub fn names(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Agent Orchestrator
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: Uuid,
    pub agent_name: String,
    pub query: String,
    #[serde(default)]
    pub depends_on: Vec<Uuid>,
    #[serde(default)]
    pub status: TaskStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: Uuid,
    pub result: AgentResult,
}

pub struct AgentOrchestrator {
    registry: Arc<AgentRegistry>,
    context: Arc<AgentContext>,
    tasks: Arc<RwLock<HashMap<Uuid, AgentTask>>>,
    results: Arc<RwLock<HashMap<Uuid, TaskResult>>>,
}

impl AgentOrchestrator {
    pub fn new(registry: Arc<AgentRegistry>, context: AgentContext) -> Self {
        Self {
            registry,
            context: Arc::new(context),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn submit_task(&self, agent_name: &str, query: &str) -> Result<Uuid> {
        let task = AgentTask {
            id: Uuid::new_v4(),
            agent_name: agent_name.to_string(),
            query: query.to_string(),
            depends_on: vec![],
            status: TaskStatus::Pending,
        };
        let id = task.id;
        self.tasks.write().await.insert(id, task);
        Ok(id)
    }

    pub async fn submit_task_with_deps(
        &self,
        agent_name: &str,
        query: &str,
        depends_on: Vec<Uuid>,
    ) -> Result<Uuid> {
        let task = AgentTask {
            id: Uuid::new_v4(),
            agent_name: agent_name.to_string(),
            query: query.to_string(),
            depends_on,
            status: TaskStatus::Pending,
        };
        let id = task.id;
        self.tasks.write().await.insert(id, task);
        Ok(id)
    }

    pub async fn run_task(&self, task_id: Uuid) -> Result<AgentResult> {
        let task = {
            let mut tasks = self.tasks.write().await;
            let task = tasks.get_mut(&task_id).ok_or_else(|| {
                theasus_core::TheasusError::Other("Task not found".to_string())
            })?;
            task.status = TaskStatus::Running;
            task.clone()
        };

        // Check dependencies
        for dep_id in &task.depends_on {
            let results = self.results.read().await;
            if !results.contains_key(dep_id) {
                return Err(theasus_core::TheasusError::Other(format!(
                    "Dependency {} not completed",
                    dep_id
                )));
            }
        }

        // Get agent and execute
        let agent = self.registry.get(&task.agent_name).ok_or_else(|| {
            theasus_core::TheasusError::Other(format!("Agent not found: {}", task.agent_name))
        })?;

        let result = agent.execute(&task.query, &self.context).await;

        // Update task status and store result
        {
            let mut tasks = self.tasks.write().await;
            if let Some(t) = tasks.get_mut(&task_id) {
                t.status = if result.is_ok() {
                    TaskStatus::Completed
                } else {
                    TaskStatus::Failed
                };
            }
        }

        if let Ok(ref r) = result {
            self.results.write().await.insert(
                task_id,
                TaskResult {
                    task_id,
                    result: r.clone(),
                },
            );
        }

        result
    }

    pub async fn run_all(&self) -> Result<Vec<TaskResult>> {
        // Simple execution: run tasks in dependency order
        let task_ids: Vec<Uuid> = self.tasks.read().await.keys().cloned().collect();
        let mut completed = vec![];

        for task_id in task_ids {
            match self.run_task(task_id).await {
                Ok(result) => {
                    completed.push(TaskResult { task_id, result });
                }
                Err(e) => {
                    tracing::error!("Task {} failed: {}", task_id, e);
                }
            }
        }

        Ok(completed)
    }

    pub async fn get_result(&self, task_id: Uuid) -> Option<TaskResult> {
        self.results.read().await.get(&task_id).cloned()
    }

    pub async fn get_task_status(&self, task_id: Uuid) -> Option<TaskStatus> {
        self.tasks.read().await.get(&task_id).map(|t| t.status.clone())
    }

    pub async fn list_pending(&self) -> Vec<AgentTask> {
        self.tasks
            .read()
            .await
            .values()
            .filter(|t| t.status == TaskStatus::Pending)
            .cloned()
            .collect()
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Agent '{0}' not found")]
    NotFound(String),

    #[error("Agent execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Agent timeout after {0}s")]
    Timeout(u64),

    #[error("No LLM provider configured")]
    NoLlmProvider,

    #[error("Max turns exceeded")]
    MaxTurnsExceeded,

    #[error("Dependency not satisfied: {0}")]
    DependencyNotSatisfied(Uuid),
}

pub type AgentResultT<T> = std::result::Result<T, AgentError>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_definition() {
        let def = AgentDefinition::new("test", "Test agent", "System prompt")
            .with_tools(vec!["tool1".to_string()])
            .with_max_turns(5);

        assert_eq!(def.name, "test");
        assert_eq!(def.allowed_tools, Some(vec!["tool1".to_string()]));
        assert_eq!(def.max_turns, Some(5));
    }

    #[test]
    fn test_agent_result() {
        let result = AgentResult::success("Done")
            .with_turns(3);

        assert!(result.success);
        assert_eq!(result.output, "Done");
        assert_eq!(result.turns_used, 3);
    }

    #[test]
    fn test_agent_registry() {
        let registry = AgentRegistry::new();
        let names = registry.names();

        assert!(names.contains(&"general-purpose".to_string()));
        assert!(names.contains(&"explore".to_string()));
        assert!(names.contains(&"plan".to_string()));
        assert!(names.contains(&"task".to_string()));
        assert!(names.contains(&"code-review".to_string()));
    }

    #[test]
    fn test_task_status() {
        assert_eq!(TaskStatus::default(), TaskStatus::Pending);
    }
}
