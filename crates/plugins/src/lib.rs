//! # Theasus Plugin System
//!
//! Provides dynamic plugin loading for extending Theasus with custom tools,
//! commands, and integrations. Supports both native (dylib) and WASM plugins.
//!
//! ## Features
//!
//! - **PluginManifest**: Metadata describing a plugin's capabilities
//! - **Plugin Trait**: Interface for implementing plugins
//! - **PluginManager**: Discovers, loads, and manages plugin lifecycle
//! - **Hot Reload**: Supports dynamic loading/unloading of plugins
//!
//! ## Example
//!
//! ```rust,ignore
//! use theasus_plugins::{PluginManager, PluginManifest, Plugin, PluginContext};
//! use async_trait::async_trait;
//!
//! struct MyPlugin;
//!
//! #[async_trait]
//! impl Plugin for MyPlugin {
//!     fn manifest(&self) -> &PluginManifest {
//!         // Return plugin metadata
//!     }
//!     
//!     async fn on_load(&self, ctx: &PluginContext) -> Result<()> {
//!         // Initialize plugin
//!         Ok(())
//!     }
//! }
//!
//! let mut manager = PluginManager::new();
//! manager.register(Box::new(MyPlugin)).await?;
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use theasus_core::Result;
use theasus_tools::{Tool, ToolDefinition, ToolRegistry};
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// Plugin Metadata
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginCapabilities {
    #[serde(default)]
    pub tools: bool,
    #[serde(default)]
    pub commands: bool,
    #[serde(default)]
    pub agents: bool,
    #[serde(default)]
    pub filesystem: bool,
    #[serde(default)]
    pub network: bool,
}

// ============================================================================
// Plugin Trait
// ============================================================================

#[async_trait]
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;

    async fn on_load(&self, context: &mut PluginContext) -> Result<()>;

    async fn on_unload(&self, context: &mut PluginContext) -> Result<()>;

    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![]
    }

    fn on_message(&self, _message: &PluginMessage) -> Option<PluginMessage> {
        None
    }
}

// ============================================================================
// Plugin Context
// ============================================================================

pub struct PluginContext {
    pub plugin_id: Uuid,
    pub data_dir: PathBuf,
    pub tool_registry: Arc<ToolRegistry>,
    pub settings: HashMap<String, serde_json::Value>,
}

impl PluginContext {
    pub fn new(plugin_id: Uuid, data_dir: PathBuf, tool_registry: Arc<ToolRegistry>) -> Self {
        Self { plugin_id, data_dir, tool_registry, settings: HashMap::new() }
    }

    pub fn get_setting<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.settings.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn set_setting<T: Serialize>(&mut self, key: &str, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.settings.insert(key.to_string(), v);
        }
    }
}

// ============================================================================
// Plugin Messages
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginMessage {
    ToolExecuted { tool_name: String, success: bool },
    ConversationUpdated { message_count: usize },
    SettingsChanged { key: String },
    Custom { event: String, data: serde_json::Value },
}

// ============================================================================
// Plugin State
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum PluginState {
    Discovered,
    Loading,
    Loaded,
    Failed(String),
    Unloading,
    Unloaded,
}

struct LoadedPlugin {
    manifest: PluginManifest,
    plugin: Arc<dyn Plugin>,
    state: PluginState,
    context: PluginContext,
}

// ============================================================================
// Plugin Manager
// ============================================================================

pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, LoadedPlugin>>>,
    plugin_dirs: Vec<PathBuf>,
    tool_registry: Arc<ToolRegistry>,
    data_dir: PathBuf,
}

