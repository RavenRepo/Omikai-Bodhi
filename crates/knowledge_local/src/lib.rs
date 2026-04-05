//! # Theasus Knowledge Local
//!
//! Filesystem-backed implementation of the `KnowledgeProvider` trait.
//!
//! Stores knowledge entries as individual JSON files organized by domain:
//!
//! ```text
//! <root>/
//! ├── architecture/
//! │   ├── <uuid>.json
//! │   └── <uuid>.json
//! ├── security/
//! │   └── <uuid>.json
//! └── rust-patterns/
//!     └── <uuid>.json
//! ```
//!
//! ## Design Rationale
//!
//! - **One file per entry**: Simplifies CRUD operations — no file locking, no
//!   append-only logs, no index corruption. Each entry is independently
//!   readable and writable.
//! - **Domain directories**: Natural organization that maps to filesystem
//!   browsing. A developer can `ls ~/.omikai/bodhi/knowledge/security/` to
//!   see all security knowledge.
//! - **JSON format**: Human-readable and debuggable. Agents can inspect their
//!   own knowledge store. Serde handles serialization.
//! - **Full scan for queries**: Acceptable for the filesystem scale (~100s of
//!   entries). Future backends (Neo4j, Qdrant) will provide indexed queries
//!   for larger knowledge bases.

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use theasus_knowledge::{
    DomainContext, KnowledgeEntry, KnowledgeError, KnowledgeProvider, KnowledgeQuery,
};
use uuid::Uuid;

/// Filesystem-backed knowledge provider.
///
/// Stores entries as `<root>/<domain>/<uuid>.json`. All operations are
/// async via `tokio::fs` to avoid blocking the runtime on I/O.
///
/// ## Example
///
/// ```rust,ignore
/// use theasus_knowledge_local::LocalKnowledgeProvider;
///
/// let provider = LocalKnowledgeProvider::new("~/.omikai/bodhi/knowledge");
/// provider.initialize().await?;
///
/// let entry = KnowledgeEntry::new("security", "SQL injection", "...", EntryType::Rule);
/// provider.store(entry).await?;
/// ```
pub struct LocalKnowledgeProvider {
    root: PathBuf,
}

impl LocalKnowledgeProvider {
    /// Create a new filesystem-backed knowledge provider.
    ///
    /// The `root` path is where all knowledge entries will be stored.
    /// Call `initialize()` to create the directory structure.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Ensure the root directory exists.
    ///
    /// Called automatically before write operations, but can be called
    /// explicitly during application startup for fail-fast behavior.
    pub async fn initialize(&self) -> Result<(), KnowledgeError> {
        tokio::fs::create_dir_all(&self.root).await?;
        Ok(())
    }

    /// Get the filesystem path for a knowledge entry.
    ///
    /// Path format: `<root>/<domain>/<uuid>.json`
    ///
    /// The domain is sanitized to prevent path traversal:
    /// slashes and dots are replaced with hyphens.
    fn entry_path(&self, domain: &str, id: Uuid) -> PathBuf {
        let safe_domain = domain
            .replace(['/', '\\'], "-")
            .replace("..", "-");
        self.root.join(&safe_domain).join(format!("{}.json", id))
    }

    /// Get the directory path for a domain.
    #[allow(dead_code)]
    fn domain_path(&self, domain: &str) -> PathBuf {
        let safe_domain = domain
            .replace(['/', '\\'], "-")
            .replace("..", "-");
        self.root.join(&safe_domain)
    }

    /// Load all entries from all domain directories.
    ///
    /// Walks the entire `<root>/` tree, deserializing every `.json` file.
    /// Errors on individual files are logged and skipped (graceful degradation).
    async fn load_all(&self) -> Result<Vec<KnowledgeEntry>, KnowledgeError> {
        let mut entries = Vec::new();

        // Read root directory for domain subdirs
        let mut root_entries = match tokio::fs::read_dir(&self.root).await {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(entries),
            Err(e) => return Err(KnowledgeError::IoError(e)),
        };

        while let Some(domain_dir) = root_entries.next_entry().await? {
            let domain_path = domain_dir.path();
            if !domain_path.is_dir() {
                continue;
            }

            let mut domain_entries = match tokio::fs::read_dir(&domain_path).await {
                Ok(entries) => entries,
                Err(e) => {
                    tracing::warn!("Failed to read domain directory {:?}: {}", domain_path, e);
                    continue;
                }
            };

            while let Some(entry_file) = domain_entries.next_entry().await? {
                let file_path = entry_file.path();
                if file_path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }

                match self.load_entry_from_file(&file_path).await {
                    Ok(entry) => entries.push(entry),
                    Err(e) => {
                        tracing::warn!("Failed to load knowledge entry {:?}: {}", file_path, e);
                    }
                }
            }
        }

