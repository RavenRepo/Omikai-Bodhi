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
use theasus_knowledge::{
    KnowledgeEntry, KnowledgeProvider, KnowledgeQuery, EntryType,
    promotion::PromotionEvaluation,
};
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
    /// Knowledge domains this agent queries before execution.
    /// None means no knowledge binding — backward compatible.
    #[serde(default)]
    pub knowledge: Option<KnowledgeBinding>,
    /// Structured reasoning instructions injected alongside domain context.
    /// None means no reasoning framework — backward compatible.
    #[serde(default)]
    pub reasoning: Option<ReasoningFramework>,
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
            knowledge: None,
            reasoning: None,
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

    /// Bind this agent to knowledge domains for pre-execution context injection
    /// and post-execution knowledge capture.
    pub fn with_knowledge(mut self, binding: KnowledgeBinding) -> Self {
        self.knowledge = Some(binding);
        self
    }

    /// Attach a reasoning framework that gets injected into the system prompt
    /// as structured reasoning instructions.
    pub fn with_reasoning(mut self, reasoning: ReasoningFramework) -> Self {
        self.reasoning = Some(reasoning);
        self
    }
}

// ============================================================================
// Knowledge Binding — Connects agents to domain knowledge
// ============================================================================

/// Binds an agent to specific knowledge domains.
///
/// This is the bridge between the agent definition (what the agent IS)
/// and the knowledge layer (what the agent KNOWS). It declares:
/// - Which domains to query before the LLM loop starts
/// - What queries to execute for pre-execution context
/// - What to extract from agent output after execution
/// - How much token budget to allocate for injected knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBinding {
    /// Knowledge domains this agent is interested in.
    /// Maps to `KnowledgeEntry.domain` values.
    pub domains: Vec<String>,

    /// Queries executed before the LLM loop to build domain context.
    /// Results are compiled into a prompt fragment and appended to system_prompt.
    pub pre_execution_queries: Vec<KnowledgeQuery>,

    /// Patterns for extracting knowledge from agent output after execution.
    /// Each pattern defines what to capture and when.
    pub post_execution_captures: Vec<CapturePattern>,

    /// Maximum tokens to allocate for injected knowledge context.
    /// Prevents domain context from overwhelming the system prompt.
    /// None means no limit (use all available context).
    pub max_context_tokens: Option<usize>,

    /// Enable contract-based evaluation for graded knowledge promotion.
    /// When true, observations go through PromotionEvaluation scoring.
    #[serde(default)]
    pub enable_graded_promotion: bool,

    /// Minimum promotion score (0.0-1.0) required to store captured knowledge.
    /// Only used when `enable_graded_promotion` is true.
    #[serde(default = "default_promotion_threshold")]
    pub promotion_threshold: f32,
}

fn default_promotion_threshold() -> f32 {
    0.6
}

/// Defines what knowledge to extract from agent output after execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturePattern {
    /// The domain to store captured knowledge in.
    pub domain: String,

    /// The entry type for captured knowledge.
    pub entry_type: EntryType,

    /// When to trigger knowledge capture.
    pub trigger: CaptureTrigger,
}

impl CapturePattern {
    /// Check whether this capture should trigger based on the agent result.
    pub fn should_trigger(&self, result: &AgentResult) -> bool {
        match &self.trigger {
            CaptureTrigger::Always => true,
            CaptureTrigger::OnSuccess => result.success,
            CaptureTrigger::OnPattern(pattern) => result.output.contains(pattern),
        }
    }
}

/// When to trigger post-execution knowledge capture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaptureTrigger {
    /// Always capture after execution completes.
    Always,
    /// Only capture when the agent reports success.
    OnSuccess,
    /// Capture when agent output contains this substring.
    OnPattern(String),
}

// ============================================================================
// Reasoning Framework — Structured reasoning instructions
// ============================================================================

