use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use theasus_mcp::McpServerConfig;

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