        Ok(entries)
    }

    /// Load a single entry from a JSON file.
    async fn load_entry_from_file(&self, path: &Path) -> Result<KnowledgeEntry, KnowledgeError> {
        let content = tokio::fs::read_to_string(path).await?;
        serde_json::from_str(&content).map_err(|e| {
            KnowledgeError::SerializationError(format!(
                "Failed to deserialize {:?}: {}",
                path, e
            ))
        })
    }

    /// Write a single entry to its JSON file.
    ///
    /// Creates the domain directory if it doesn't exist.
    async fn write_entry(&self, entry: &KnowledgeEntry) -> Result<(), KnowledgeError> {
        let path = self.entry_path(&entry.domain, entry.id);

        // Ensure domain directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(entry).map_err(|e| {
            KnowledgeError::SerializationError(format!("Failed to serialize entry: {}", e))
        })?;

        tokio::fs::write(&path, content).await?;
        Ok(())
    }

    /// Find the file path for an entry by UUID (searches all domains).
    ///
    /// Since we don't maintain an index, finding an entry by ID requires
    /// scanning domain directories. Returns `None` if not found.
    async fn find_entry_path(&self, id: Uuid) -> Result<Option<PathBuf>, KnowledgeError> {
        let filename = format!("{}.json", id);

        let mut root_entries = match tokio::fs::read_dir(&self.root).await {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(KnowledgeError::IoError(e)),
        };

        while let Some(domain_dir) = root_entries.next_entry().await? {
            let candidate = domain_dir.path().join(&filename);
            if tokio::fs::try_exists(&candidate).await.unwrap_or(false) {
                return Ok(Some(candidate));
            }
        }

        Ok(None)
    }
}