impl PluginManager {
    pub fn new(tool_registry: Arc<ToolRegistry>, data_dir: PathBuf) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_dirs: vec![],
            tool_registry,
            data_dir,
        }
    }

    pub fn add_plugin_dir(&mut self, dir: PathBuf) {
        self.plugin_dirs.push(dir);
    }

    pub async fn discover_plugins(&self) -> Result<Vec<PluginManifest>> {
        let mut manifests = vec![];

        for dir in &self.plugin_dirs {
            if !dir.exists() {
                continue;
            }

            let entries = std::fs::read_dir(dir)?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let manifest_path = path.join("plugin.json");
                    if manifest_path.exists() {
                        let content = std::fs::read_to_string(&manifest_path)?;
                        if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                            manifests.push(manifest);
                        }
                    }
                }
            }
        }

        Ok(manifests)
    }

    pub async fn load_plugin(&self, plugin: Arc<dyn Plugin>) -> Result<()> {
        let manifest = plugin.manifest().clone();
        let name = manifest.name.clone();

        tracing::info!("Loading plugin: {} v{}", name, manifest.version);

        let plugin_data_dir = self.data_dir.join("plugins").join(&name);
        std::fs::create_dir_all(&plugin_data_dir)?;

        let mut context =
            PluginContext::new(Uuid::new_v4(), plugin_data_dir, self.tool_registry.clone());

        plugin.on_load(&mut context).await?;

        let loaded = LoadedPlugin { manifest, plugin, state: PluginState::Loaded, context };

        self.plugins.write().await.insert(name.clone(), loaded);

        tracing::info!("Plugin loaded: {}", name);
        Ok(())
    }

    pub async fn unload_plugin(&self, name: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;

        if let Some(mut loaded) = plugins.remove(name) {
            tracing::info!("Unloading plugin: {}", name);
            loaded.state = PluginState::Unloading;
            loaded.plugin.on_unload(&mut loaded.context).await?;
            tracing::info!("Plugin unloaded: {}", name);
        }

        Ok(())
    }

    pub async fn get_plugin(&self, name: &str) -> Option<Arc<dyn Plugin>> {
        self.plugins.read().await.get(name).map(|p| p.plugin.clone())
    }

    pub async fn list_plugins(&self) -> Vec<PluginManifest> {
        self.plugins.read().await.values().map(|p| p.manifest.clone()).collect()
    }

    pub async fn get_plugin_state(&self, name: &str) -> Option<PluginState> {
        self.plugins.read().await.get(name).map(|p| p.state.clone())
    }

    pub async fn broadcast_message(&self, message: PluginMessage) {
        let plugins = self.plugins.read().await;
        for loaded in plugins.values() {
            if loaded.state == PluginState::Loaded {
                loaded.plugin.on_message(&message);
            }
        }
    }

    pub async fn get_all_tools(&self) -> Vec<ToolDefinition> {
        let plugins = self.plugins.read().await;
        let mut tools = vec![];

        for loaded in plugins.values() {
            if loaded.state == PluginState::Loaded {
                for tool in loaded.plugin.tools() {
                    tools.push(tool.definition());
                }
            }
        }

        tools
    }
}

// ============================================================================
// Built-in Plugin Loader (for native dylib plugins)
// ============================================================================

#[cfg(feature = "native-plugins")]
pub mod native {
    use super::*;
    use libloading::{Library, Symbol};

    type CreatePluginFn = unsafe extern "C" fn() -> *mut dyn Plugin;
    type DestroyPluginFn = unsafe extern "C" fn(*mut dyn Plugin);

    pub struct NativePluginLoader {
        libraries: HashMap<String, Library>,
    }

    impl NativePluginLoader {
        pub fn new() -> Self {
            Self { libraries: HashMap::new() }
        }

        pub unsafe fn load(&mut self, path: &Path) -> Result<Arc<dyn Plugin>> {
            let library = Library::new(path)?;

            let create_fn: Symbol<CreatePluginFn> = library.get(b"create_plugin")?;
            let plugin_ptr = create_fn();
            let plugin = Arc::from_raw(plugin_ptr);

            let name = plugin.manifest().name.clone();
            self.libraries.insert(name, library);

            Ok(plugin)
        }
    }

    impl Default for NativePluginLoader {
        fn default() -> Self {
            Self::new()
        }
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),

    #[error("Plugin load failed: {0}")]
    LoadFailed(String),

    #[error("Plugin version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },

    #[error("Dependency not satisfied: {0}")]
    DependencyNotSatisfied(String),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
}

pub type PluginResult<T> = std::result::Result<T, PluginError>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        manifest: PluginManifest,
    }

    impl TestPlugin {
        fn new() -> Self {
            Self {
                manifest: PluginManifest {
                    name: "test-plugin".to_string(),
                    version: "1.0.0".to_string(),
                    description: "A test plugin".to_string(),
                    author: Some("Test".to_string()),
                    homepage: None,
                    license: Some("MIT".to_string()),
                    keywords: vec!["test".to_string()],
                    dependencies: vec![],
                    capabilities: PluginCapabilities::default(),
                },
            }
        }
    }

    #[async_trait]
    impl Plugin for TestPlugin {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }

        async fn on_load(&self, _context: &mut PluginContext) -> Result<()> {
            Ok(())
        }

        async fn on_unload(&self, _context: &mut PluginContext) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_plugin_manifest() {
        let plugin = TestPlugin::new();
        assert_eq!(plugin.manifest().name, "test-plugin");
        assert_eq!(plugin.manifest().version, "1.0.0");
    }

    #[test]
    fn test_plugin_state() {
        assert_eq!(PluginState::Discovered, PluginState::Discovered);
        assert_ne!(PluginState::Loaded, PluginState::Unloaded);
    }

    #[tokio::test]
    async fn test_plugin_manager() {
        let registry = Arc::new(ToolRegistry::new());
        let manager = PluginManager::new(registry, PathBuf::from("/tmp/test-plugins"));

        let plugins = manager.list_plugins().await;
        assert!(plugins.is_empty());
    }

    #[tokio::test]
    async fn test_load_plugin() {
        let registry = Arc::new(ToolRegistry::new());
        let manager = PluginManager::new(registry, PathBuf::from("/tmp/test-plugins"));

        let plugin = Arc::new(TestPlugin::new());
        manager.load_plugin(plugin).await.unwrap();

        let plugins = manager.list_plugins().await;
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "test-plugin");

        let state = manager.get_plugin_state("test-plugin").await;
        assert_eq!(state, Some(PluginState::Loaded));
    }
}
