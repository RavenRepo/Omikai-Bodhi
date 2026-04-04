use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use theasus_core::Message;
use theasus_mcp::McpServerConfig;
use uuid::Uuid;

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
pub enum Theme {
    Dark,
    Light,
    System,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Dark
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    Default,
    AcceptEdits,
    BypassPermissions,
    DontAsk,
    Plan,
    Auto,
    Bubble,
}

impl Default for PermissionMode {
    fn default() -> Self {
        Self::Default
    }
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
            if path.extension().map_or(false, |ext| ext == "json") {
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

    pub fn create_session(&self, name: Option<&str>) -> Session {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

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