#[async_trait]
impl KnowledgeProvider for LocalKnowledgeProvider {
    /// Query entries by loading all files and applying in-memory filters.
    ///
    /// Filter application order:
    /// 1. Domain filter (skip non-matching domain directories at FS level)
    /// 2. Tag intersection filter
    /// 3. Entry type filter
    /// 4. Confidence threshold filter
    /// 5. Full-text search (case-insensitive substring)
    /// 6. Result limit
    ///
    /// For the filesystem scale (~100s of entries), full scan is acceptable.
    /// Future backends will use indexed queries.
    async fn query(
        &self,
        query: KnowledgeQuery,
    ) -> Result<Vec<KnowledgeEntry>, KnowledgeError> {
        let all_entries = self.load_all().await?;

        let mut results: Vec<KnowledgeEntry> = all_entries
            .into_iter()
            .filter(|entry| query.matches(entry))
            .collect();

        // Sort by confidence descending (most reliable knowledge first)
        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    /// Store a new knowledge entry as a JSON file.
    ///
    /// File path: `<root>/<domain>/<uuid>.json`
    /// Creates the domain directory if it doesn't exist.
    async fn store(&self, entry: KnowledgeEntry) -> Result<Uuid, KnowledgeError> {
        let id = entry.id;
        self.write_entry(&entry).await?;
        tracing::info!(
            id = %id,
            domain = %entry.domain,
            title = %entry.title,
            "Stored knowledge entry"
        );
        Ok(id)
    }

    /// Update an existing entry by overwriting its JSON file.
    ///
    /// Finds the entry by ID, updates the `updated_at` timestamp,
    /// handles domain changes by moving the file.
    async fn update(&self, id: Uuid, mut entry: KnowledgeEntry) -> Result<(), KnowledgeError> {
        // Find existing entry to handle domain changes
        let existing_path = self
            .find_entry_path(id)
            .await?
            .ok_or(KnowledgeError::NotFound(id))?;

        // Update timestamp
        entry.updated_at = chrono::Utc::now();
        entry.id = id;

        let new_path = self.entry_path(&entry.domain, id);

        // If domain changed, remove old file
        if existing_path != new_path {
            tokio::fs::remove_file(&existing_path).await.ok();
        }

        self.write_entry(&entry).await?;

        tracing::info!(id = %id, "Updated knowledge entry");
        Ok(())
    }

    /// Delete an entry by removing its JSON file.
    ///
    /// Idempotent: deleting a non-existent entry succeeds silently.
    /// Empty domain directories are NOT cleaned up (minimal overhead).
    async fn delete(&self, id: Uuid) -> Result<(), KnowledgeError> {
        if let Some(path) = self.find_entry_path(id).await? {
            tokio::fs::remove_file(&path).await.ok();
            tracing::info!(id = %id, "Deleted knowledge entry");
        }
        Ok(())
    }

    /// Execute multiple queries and compile results into a `DomainContext`.
    ///
    /// Process:
    /// 1. Execute each query independently
    /// 2. Merge results, deduplicating by entry ID
    /// 3. Compile into a formatted prompt string via `DomainContext::compile()`
    /// 4. Respect the `max_tokens` limit for prompt budget
    async fn compile_context(
        &self,
        queries: &[KnowledgeQuery],
        max_tokens: Option<usize>,
    ) -> Result<DomainContext, KnowledgeError> {
        if queries.is_empty() {
            return Ok(DomainContext::empty());
        }

        let mut all_entries = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for query in queries {
            let results = self.query(query.clone()).await?;
            for entry in results {
                if seen_ids.insert(entry.id) {
                    all_entries.push(entry);
                }
            }
        }

        Ok(DomainContext::compile(all_entries, max_tokens))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use theasus_knowledge::{EntryType, KnowledgeSource};

    /// Create a test provider with a temporary directory.
    fn test_provider(dir: &tempfile::TempDir) -> LocalKnowledgeProvider {
        LocalKnowledgeProvider::new(dir.path())
    }

    /// Create a test entry with sensible defaults.
    fn test_entry(domain: &str, title: &str, entry_type: EntryType) -> KnowledgeEntry {
        KnowledgeEntry::new(domain, title, format!("Content for {}", title), entry_type)
    }

    #[tokio::test]
    async fn test_initialize_creates_root_directory() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("knowledge");
        let provider = LocalKnowledgeProvider::new(&root);

        provider.initialize().await.unwrap();
        assert!(root.exists());
    }

    #[tokio::test]
    async fn test_store_and_query_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        let entry = test_entry("architecture", "ADR-001", EntryType::ArchitectureDecision)
            .with_tags(vec!["traits".into(), "io".into()]);

        let id = provider.store(entry.clone()).await.unwrap();

        // Query by domain
        let results = provider
            .query(KnowledgeQuery::new().with_domains(vec!["architecture".into()]))
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
        assert_eq!(results[0].title, "ADR-001");
        assert_eq!(results[0].tags, vec!["traits", "io"]);
    }

    #[tokio::test]
    async fn test_store_creates_domain_directory() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        let entry = test_entry("security", "OWASP", EntryType::Rule);
        provider.store(entry).await.unwrap();

