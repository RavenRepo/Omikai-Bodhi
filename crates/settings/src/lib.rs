use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// MCP Server configuration for settings.
///
/// This is a simplified version of the MCP server config stored in settings.
/// The full MCP runtime types are in `theasus_mcp`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub transport: String,
}

impl McpServerConfig {
    pub fn new(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args: vec![],
            env: HashMap::new(),
            timeout_ms: Some(30000),
            transport: "stdio".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub model: String,
    pub api_key: Option<String>,
    pub llm_provider: String,
    pub llm_base_url: Option<String>,
    pub theme: Theme,
    pub max_budget_usd: Option<f64>,
    pub permission_mode: PermissionMode,
    pub mcp_servers: Vec<McpServerConfig>,
    pub custom_tools: Vec<String>,
    pub shortcuts: HashMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            api_key: None,
            llm_provider: "openai".to_string(),
            llm_base_url: None,
            theme: Theme::default(),
            max_budget_usd: None,
            permission_mode: PermissionMode::Default,
            mcp_servers: vec![],
            custom_tools: vec![],
            shortcuts: HashMap::new(),
        }
    }
}

impl Settings {
    pub fn get_config_dir() -> Option<PathBuf> {
        ProjectDirs::from("com", "omikai", "bodhi").map(|dirs| dirs.config_dir().to_path_buf())
    }

    pub fn get_config_path() -> Option<PathBuf> {
        Self::get_config_dir().map(|dir| dir.join("config.json"))
    }

    pub fn load() -> std::io::Result<Self> {
        let path = Self::get_config_path()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No config dir"))?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let settings: Settings = serde_json::from_str(&content)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(settings)
        } else {
            let settings = Self::default();
            settings.save()?;
            Ok(settings)
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let dir = Self::get_config_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No config dir"))?;

        std::fs::create_dir_all(&dir)?;

        let path = dir.join("config.json");
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        std::fs::write(&path, content)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PermissionMode {
    #[default]
    Default,
    AcceptEdits,
    BypassPermissions,
    DontAsk,
    Plan,
    Auto,
    Bubble,
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Failed to load settings: {0}")]
    LoadFailed(String),

    #[error("Failed to save settings: {0}")]
    SaveFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type SettingsResult<T> = std::result::Result<T, SettingsError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsBuilder {
    model: Option<String>,
    api_key: Option<String>,
    theme: Option<Theme>,
    max_budget_usd: Option<f64>,
    permission_mode: Option<PermissionMode>,
}

impl SettingsBuilder {
    pub fn new() -> Self {
        Self {
            model: None,
            api_key: None,
            theme: None,
            max_budget_usd: None,
            permission_mode: None,
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    pub fn max_budget_usd(mut self, budget: f64) -> Self {
        self.max_budget_usd = Some(budget);
        self
    }

    pub fn permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = Some(mode);
        self
    }

    pub fn build(self) -> Settings {
        let mut settings = Settings::default();

        if let Some(model) = self.model {
            settings.model = model;
        }
        if let Some(api_key) = self.api_key {
            settings.api_key = Some(api_key);
        }
        if let Some(theme) = self.theme {
            settings.theme = theme;
        }
        if let Some(budget) = self.max_budget_usd {
            settings.max_budget_usd = Some(budget);
        }
        if let Some(mode) = self.permission_mode {
            settings.permission_mode = mode;
        }

        settings
    }
}

impl Default for SettingsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub name: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub messages: Vec<SessionMessage>,
    pub metadata: SessionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub model: Option<String>,
    pub total_tokens: u32,
    pub message_count: usize,
}

pub struct SessionManager {
    sessions_dir: PathBuf,
}

impl SessionManager {
    pub fn new() -> std::io::Result<Self> {
        let config_dir = Settings::get_config_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No config dir"))?;
        let sessions_dir = config_dir.join("sessions");
        fs::create_dir_all(&sessions_dir)?;
        Ok(Self { sessions_dir })
    }

    pub fn save_session(&self, session: &Session) -> std::io::Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", session.id));
        let content = serde_json::to_string_pretty(session)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&path, content)
    }

    pub fn load_session(&self, id: Uuid) -> std::io::Result<Option<Session>> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let session: Session = serde_json::from_str(&content)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    pub fn list_sessions(&self) -> std::io::Result<Vec<Session>> {
        let mut sessions = Vec::new();

        for entry in fs::read_dir(&self.sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(session) = serde_json::from_str::<Session>(&content) {
                        sessions.push(session);
                    }
                }
            }
        }

        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    pub fn delete_session(&self, id: Uuid) -> std::io::Result<()> {
        let path = self.sessions_dir.join(format!("{}.json", id));
        if path.exists() {
            fs::remove_file(path)
        } else {
            Ok(())
        }
    }

