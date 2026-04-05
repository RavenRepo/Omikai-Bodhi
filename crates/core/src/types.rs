use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Branded ID Types
// ============================================================================

/// A strongly-typed session identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    /// Create a new random SessionId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a nil (all-zeros) SessionId.
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }

    /// Returns the inner UUID.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for SessionId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<SessionId> for Uuid {
    fn from(id: SessionId) -> Self {
        id.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A strongly-typed agent identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    /// Create a new random AgentId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a nil (all-zeros) AgentId.
    pub fn nil() -> Self {
        Self(Uuid::nil())
    }

    /// Returns the inner UUID.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for AgentId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<AgentId> for Uuid {
    fn from(id: AgentId) -> Self {
        id.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Task Types
// ============================================================================

/// The type of task being executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TaskType {
    /// A local bash command execution.
    #[default]
    LocalBash,
    /// A local agent running in-process.
    LocalAgent,
    /// A remote agent running on another machine.
    RemoteAgent,
    /// An in-process teammate agent.
    InProcessTeammate,
    /// A local workflow execution.
    LocalWorkflow,
    /// An MCP server monitor task.
    MonitorMcp,
    /// A background "dream" task for autonomous processing.
    Dream,
}

/// The status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TaskStatus {
    /// Task is waiting to be executed.
    #[default]
    Pending,
    /// Task is currently running.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task failed with an error.
    Failed,
    /// Task was forcefully terminated.
    Killed,
}

/// A task being tracked in the application state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            task_type: TaskType::default(),
            status: TaskStatus::default(),
            created_at: Utc::now(),
        }
    }
}

impl Task {
    pub fn new(task_type: TaskType) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_type,
            status: TaskStatus::Pending,
            created_at: Utc::now(),
        }
    }
}

/// Context for tool permission decisions within a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolPermissionContext {
    pub session_rules: Vec<String>,
    pub pending_asks: Vec<String>,
}

impl ToolPermissionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_rule(&mut self, rule: String) {
        self.session_rules.push(rule);
    }

    pub fn add_pending_ask(&mut self, ask: String) {
        self.pending_asks.push(ask);
    }

    pub fn clear_pending_asks(&mut self) {
        self.pending_asks.clear();
    }
}