        assert!(dir.path().join("security").exists());
    }

    #[tokio::test]
    async fn test_query_empty_store() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        let results = provider.query(KnowledgeQuery::new()).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_query_by_tags() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        provider
            .store(
                test_entry("security", "SQL Injection", EntryType::Rule)
                    .with_tags(vec!["owasp".into(), "sql".into()]),
            )
            .await
            .unwrap();

        provider
            .store(
                test_entry("security", "XSS Prevention", EntryType::Rule)
                    .with_tags(vec!["owasp".into(), "xss".into()]),
            )
            .await
            .unwrap();

        // Query for sql tag - should only match first
        let results = provider
            .query(KnowledgeQuery::new().with_tags(vec!["sql".into()]))
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "SQL Injection");

        // Query for owasp tag - should match both
        let results = provider
            .query(KnowledgeQuery::new().with_tags(vec!["owasp".into()]))
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_query_by_entry_type() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        provider
            .store(test_entry("arch", "ADR-001", EntryType::ArchitectureDecision))
            .await
            .unwrap();
        provider
            .store(test_entry("arch", "No unwrap", EntryType::Rule))
            .await
            .unwrap();
        provider
            .store(test_entry("arch", "Error pattern", EntryType::Pattern))
            .await
            .unwrap();

        let results = provider
            .query(KnowledgeQuery::new().with_entry_types(vec![EntryType::Rule]))
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "No unwrap");
    }

    #[tokio::test]
    async fn test_query_by_confidence() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        provider
            .store(test_entry("test", "High conf", EntryType::Rule).with_confidence(0.95))
            .await
            .unwrap();
        provider
            .store(test_entry("test", "Low conf", EntryType::Observation).with_confidence(0.3))
            .await
            .unwrap();

        let results = provider
            .query(KnowledgeQuery::new().with_min_confidence(0.8))
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "High conf");
    }

    #[tokio::test]
    async fn test_query_text_search() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        provider
            .store(KnowledgeEntry::new(
                "arch",
                "Trait Abstraction",
                "Every external dependency must be behind a trait",
                EntryType::ArchitectureDecision,
            ))
            .await
            .unwrap();

        provider
            .store(KnowledgeEntry::new(
                "arch",
                "Error Handling",
                "Use thiserror for libraries, anyhow for applications",
                EntryType::Pattern,
            ))
            .await
            .unwrap();

        let results = provider
            .query(KnowledgeQuery::new().with_search_text("trait"))
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Trait Abstraction");
    }

    #[tokio::test]
    async fn test_query_with_limit() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        for i in 0..10 {
            provider
                .store(test_entry("test", &format!("Entry {}", i), EntryType::Observation))
                .await
                .unwrap();
        }

        let results = provider
            .query(KnowledgeQuery::new().with_limit(3))
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_query_combined_filters() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        provider
            .store(
                test_entry("security", "SQL Rule", EntryType::Rule)
                    .with_tags(vec!["owasp".into()])
                    .with_confidence(0.95),
            )
            .await
            .unwrap();

        provider
            .store(
                test_entry("security", "Low Conf Obs", EntryType::Observation)
                    .with_tags(vec!["owasp".into()])
                    .with_confidence(0.3),
            )
            .await
            .unwrap();

        provider
            .store(
                test_entry("architecture", "ADR", EntryType::ArchitectureDecision)
                    .with_confidence(0.9),
            )
            .await
            .unwrap();

        let results = provider
            .query(
                KnowledgeQuery::new()
                    .with_domains(vec!["security".into()])
                    .with_tags(vec!["owasp".into()])
                    .with_min_confidence(0.8),
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "SQL Rule");
    }

    #[tokio::test]
    async fn test_update_entry() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        let entry = test_entry("arch", "ADR-001", EntryType::ArchitectureDecision);
        let id = provider.store(entry).await.unwrap();

        // Update the entry
        let mut updated = test_entry("arch", "ADR-001 (Updated)", EntryType::ArchitectureDecision);
        updated.id = id;
        provider.update(id, updated).await.unwrap();

        let results = provider
            .query(KnowledgeQuery::new().with_domains(vec!["arch".into()]))
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "ADR-001 (Updated)");
    }

    #[tokio::test]
    async fn test_update_nonexistent_entry_fails() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);
        provider.initialize().await.unwrap();

        let entry = test_entry("arch", "Ghost", EntryType::Observation);
        let result = provider.update(Uuid::new_v4(), entry).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_entry() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        let entry = test_entry("arch", "To Delete", EntryType::Observation);
        let id = provider.store(entry).await.unwrap();

        // Verify it exists
        let results = provider.query(KnowledgeQuery::new()).await.unwrap();
        assert_eq!(results.len(), 1);

        // Delete
        provider.delete(id).await.unwrap();

        // Verify it's gone
        let results = provider.query(KnowledgeQuery::new()).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_delete_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);
        provider.initialize().await.unwrap();

        // Deleting a non-existent entry should succeed
        provider.delete(Uuid::new_v4()).await.unwrap();
    }

    #[tokio::test]
    async fn test_compile_context_empty() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        let context = provider.compile_context(&[], None).await.unwrap();
        assert!(context.entries.is_empty());
        assert!(context.compiled_prompt.is_empty());
    }

    #[tokio::test]
    async fn test_compile_context_deduplicates() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        // Store entry that matches multiple queries
        provider
            .store(
                test_entry("security", "SQL Rule", EntryType::Rule)
                    .with_tags(vec!["owasp".into()]),
            )
            .await
            .unwrap();

        // Two queries that both match the same entry
        let queries = vec![
            KnowledgeQuery::new().with_domains(vec!["security".into()]),
            KnowledgeQuery::new().with_tags(vec!["owasp".into()]),
        ];

        let context = provider.compile_context(&queries, None).await.unwrap();
        assert_eq!(context.entries.len(), 1); // Deduplicated
    }

    #[tokio::test]
    async fn test_compile_context_with_token_limit() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        for i in 0..20 {
            provider
                .store(KnowledgeEntry::new(
                    "test",
                    format!("Entry {}", i),
                    "A".repeat(200),
                    EntryType::Observation,
                ))
                .await
                .unwrap();
        }

        let queries = vec![KnowledgeQuery::new()];
        let context = provider.compile_context(&queries, Some(100)).await.unwrap();

        assert!(context.entries.len() < 20);
        assert!(context.token_estimate <= 150);
    }

    #[tokio::test]
    async fn test_domain_directory_sanitization() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        // Attempt path traversal in domain name
        let entry = test_entry("../../../etc", "Evil", EntryType::Observation);
        provider.store(entry).await.unwrap();

        // Should be stored in sanitized path, not actual ../../../etc
        let sanitized_path = dir.path().join("------etc");
        assert!(sanitized_path.exists());
    }

    #[tokio::test]
    async fn test_multiple_domains() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        provider
            .store(test_entry("security", "Sec Entry", EntryType::Rule))
            .await
            .unwrap();
        provider
            .store(test_entry("architecture", "Arch Entry", EntryType::ArchitectureDecision))
            .await
            .unwrap();
        provider
            .store(test_entry("patterns", "Pat Entry", EntryType::Pattern))
            .await
            .unwrap();

        // Query all
        let all = provider.query(KnowledgeQuery::new()).await.unwrap();
        assert_eq!(all.len(), 3);

        // Query single domain
        let sec = provider
            .query(KnowledgeQuery::new().with_domains(vec!["security".into()]))
            .await
            .unwrap();
        assert_eq!(sec.len(), 1);
        assert_eq!(sec[0].title, "Sec Entry");
    }

    #[tokio::test]
    async fn test_results_sorted_by_confidence() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        provider
            .store(test_entry("test", "Low", EntryType::Observation).with_confidence(0.3))
            .await
            .unwrap();
        provider
            .store(test_entry("test", "High", EntryType::Rule).with_confidence(0.95))
            .await
            .unwrap();
        provider
            .store(test_entry("test", "Mid", EntryType::Pattern).with_confidence(0.7))
            .await
            .unwrap();

        let results = provider.query(KnowledgeQuery::new()).await.unwrap();

        assert_eq!(results[0].title, "High");
        assert_eq!(results[1].title, "Mid");
        assert_eq!(results[2].title, "Low");
    }

    #[tokio::test]
    async fn test_agent_discovered_entry() {
        let dir = tempfile::tempdir().unwrap();
        let provider = test_provider(&dir);

        let entry = KnowledgeEntry::from_agent_output(
            "rust-patterns",
            "All tests use tokio::test",
            "Discovered: every integration test uses #[tokio::test] runtime",
            EntryType::Observation,
        );

        let id = provider.store(entry).await.unwrap();

        let results = provider.query(KnowledgeQuery::new()).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
        assert_eq!(results[0].source, KnowledgeSource::AgentDiscovered);
        assert_eq!(results[0].confidence, 0.5);
    }
}