/// Structured reasoning instructions injected into the agent's system prompt.
///
/// Instead of bloating system_prompt with reasoning instructions directly,
/// the ReasoningFramework provides a structured way to declare the agent's
/// reasoning strategy, constraints, and output format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningFramework {
    /// The high-level reasoning strategy to use.
    pub approach: ReasoningApproach,

    /// Domain-specific constraints that must be respected during reasoning.
    pub constraints: Vec<String>,

    /// Expected output format (e.g., "FINDING: [severity] [file:line] [desc]").
    pub output_format: Option<String>,
}

impl ReasoningFramework {
    /// Convert the reasoning framework into prompt instructions.
    pub fn to_prompt_instructions(&self) -> String {
        let mut instructions = String::new();

        instructions.push_str(&format!("Reasoning Approach: {}\n", self.approach));

        if !self.constraints.is_empty() {
            instructions.push_str("\nConstraints:\n");
            for constraint in &self.constraints {
                instructions.push_str(&format!("- {}\n", constraint));
            }
        }

        if let Some(ref format) = self.output_format {
            instructions.push_str(&format!("\nOutput Format: {}\n", format));
        }

        instructions
    }
}

/// High-level reasoning strategy for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReasoningApproach {
    /// Chain-of-thought: break down problem step by step.
    StepByStep,
    /// Explore multiple reasoning paths before committing.
    TreeOfThought,
    /// Simulate multiple expert perspectives.
    ExpertPanel,
    /// Start from constraints, work toward solutions.
    ConstraintFirst,
    /// Verify assumptions before taking any action.
    VerifyThenAct,
}

impl std::fmt::Display for ReasoningApproach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StepByStep => write!(f, "Step-by-step chain of thought"),
            Self::TreeOfThought => write!(f, "Explore multiple paths before committing"),
            Self::ExpertPanel => write!(f, "Simulate multiple expert perspectives"),
            Self::ConstraintFirst => write!(f, "Start from constraints, work toward solutions"),
            Self::VerifyThenAct => write!(f, "Verify all assumptions before acting"),
        }
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
    /// Knowledge provider for domain context injection and learning capture.
    /// None means no knowledge layer — agents execute without domain context.
    pub knowledge_provider: Option<Arc<dyn KnowledgeProvider>>,
    pub extra: HashMap<String, serde_json::Value>,
}