// ============================================================================
// Message Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    System(SystemMessage),
    Progress(ProgressMessage),
    Attachment(AttachmentMessage),
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Message::User(UserMessage {
            id: Uuid::new_v4(),
            content: vec![ContentBlock::Text { text: content.into() }],
            timestamp: Utc::now(),
        })
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Message::Assistant(AssistantMessage {
            id: Uuid::new_v4(),
            content: vec![ContentBlock::Text { text: content.into() }],
            tool_calls: vec![],
            usage: Usage::default(),
            model: String::new(),
            stop_reason: None,
            timestamp: Utc::now(),
        })
    }

    pub fn system(content: impl Into<String>) -> Self {
        Message::System(SystemMessage {
            content: content.into(),
        })
    }

    pub fn tool_result(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Message::User(UserMessage {
            id: Uuid::new_v4(),
            content: vec![ContentBlock::ToolResult {
                tool_use_id: tool_use_id.into(),
                content: content.into(),
            }],
            timestamp: Utc::now(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub id: Uuid,
    pub content: Vec<ContentBlock>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub id: Uuid,
    pub content: Vec<ContentBlock>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    pub model: String,
    pub stop_reason: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressMessage {
    pub message: String,
    pub progress: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentMessage {
    pub id: Uuid,
    pub file_name: String,
    pub mime_type: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Image {
        url: String,
        detail: String,
    },
    ToolUse {
        tool: ToolCall,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_use_id: String,
    pub content: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub messages: Vec<Message>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub messages: Vec<Message>,
    pub session_id: SessionId,
    pub cwd: std::path::PathBuf,
    pub tasks: Vec<Task>,
    pub tool_permission_context: ToolPermissionContext,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            session_id: SessionId::new(),
            cwd: std::env::current_dir().unwrap_or_default(),
            tasks: Vec::new(),
            tool_permission_context: ToolPermissionContext::default(),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn get_task(&self, id: Uuid) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn get_task_mut(&mut self, id: Uuid) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn update_task_status(&mut self, id: Uuid, status: TaskStatus) -> bool {
        if let Some(task) = self.get_task_mut(id) {
            task.status = status;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: String,
    pub max_budget_usd: Option<f64>,
    pub max_tokens: Option<u32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            api_key: None,
            base_url: "https://api.openai.com".to_string(),
            max_budget_usd: Some(10.0),
            max_tokens: Some(4096),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionMode {
    Default,
    AcceptEdits,
    BypassPermissions,
    DontAsk,
    Plan,
    Auto,
    Bubble,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionResult {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionContext {
    pub session_id: SessionId,
    pub cwd: std::path::PathBuf,
    pub mode: PermissionMode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_creation() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_session_id_nil() {
        let nil_id = SessionId::nil();
        assert_eq!(nil_id.0, Uuid::nil());
    }

    #[test]
    fn test_session_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let session_id = SessionId::from(uuid);
        assert_eq!(session_id.0, uuid);
    }

    #[test]
    fn test_session_id_into_uuid() {
        let session_id = SessionId::new();
        let uuid: Uuid = session_id.into();
        assert_eq!(uuid, session_id.0);
    }

    #[test]
    fn test_session_id_as_uuid() {
        let session_id = SessionId::new();
        assert_eq!(session_id.as_uuid(), session_id.0);
    }

    #[test]
    fn test_session_id_display() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let session_id = SessionId(uuid);
        assert_eq!(session_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_session_id_serde_roundtrip() {
        let original = SessionId::new();
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: SessionId = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_agent_id_creation() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_agent_id_nil() {
        let nil_id = AgentId::nil();
        assert_eq!(nil_id.0, Uuid::nil());
    }

    #[test]
    fn test_agent_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let agent_id = AgentId::from(uuid);
        assert_eq!(agent_id.0, uuid);
    }

    #[test]
    fn test_agent_id_into_uuid() {
        let agent_id = AgentId::new();
        let uuid: Uuid = agent_id.into();
        assert_eq!(uuid, agent_id.0);
    }

    #[test]
    fn test_agent_id_as_uuid() {
        let agent_id = AgentId::new();
        assert_eq!(agent_id.as_uuid(), agent_id.0);
    }

    #[test]
    fn test_agent_id_display() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let agent_id = AgentId(uuid);
        assert_eq!(agent_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_agent_id_serde_roundtrip() {
        let original = AgentId::new();
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: AgentId = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_task_type_serde() {
        let types = [
            TaskType::LocalBash,
            TaskType::LocalAgent,
            TaskType::RemoteAgent,
            TaskType::InProcessTeammate,
            TaskType::LocalWorkflow,
            TaskType::MonitorMcp,
            TaskType::Dream,
        ];

        for task_type in types {
            let json = serde_json::to_string(&task_type).unwrap();
            let deserialized: TaskType = serde_json::from_str(&json).unwrap();
            assert_eq!(task_type, deserialized);
        }
    }

    #[test]
    fn test_task_status_serde() {
        let statuses = [
            TaskStatus::Pending,
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Killed,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: TaskStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_task_type_json_format() {
        assert_eq!(
            serde_json::to_string(&TaskType::LocalBash).unwrap(),
            "\"LocalBash\""
        );
        assert_eq!(
            serde_json::to_string(&TaskType::InProcessTeammate).unwrap(),
            "\"InProcessTeammate\""
        );
    }

    #[test]
    fn test_task_status_json_format() {
        assert_eq!(
            serde_json::to_string(&TaskStatus::Pending).unwrap(),
            "\"Pending\""
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Completed).unwrap(),
            "\"Completed\""
        );
    }

    #[test]
    fn test_session_id_hash() {
        use std::collections::HashSet;
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        let mut set = HashSet::new();
        set.insert(id1);
        set.insert(id2);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&id1));
        assert!(set.contains(&id2));
    }

    #[test]
    fn test_agent_id_hash() {
        use std::collections::HashSet;
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        let mut set = HashSet::new();
        set.insert(id1);
        set.insert(id2);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&id1));
        assert!(set.contains(&id2));
    }

    #[test]
    fn test_task_default() {
        let task = Task::default();
        assert_eq!(task.task_type, TaskType::LocalBash);
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_task_new() {
        let task = Task::new(TaskType::RemoteAgent);
        assert_eq!(task.task_type, TaskType::RemoteAgent);
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_task_serde_roundtrip() {
        let task = Task::new(TaskType::LocalWorkflow);
        let json = serde_json::to_string(&task).unwrap();
        let deserialized: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(task.id, deserialized.id);
        assert_eq!(task.task_type, deserialized.task_type);
        assert_eq!(task.status, deserialized.status);
    }

    #[test]
    fn test_tool_permission_context_default() {
        let ctx = ToolPermissionContext::default();
        assert!(ctx.session_rules.is_empty());
        assert!(ctx.pending_asks.is_empty());
    }

    #[test]
    fn test_tool_permission_context_add_rule() {
        let mut ctx = ToolPermissionContext::new();
        ctx.add_rule("allow:bash:*".to_string());
        ctx.add_rule("deny:file_write:/etc/*".to_string());
        assert_eq!(ctx.session_rules.len(), 2);
        assert_eq!(ctx.session_rules[0], "allow:bash:*");
    }

    #[test]
    fn test_tool_permission_context_pending_asks() {
        let mut ctx = ToolPermissionContext::new();
        ctx.add_pending_ask("Execute bash command?".to_string());
        ctx.add_pending_ask("Write to file?".to_string());
        assert_eq!(ctx.pending_asks.len(), 2);
        ctx.clear_pending_asks();
        assert!(ctx.pending_asks.is_empty());
    }

    #[test]
    fn test_tool_permission_context_serde_roundtrip() {
        let mut ctx = ToolPermissionContext::new();
        ctx.add_rule("allow:*".to_string());
        ctx.add_pending_ask("Test ask".to_string());

        let json = serde_json::to_string(&ctx).unwrap();
        let deserialized: ToolPermissionContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx.session_rules, deserialized.session_rules);
        assert_eq!(ctx.pending_asks, deserialized.pending_asks);
    }

    #[test]
    fn test_app_state_with_tasks() {
        let mut state = AppState::new();
        assert!(state.tasks.is_empty());

        let task1 = Task::new(TaskType::LocalBash);
        let task1_id = task1.id;
        state.add_task(task1);
        assert_eq!(state.tasks.len(), 1);

        let task2 = Task::new(TaskType::RemoteAgent);
        state.add_task(task2);
        assert_eq!(state.tasks.len(), 2);

        let found = state.get_task(task1_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().task_type, TaskType::LocalBash);
    }

    #[test]
    fn test_app_state_update_task_status() {
        let mut state = AppState::new();
        let task = Task::new(TaskType::LocalAgent);
        let task_id = task.id;
        state.add_task(task);

        assert!(state.update_task_status(task_id, TaskStatus::Running));
        assert_eq!(state.get_task(task_id).unwrap().status, TaskStatus::Running);

        assert!(state.update_task_status(task_id, TaskStatus::Completed));
        assert_eq!(state.get_task(task_id).unwrap().status, TaskStatus::Completed);

        let nonexistent_id = Uuid::new_v4();
        assert!(!state.update_task_status(nonexistent_id, TaskStatus::Failed));
    }

    #[test]
    fn test_app_state_with_tool_permission_context() {
        let mut state = AppState::new();
        state.tool_permission_context.add_rule("allow:bash:ls".to_string());
        state.tool_permission_context.add_pending_ask("Execute ls?".to_string());

        assert_eq!(state.tool_permission_context.session_rules.len(), 1);
        assert_eq!(state.tool_permission_context.pending_asks.len(), 1);
    }

    #[test]
    fn test_app_state_serde_with_tasks() {
        let mut state = AppState::new();
        state.add_task(Task::new(TaskType::LocalBash));
        state.tool_permission_context.add_rule("test-rule".to_string());

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: AppState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.tasks.len(), deserialized.tasks.len());
        assert_eq!(state.tool_permission_context.session_rules, deserialized.tool_permission_context.session_rules);
    }
}
