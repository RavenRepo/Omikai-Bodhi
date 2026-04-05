//! Extension API for Theasus
//!
//! Provides traits and types for building extensions.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtensionManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub entry_point: String,
}

#[async_trait]
pub trait Extension: Send + Sync {
    fn manifest(&self) -> &ExtensionManifest;
    async fn initialize(&mut self) -> anyhow::Result<()>;
    async fn shutdown(&mut self) -> anyhow::Result<()>;
    fn handle_message(&self, msg: &[u8]) -> anyhow::Result<Vec<u8>>;
}

pub type DynExtension = Box<dyn Extension>;

#[derive(Default)]
pub struct ExtensionRegistry {
    extensions: HashMap<String, DynExtension>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, ext: DynExtension) {
        let name = ext.manifest().name.clone();
        self.extensions.insert(name, ext);
    }

    pub fn get(&self, name: &str) -> Option<&DynExtension> {
        self.extensions.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut DynExtension> {
        self.extensions.get_mut(name)
    }

    pub fn unregister(&mut self, name: &str) -> Option<DynExtension> {
        self.extensions.remove(name)
    }

    pub fn list(&self) -> Vec<&str> {
        self.extensions.keys().map(|s| s.as_str()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }

    pub fn len(&self) -> usize {
        self.extensions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> ExtensionManifest {
        ExtensionManifest {
            name: "test-extension".to_string(),
            version: "1.0.0".to_string(),
            description: "A test extension".to_string(),
            author: Some("Test Author".to_string()),
            entry_point: "main.wasm".to_string(),
        }
    }

    #[test]
    fn test_manifest_serialization() {
        let manifest = sample_manifest();

        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: ExtensionManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(manifest, deserialized);
    }

    #[test]
    fn test_manifest_json_format() {
        let manifest = sample_manifest();
        let json = serde_json::to_value(&manifest).unwrap();

        assert_eq!(json["name"], "test-extension");
        assert_eq!(json["version"], "1.0.0");
        assert_eq!(json["description"], "A test extension");
        assert_eq!(json["author"], "Test Author");
        assert_eq!(json["entry_point"], "main.wasm");
    }

    #[test]
    fn test_manifest_without_author() {
        let manifest = ExtensionManifest {
            name: "minimal".to_string(),
            version: "0.1.0".to_string(),
            description: "Minimal extension".to_string(),
            author: None,
            entry_point: "index.wasm".to_string(),
        };

        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: ExtensionManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(manifest, deserialized);
        assert!(deserialized.author.is_none());
    }

    struct MockExtension {
        manifest: ExtensionManifest,
        initialized: bool,
    }

    impl MockExtension {
        fn new(name: &str) -> Self {
            Self {
                manifest: ExtensionManifest {
                    name: name.to_string(),
                    version: "1.0.0".to_string(),
                    description: format!("{} extension", name),
                    author: None,
                    entry_point: "main.wasm".to_string(),
                },
                initialized: false,
            }
        }
    }

    #[async_trait]
    impl Extension for MockExtension {
        fn manifest(&self) -> &ExtensionManifest {
            &self.manifest
        }

        async fn initialize(&mut self) -> anyhow::Result<()> {
            self.initialized = true;
            Ok(())
        }

        async fn shutdown(&mut self) -> anyhow::Result<()> {
            self.initialized = false;
            Ok(())
        }

        fn handle_message(&self, msg: &[u8]) -> anyhow::Result<Vec<u8>> {
            Ok(msg.to_vec())
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = ExtensionRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = ExtensionRegistry::new();
        let ext = Box::new(MockExtension::new("test"));

        registry.register(ext);

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.get("test").is_some());
        assert_eq!(registry.get("test").unwrap().manifest().name, "test");
    }

    #[test]
    fn test_registry_list() {
        let mut registry = ExtensionRegistry::new();
        registry.register(Box::new(MockExtension::new("alpha")));
        registry.register(Box::new(MockExtension::new("beta")));
        registry.register(Box::new(MockExtension::new("gamma")));

        let mut names = registry.list();
        names.sort();

        assert_eq!(names, vec!["alpha", "beta", "gamma"]);
    }

    #[test]
    fn test_registry_unregister() {
        let mut registry = ExtensionRegistry::new();
        registry.register(Box::new(MockExtension::new("removable")));

        assert!(registry.get("removable").is_some());

        let removed = registry.unregister("removable");
        assert!(removed.is_some());
        assert!(registry.get("removable").is_none());
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let registry = ExtensionRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_extension_handle_message() {
        let ext = MockExtension::new("echo");
        let input = b"hello world";
        let output = ext.handle_message(input).unwrap();
        assert_eq!(output, input.to_vec());
    }

    #[tokio::test]
    async fn test_extension_lifecycle() {
        let mut ext = MockExtension::new("lifecycle");

        assert!(!ext.initialized);

        ext.initialize().await.unwrap();
        assert!(ext.initialized);

        ext.shutdown().await.unwrap();
        assert!(!ext.initialized);
    }
}