impl AgentContext {
    pub fn new(cwd: std::path::PathBuf, tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            cwd,
            tool_registry,
            llm_provider: None,
            knowledge_provider: None,
            extra: HashMap::new(),
        }
    }

    pub fn with_llm(mut self, provider: Arc<dyn LlmProvider>) -> Self {
        self.llm_provider = Some(provider);
        self
    }

    /// Attach a knowledge provider for domain context injection.
    pub fn with_knowledge(mut self, provider: Arc<dyn KnowledgeProvider>) -> Self {
        self.knowledge_provider = Some(provider);
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

        // ================================================================
        // PRE-EXECUTION: Knowledge injection
        // ================================================================
        // Build the effective system prompt by starting with the base prompt
        // and optionally appending domain knowledge and reasoning framework.
        let mut system_prompt = self.definition.system_prompt.clone();

        // Inject domain knowledge if both binding and provider are present
        if let (Some(binding), Some(kp)) = (
            &self.definition.knowledge,
            &context.knowledge_provider,
        ) {
            match kp
                .compile_context(
                    &binding.pre_execution_queries,
                    binding.max_context_tokens,
                )
                .await
            {
                Ok(domain_context) => {
                    if !domain_context.compiled_prompt.is_empty() {
                        system_prompt.push_str("\n\n--- Domain Knowledge ---\n");
                        system_prompt.push_str(&domain_context.compiled_prompt);
                        tracing::info!(
                            agent = %self.definition.name,
                            entries = domain_context.entries.len(),
                            tokens = domain_context.token_estimate,
                            "Injected domain knowledge into system prompt"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        agent = %self.definition.name,
                        error = %e,
                        "Failed to compile domain context, continuing without knowledge"
                    );
                }
            }
        }

        // Inject reasoning framework instructions
        if let Some(reasoning) = &self.definition.reasoning {
            system_prompt.push_str("\n\n--- Reasoning Framework ---\n");
            system_prompt.push_str(&reasoning.to_prompt_instructions());
        }

        // ================================================================
        // LLM LOOP (unchanged core logic)
        // ================================================================
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
                system: Some(system_prompt.clone()),
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
                let result = AgentResult::success(&response.text)
                    .with_messages(messages)
                    .with_tool_calls(all_tool_calls)
                    .with_turns(turns);

                // ========================================================
                // POST-EXECUTION: Knowledge capture with graded promotion
                // ========================================================
                if let (Some(binding), Some(kp)) = (
                    &self.definition.knowledge,
                    &context.knowledge_provider,
                ) {
                    for capture in &binding.post_execution_captures {
                        if capture.should_trigger(&result) {
                            let mut entry = KnowledgeEntry::from_agent_output(
                                &capture.domain,
                                format!(
                                    "[{}] Observation from task",
                                    self.definition.name
                                ),
                                &result.output,
                                capture.entry_type.clone(),
                            );

                            // Apply graded promotion if enabled
                            if binding.enable_graded_promotion {
                                let evaluation = PromotionEvaluation::from_success_result(
                                    result.success,
                                    &result.output,
                                );
                                let score = evaluation.weighted_score();

                                if score >= binding.promotion_threshold {
                                    // Promote with adjusted confidence
                                    entry = entry.with_confidence(score);
                                    tracing::info!(
                                        agent = %self.definition.name,
                                        domain = %capture.domain,
                                        score = %score,
                                        "Knowledge promoted via graded evaluation"
                                    );
                                } else {
                                    tracing::debug!(
                                        agent = %self.definition.name,
                                        domain = %capture.domain,
                                        score = %score,
                                        threshold = %binding.promotion_threshold,
                                        "Knowledge below promotion threshold, skipping"
                                    );
                                    continue;
                                }
                            }

                            if let Err(e) = kp.store(entry).await {
                                tracing::warn!(
                                    agent = %self.definition.name,
                                    error = %e,
                                    "Failed to capture post-execution knowledge"
                                );
                            }
                        }
                    }
                }

                return Ok(result);
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

pub struct VerificationAgent(LlmAgent);

impl VerificationAgent {
    pub fn new() -> Self {
        Self(LlmAgent::new(
            AgentDefinition::new(
                "verification",
                "Verifies code changes and test results",
                r#"You are a verification agent. Your job is to verify that code changes are correct, tests pass, and the implementation matches requirements. Check for edge cases, error handling, and potential bugs.

Your role:
- Run tests and verify they pass
- Check code changes match requirements
- Verify edge cases are handled
- Ensure proper error handling exists

Guidelines:
- Be thorough but efficient
- Report specific issues with file:line references
- Suggest fixes for problems found
- Confirm when verification passes"#,
            )
            .with_tools(vec![
                "bash".into(),
                "file_read".into(),
                "grep".into(),
                "glob".into(),
            ])
            .with_max_turns(5),
        ))
    }
}

impl Default for VerificationAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for VerificationAgent {
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
        self.register(VerificationAgent::new());
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
    pub parent_task_id: Option<Uuid>,
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
            parent_task_id: None,
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
            parent_task_id: None,
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

    pub async fn spawn_background(&self, agent_name: &str, query: &str) -> Result<Uuid> {
        let task_id = Uuid::new_v4();
        let task = AgentTask {
            id: task_id,
            agent_name: agent_name.to_string(),
            query: query.to_string(),
            depends_on: vec![],
            parent_task_id: None,
            status: TaskStatus::Pending,
        };

        self.tasks.write().await.insert(task_id, task);

        let registry = self.registry.clone();
        let context = self.context.clone();
        let tasks = self.tasks.clone();
        let results = self.results.clone();
        let agent_name = agent_name.to_string();
        let query = query.to_string();

        tokio::spawn(async move {
            // Update status to running
            {
                let mut task_map = tasks.write().await;
                if let Some(t) = task_map.get_mut(&task_id) {
                    t.status = TaskStatus::Running;
                }
            }

            // Execute agent
            let agent = match registry.get(&agent_name) {
                Some(a) => a,
                None => {
                    let mut task_map = tasks.write().await;
                    if let Some(t) = task_map.get_mut(&task_id) {
                        t.status = TaskStatus::Failed;
                    }
                    tracing::error!("Agent not found: {}", agent_name);
                    return;
                }
            };

            let result = agent.execute(&query, &context).await;

            // Update tasks/results when done
            let (status, agent_result) = match result {
                Ok(r) => (TaskStatus::Completed, r),
                Err(e) => {
                    tracing::error!("Background task {} failed: {}", task_id, e);
                    (TaskStatus::Failed, AgentResult::error(e.to_string()))
                }
            };

            {
                let mut task_map = tasks.write().await;
                if let Some(t) = task_map.get_mut(&task_id) {
                    t.status = status;
                }
            }

            results.write().await.insert(
                task_id,
                TaskResult {
                    task_id,
                    result: agent_result,
                },
            );
        });

        Ok(task_id)
    }

    pub async fn fork_agent(
        &self,
        parent_task_id: Uuid,
        agent_name: &str,
        query: &str,
    ) -> Result<Uuid> {
        // Verify parent task exists
        let parent_exists = self.tasks.read().await.contains_key(&parent_task_id);
        if !parent_exists {
            return Err(theasus_core::TheasusError::Other(format!(
                "Parent task not found: {}",
                parent_task_id
            )));
        }

        let child_task_id = Uuid::new_v4();
        let task = AgentTask {
            id: child_task_id,
            agent_name: agent_name.to_string(),
            query: query.to_string(),
            depends_on: vec![],
            parent_task_id: Some(parent_task_id),
            status: TaskStatus::Pending,
        };

        self.tasks.write().await.insert(child_task_id, task);

        let registry = self.registry.clone();
        let context = self.context.clone();
        let tasks = self.tasks.clone();
        let results = self.results.clone();
        let agent_name = agent_name.to_string();
        let query = query.to_string();

        tokio::spawn(async move {
            // Update status to running
            {
                let mut task_map = tasks.write().await;
                if let Some(t) = task_map.get_mut(&child_task_id) {
                    t.status = TaskStatus::Running;
                }
            }

            // Execute agent (inherits context from parent via shared context)
            let agent = match registry.get(&agent_name) {
                Some(a) => a,
                None => {
                    let mut task_map = tasks.write().await;
                    if let Some(t) = task_map.get_mut(&child_task_id) {
                        t.status = TaskStatus::Failed;
                    }
                    tracing::error!("Agent not found for fork: {}", agent_name);
                    return;
                }
            };

            let result = agent.execute(&query, &context).await;

            let (status, agent_result) = match result {
                Ok(r) => (TaskStatus::Completed, r),
                Err(e) => {
                    tracing::error!("Forked task {} failed: {}", child_task_id, e);
                    (TaskStatus::Failed, AgentResult::error(e.to_string()))
                }
            };

            {
                let mut task_map = tasks.write().await;
                if let Some(t) = task_map.get_mut(&child_task_id) {
                    t.status = status;
                }
            }

            results.write().await.insert(
                child_task_id,
                TaskResult {
                    task_id: child_task_id,
                    result: agent_result,
                },
            );
        });

        Ok(child_task_id)
    }

    pub async fn get_child_tasks(&self, parent_task_id: Uuid) -> Vec<AgentTask> {
        self.tasks
            .read()
            .await
            .values()
            .filter(|t| t.parent_task_id == Some(parent_task_id))
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
        assert!(def.knowledge.is_none());
        assert!(def.reasoning.is_none());
    }

    #[test]
    fn test_agent_definition_with_knowledge() {
        let def = AgentDefinition::new("security", "Security agent", "Check security")
            .with_knowledge(KnowledgeBinding {
                domains: vec!["security".into()],
                pre_execution_queries: vec![
                    KnowledgeQuery::new()
                        .with_domains(vec!["security".into()])
                        .with_entry_types(vec![EntryType::Rule]),
                ],
                post_execution_captures: vec![CapturePattern {
                    domain: "security".into(),
                    entry_type: EntryType::Observation,
                    trigger: CaptureTrigger::OnSuccess,
                }],
                max_context_tokens: Some(2000),
                enable_graded_promotion: false,
                promotion_threshold: 0.6,
            });

        assert!(def.knowledge.is_some());
        let binding = def.knowledge.unwrap();
        assert_eq!(binding.domains, vec!["security"]);
        assert_eq!(binding.max_context_tokens, Some(2000));
    }

    #[test]
    fn test_agent_definition_with_reasoning() {
        let def = AgentDefinition::new("reviewer", "Code reviewer", "Review code")
            .with_reasoning(ReasoningFramework {
                approach: ReasoningApproach::ConstraintFirst,
                constraints: vec!["Check OWASP Top 10".into()],
                output_format: Some("FINDING: [severity] [description]".into()),
            });

        assert!(def.reasoning.is_some());
        let reasoning = def.reasoning.unwrap();
        let instructions = reasoning.to_prompt_instructions();
        assert!(instructions.contains("Constraint"));
        assert!(instructions.contains("OWASP"));
    }

    #[test]
    fn test_backward_compat_serde() {
        // JSON without knowledge/reasoning fields should deserialize fine
        let json = r#"{
            "name": "test",
            "description": "legacy agent",
            "system_prompt": "do things"
        }"#;

        let def: AgentDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.name, "test");
        assert!(def.knowledge.is_none());
        assert!(def.reasoning.is_none());
        assert!(def.allowed_tools.is_none());
        assert!(def.max_turns.is_none());
        assert!(def.temperature.is_none());
    }

    #[test]
    fn test_capture_trigger_always() {
        let capture = CapturePattern {
            domain: "test".into(),
            entry_type: EntryType::Observation,
            trigger: CaptureTrigger::Always,
        };
        assert!(capture.should_trigger(&AgentResult::success("ok")));
        assert!(capture.should_trigger(&AgentResult::error("fail")));
    }

    #[test]
    fn test_capture_trigger_on_success() {
        let capture = CapturePattern {
            domain: "test".into(),
            entry_type: EntryType::Observation,
            trigger: CaptureTrigger::OnSuccess,
        };
        assert!(capture.should_trigger(&AgentResult::success("ok")));
        assert!(!capture.should_trigger(&AgentResult::error("fail")));
    }

    #[test]
    fn test_capture_trigger_on_pattern() {
        let capture = CapturePattern {
            domain: "test".into(),
            entry_type: EntryType::Observation,
            trigger: CaptureTrigger::OnPattern("DISCOVERY:".into()),
        };
        assert!(capture.should_trigger(&AgentResult::success("DISCOVERY: found pattern")));
        assert!(!capture.should_trigger(&AgentResult::success("nothing special")));
    }

    #[test]
    fn test_reasoning_approach_display() {
        assert!(ReasoningApproach::StepByStep.to_string().contains("step"));
        assert!(ReasoningApproach::ConstraintFirst.to_string().contains("onstraint"));
        assert!(ReasoningApproach::VerifyThenAct.to_string().contains("erify"));
    }

    #[test]
    fn test_reasoning_framework_to_prompt() {
        let framework = ReasoningFramework {
            approach: ReasoningApproach::StepByStep,
            constraints: vec!["Be thorough".into(), "Check edge cases".into()],
            output_format: Some("RESULT: [status] [details]".into()),
        };

        let prompt = framework.to_prompt_instructions();
        assert!(prompt.contains("Step-by-step"));
        assert!(prompt.contains("Be thorough"));
        assert!(prompt.contains("Check edge cases"));
        assert!(prompt.contains("RESULT:"));
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
        assert!(names.contains(&"verification".to_string()));
    }

    #[test]
    fn test_task_status() {
        assert_eq!(TaskStatus::default(), TaskStatus::Pending);
    }

    #[test]
    fn test_verification_agent_creation() {
        let agent = VerificationAgent::new();
        let def = agent.definition();

        assert_eq!(def.name, "verification");
        assert_eq!(def.description, "Verifies code changes and test results");
        assert_eq!(def.max_turns, Some(5));
        assert_eq!(
            def.allowed_tools,
            Some(vec![
                "bash".to_string(),
                "file_read".to_string(),
                "grep".to_string(),
                "glob".to_string(),
            ])
        );
    }

    #[test]
    fn test_verification_agent_default() {
        let agent = VerificationAgent::default();
        assert_eq!(agent.definition().name, "verification");
    }

    #[tokio::test]
    async fn test_background_task_spawning() {
        let registry = Arc::new(AgentRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let context = AgentContext::new(std::env::temp_dir(), tool_registry);
        let orchestrator = AgentOrchestrator::new(registry, context);

        // Spawn a background task
        let task_id = orchestrator
            .spawn_background("explore", "test query")
            .await
            .expect("Failed to spawn background task");

        // Task should be registered
        let status = orchestrator.get_task_status(task_id).await;
        assert!(status.is_some());

        // Status should be Pending or Running (depending on timing)
        let status = status.unwrap();
        assert!(status == TaskStatus::Pending || status == TaskStatus::Running);
    }

    #[tokio::test]
    async fn test_agent_forking() {
        let registry = Arc::new(AgentRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let context = AgentContext::new(std::env::temp_dir(), tool_registry);
        let orchestrator = AgentOrchestrator::new(registry, context);

        // Create parent task
        let parent_id = orchestrator
            .submit_task("explore", "parent query")
            .await
            .expect("Failed to submit parent task");

        // Fork from parent
        let child_id = orchestrator
            .fork_agent(parent_id, "task", "child query")
            .await
            .expect("Failed to fork agent");

        // Verify child task was created with parent reference
        let tasks = orchestrator.tasks.read().await;
        let child_task = tasks.get(&child_id).expect("Child task not found");
        assert_eq!(child_task.parent_task_id, Some(parent_id));
        assert_eq!(child_task.agent_name, "task");
    }

    #[tokio::test]
    async fn test_fork_agent_invalid_parent() {
        let registry = Arc::new(AgentRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let context = AgentContext::new(std::env::temp_dir(), tool_registry);
        let orchestrator = AgentOrchestrator::new(registry, context);

        // Try to fork from non-existent parent
        let fake_parent_id = Uuid::new_v4();
        let result = orchestrator
            .fork_agent(fake_parent_id, "task", "query")
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_child_tasks() {
        let registry = Arc::new(AgentRegistry::new());
        let tool_registry = Arc::new(ToolRegistry::new());
        let context = AgentContext::new(std::env::temp_dir(), tool_registry);
        let orchestrator = AgentOrchestrator::new(registry, context);

        // Create parent task
        let parent_id = orchestrator
            .submit_task("explore", "parent query")
            .await
            .expect("Failed to submit parent task");

        // Fork multiple children
        let _child1 = orchestrator
            .fork_agent(parent_id, "task", "child 1")
            .await
            .expect("Failed to fork child 1");
        let _child2 = orchestrator
            .fork_agent(parent_id, "plan", "child 2")
            .await
            .expect("Failed to fork child 2");

        // Get child tasks
        let children = orchestrator.get_child_tasks(parent_id).await;
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_agent_task_with_parent() {
        let parent_id = Uuid::new_v4();
        let task = AgentTask {
            id: Uuid::new_v4(),
            agent_name: "test".to_string(),
            query: "query".to_string(),
            depends_on: vec![],
            parent_task_id: Some(parent_id),
            status: TaskStatus::Pending,
        };

        assert_eq!(task.parent_task_id, Some(parent_id));
    }
}
