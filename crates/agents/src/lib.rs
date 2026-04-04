use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use theasus_core::Result;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub success: bool,
    pub output: String,
    pub messages: Vec<theasus_core::Message>,
    pub tool_calls: Vec<theasus_core::ToolCall>,
}

impl AgentResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            messages: vec![],
            tool_calls: vec![],
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            output: msg.into(),
            messages: vec![],
            tool_calls: vec![],
        }
    }
}

pub struct AgentContext {
    pub session_id: Uuid,
    pub cwd: std::path::PathBuf,
    pub tool_registry: Arc<theasus_tools::ToolRegistry>,
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn definition(&self) -> AgentDefinition;

    async fn execute(&self, query: &str, context: &AgentContext) -> Result<AgentResult>;
}

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
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GeneralPurposeAgent;

impl GeneralPurposeAgent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Agent for GeneralPurposeAgent {
    fn definition(&self) -> AgentDefinition {
        AgentDefinition {
            name: "general-purpose".to_string(),
            description: "General purpose agent for answering questions and executing tasks".to_string(),
            system_prompt: "You are a helpful AI assistant. Use the available tools to help the user with their tasks.".to_string(),
        }
    }

    async fn execute(&self, query: &str, context: &AgentContext) -> Result<AgentResult> {
        tracing::info!("GeneralPurposeAgent executing query: {}", query);
        Ok(AgentResult::success(format!("Processed query: {}", query)))
    }
}

impl Default for GeneralPurposeAgent {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ExploreAgent;

impl ExploreAgent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Agent for ExploreAgent {
    fn definition(&self) -> AgentDefinition {
        AgentDefinition {
            name: "explore".to_string(),
            description: "Explore codebase and gather information".to_string(),
            system_prompt: "You are an exploration agent. Use glob and grep to explore the codebase and gather information about files, patterns, and structure.".to_string(),
        }
    }

    async fn execute(&self, query: &str, context: &AgentContext) -> Result<AgentResult> {
        tracing::info!("ExploreAgent exploring: {}", query);
        Ok(AgentResult::success(format!("Explored: {}", query)))
    }
}

impl Default for ExploreAgent {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PlanAgent;

impl PlanAgent {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Agent for PlanAgent {
    fn definition(&self) -> AgentDefinition {
        AgentDefinition {
            name: "plan".to_string(),
            description: "Break down complex tasks into actionable steps".to_string(),
            system_prompt: "You are a planning agent. Break down complex tasks into clear, actionable steps. Output your plan as a numbered list.".to_string(),
        }
    }

    async fn execute(&self, query: &str, context: &AgentContext) -> Result<AgentResult> {
        tracing::info!("PlanAgent planning: {}", query);
        Ok(AgentResult::success(format!("Planned: {}", query)))
    }
}

impl Default for PlanAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Agent '{0}' not found")]
    NotFound(String),

    #[error("Agent execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Agent timeout after {0}s")]
    Timeout(u64),
}

pub type AgentResultT<T> = std::result::Result<T, AgentError>;