    pub async fn resume_session(&self, session_id: Uuid) -> std::io::Result<Session> {
        self.load_session(session_id)?.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Session {} not found", session_id),
            )
        })
    }

    pub fn get_latest_session(&self) -> std::io::Result<Option<Session>> {
        let sessions = self.list_sessions()?;
        Ok(sessions.into_iter().next())
    }

    pub fn create_session(&self, name: Option<&str>) -> Session {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        Session {
            id: Uuid::new_v4(),
            name: name.unwrap_or("Untitled Session").to_string(),
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
            metadata: SessionMetadata::default(),
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new().expect("Failed to create session manager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.model, "gpt-4o");
        assert_eq!(settings.llm_provider, "openai");
        assert!(settings.api_key.is_none());
        assert_eq!(settings.theme, Theme::Dark);
        assert_eq!(settings.permission_mode, PermissionMode::Default);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = Settings::default();
        let json = serde_json::to_string(&settings).expect("Failed to serialize");
        let deserialized: Settings = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.model, settings.model);
        assert_eq!(deserialized.llm_provider, settings.llm_provider);
        assert_eq!(deserialized.theme, settings.theme);
    }

    #[test]
    fn test_settings_builder() {
        let settings = SettingsBuilder::new()
            .model("claude-3")
            .api_key("test-key")
            .theme(Theme::Light)
            .max_budget_usd(100.0)
            .permission_mode(PermissionMode::Auto)
            .build();

        assert_eq!(settings.model, "claude-3");
        assert_eq!(settings.api_key, Some("test-key".to_string()));
        assert_eq!(settings.theme, Theme::Light);
        assert_eq!(settings.max_budget_usd, Some(100.0));
        assert_eq!(settings.permission_mode, PermissionMode::Auto);
    }

    #[test]
    fn test_session_creation() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let session = Session {
            id: Uuid::new_v4(),
            name: "Test Session".to_string(),
            created_at: now,
            updated_at: now,
            messages: vec![],
            metadata: SessionMetadata::default(),
        };

        assert_eq!(session.name, "Test Session");
        assert!(session.messages.is_empty());
        assert_eq!(session.metadata.total_tokens, 0);
    }

    #[test]
    fn test_theme_default() {
        let theme = Theme::default();
        assert_eq!(theme, Theme::Dark);
    }

    #[test]
    fn test_session_manager_resume_and_get_latest() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        fs::create_dir_all(&sessions_dir).unwrap();

        let manager = SessionManager { sessions_dir };

        let session1 = manager.create_session(Some("First Session"));
        manager.save_session(&session1).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let mut session2 = manager.create_session(Some("Second Session"));
        session2.updated_at = session1.updated_at + 1000;
        manager.save_session(&session2).unwrap();

        let latest = manager.get_latest_session().unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().name, "Second Session");

        let rt = tokio::runtime::Runtime::new().unwrap();
        let resumed = rt.block_on(manager.resume_session(session1.id)).unwrap();
        assert_eq!(resumed.id, session1.id);
        assert_eq!(resumed.name, "First Session");
    }

    #[test]
    fn test_session_resume_not_found() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        fs::create_dir_all(&sessions_dir).unwrap();

        let manager = SessionManager { sessions_dir };

        let nonexistent_id = Uuid::new_v4();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(manager.resume_session(nonexistent_id));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_get_latest_session_empty() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        fs::create_dir_all(&sessions_dir).unwrap();

        let manager = SessionManager { sessions_dir };

        let latest = manager.get_latest_session().unwrap();
        assert!(latest.is_none());
    }
}
