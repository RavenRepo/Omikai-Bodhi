//! # Theasus Knowledge Layer
//!
//! Trait abstraction for the agent knowledge system. This crate defines the
//! interface for storing, querying, and compiling domain-specific knowledge
//! that agents use to become contextualized experts rather than stateless
//! personalities.
//!
//! ## Architecture
//!
//! The knowledge layer follows the Bodhi trait-first pattern:
//! - This crate (`knowledge`) defines the trait and types only
//! - Implementation crates (e.g., `knowledge_local`) provide concrete backends
//!
//! ## Core Concepts
//!
//! - **KnowledgeEntry**: A single piece of domain knowledge (ADR, pattern, rule)
//! - **KnowledgeQuery**: A structured query against the knowledge store
//! - **DomainContext**: Compiled knowledge ready for injection into agent prompts
//! - **KnowledgeProvider**: The trait that all knowledge backends implement
//!
//! ## Example
//!
//! ```rust,ignore
//! use theasus_knowledge::{KnowledgeProvider, KnowledgeQuery, EntryType};
//!
//! // Query all architecture rules before agent execution
//! let query = KnowledgeQuery::new()
//!     .with_domains(vec!["architecture".into()])
//!     .with_entry_types(vec![EntryType::Rule, EntryType::ArchitectureDecision]);
//!
//! let entries = provider.query(query).await?;
//! let context = provider.compile_context(&[query]).await?;
//! ```
//!
//! ## Execution Contracts
//!
//! The contract system enables pre-committed success criteria for agent tasks:
//!
//! ```rust,ignore
//! use theasus_knowledge::contract::{ExecutionContract, VerifiableCriterion, VerificationMethod};
//!
//! let contract = ExecutionContract::new(task_id)
//!     .with_criterion(
//!         VerifiableCriterion::new("Code compiles")
//!             .with_method(VerificationMethod::Build)
//!             .blocking()
//!     )
//!     .with_confidence_floor(0.7);
//! ```

pub mod contract;
pub mod execution;
pub mod promotion;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export key contract types for convenience
pub use contract::{
    ContractEvaluation, CriterionResult, ExecutionContract, VerifiableCriterion, VerificationMethod,
};
pub use execution::{
    CaptureProcessingResult, CompiledExecutionContext, Observation, ObservationType,
    PostExecutionCapture, PreExecutionQuery, TaskResult,
};
pub use promotion::{ContextRecord, EvaluationBuilder, IndependenceTracker, PromotionEvaluation};

// ============================================================================
// Knowledge Entry — The atomic unit of domain knowledge
// ============================================================================

/// A single piece of domain knowledge.
///
/// This is the foundational data type of the knowledge layer. Each entry
/// represents one discrete fact, decision, pattern, or rule that an agent
/// can reference during execution.
///
/// Entries are organized by `domain` (broad category like "architecture" or
/// "security") and further classified by `tags` for fine-grained filtering.
///
/// The `confidence` field (0.0–1.0) allows the system to weight knowledge
/// by reliability. Agent-discovered observations start at lower confidence
/// than human-authored rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    /// Unique identifier for this entry.
    pub id: Uuid,

    /// The broad knowledge domain this entry belongs to.
    ///
    /// Examples: "architecture", "security", "rust-patterns", "project-history"
    ///
    /// Domains map to logical groupings that agents declare interest in via
    /// their `KnowledgeBinding.domains` field.
    pub domain: String,

    /// Fine-grained classification tags for filtering.
    ///
    /// Examples: ["workspace", "dependencies", "cargo"] or ["owasp", "xss"]
    ///
    /// Tags support intersection-based query filtering: a query with
    /// tags ["workspace", "cargo"] matches entries that have BOTH tags.
    pub tags: Vec<String>,

    /// Human-readable title summarizing this knowledge entry.
    ///
    /// Should be concise enough to appear in a compiled context listing.
    /// Example: "ADR-001: Use trait abstraction for all external I/O"
    pub title: String,

    /// The full knowledge content in Markdown format.
    ///
    /// This is the body that gets injected into agent prompts when the
    /// entry matches a pre-execution query. Markdown formatting is
    /// preserved during context compilation.
    pub content: String,

    /// The structural type of this knowledge entry.
    ///
    /// Classification drives how entries are presented in compiled context
    /// and how they're weighted during query result ranking.
    pub entry_type: EntryType,

    /// Confidence score from 0.0 (uncertain) to 1.0 (verified fact).
    ///
    /// Scoring guidelines:
    /// - 1.0: Human-authored rules, verified ADRs
    /// - 0.8: Agent-discovered patterns confirmed by multiple observations
    /// - 0.5: Single agent observation, unverified
    /// - 0.3: Inferred from indirect evidence
    ///
    /// The `min_confidence` filter on `KnowledgeQuery` uses this to exclude
    /// low-quality knowledge from critical agent decisions.
    pub confidence: f32,

    /// Provenance tracking: where this knowledge came from.
    pub source: KnowledgeSource,

    /// When this entry was first created.
    pub created_at: DateTime<Utc>,

    /// When this entry was last modified.
    ///
    /// Updated automatically on any mutation through `KnowledgeProvider::update()`.
    pub updated_at: DateTime<Utc>,
}

impl KnowledgeEntry {
    /// Create a new knowledge entry with generated ID and timestamps.
    ///
    /// # Arguments
    ///
    /// * `domain` - The knowledge domain (e.g., "architecture")
    /// * `title` - A concise title for this entry
    /// * `content` - The full Markdown content
    /// * `entry_type` - Structural classification
    pub fn new(
        domain: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
        entry_type: EntryType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            domain: domain.into(),
            tags: Vec::new(),
            title: title.into(),
            content: content.into(),
            entry_type,
            confidence: 0.8,
            source: KnowledgeSource::Manual,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add tags to this entry (builder pattern).
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set the confidence score (builder pattern).
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set the knowledge source (builder pattern).
    pub fn with_source(mut self, source: KnowledgeSource) -> Self {
        self.source = source;
        self
    }

    /// Create an entry from agent output for post-execution capture.
    ///
    /// Automatically sets source to `AgentDiscovered` and confidence to 0.5
    /// (agent observations start at moderate confidence until verified).
    pub fn from_agent_output(
        domain: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
        entry_type: EntryType,
    ) -> Self {
        Self::new(domain, title, content, entry_type)
            .with_source(KnowledgeSource::AgentDiscovered)
            .with_confidence(0.5)
    }
}

// ============================================================================
// Entry Classification Types
// ============================================================================

/// Structural classification of a knowledge entry.
///
/// This enum drives how entries are presented in compiled context and
/// how they're weighted during query result ranking. Each variant maps
/// to a distinct role in the agent's reasoning process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    /// Architecture Decision Records — formal decisions with context and rationale.
    ///
    /// These carry the highest weight in compiled context because they represent
    /// deliberate, documented choices that constrain the solution space.
    ///
    /// Example: "ADR-001: Use trait abstraction for all external I/O"
    ArchitectureDecision,

    /// Recurring code or design patterns observed in the project.
    ///
    /// Patterns help agents generate code consistent with existing conventions.
    ///
    /// Example: "Error handling pattern: use thiserror for library crates, anyhow for apps"
    Pattern,

    /// One-off decisions with rationale that don't rise to ADR level.
    ///
    /// Example: "Chose reqwest over hyper for HTTP client — simpler API, good enough perf"
    Decision,

    /// Agent-discovered facts about the codebase or project.
    ///
    /// Observations are the lowest-confidence entry type and may be promoted
    /// to Pattern or Decision after verification.
    ///
    /// Example: "All integration tests in this project use tokio::test runtime"
    Observation,

    /// Hard constraints that must never be violated.
    ///
    /// Rules are injected with the highest priority in compiled context and
    /// are formatted as explicit prohibitions or requirements.
    ///
    /// Example: "NEVER pin dependency versions in sub-crate Cargo.toml"
    Rule,

    /// Execution history and outcomes from previous agent runs.
    ///
    /// History entries help agents avoid repeating failed approaches and
    /// build on successful ones.
    ///
    /// Example: "Previous refactoring of QueryEngine took 3 turns, key issue was..."
    History,
}

impl std::fmt::Display for EntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArchitectureDecision => write!(f, "Architecture Decision"),
            Self::Pattern => write!(f, "Pattern"),
            Self::Decision => write!(f, "Decision"),
            Self::Observation => write!(f, "Observation"),
            Self::Rule => write!(f, "Rule"),
            Self::History => write!(f, "History"),
        }
    }
}

// ============================================================================
// Knowledge Source — Provenance tracking
// ============================================================================

/// Provenance of a knowledge entry — where it came from.
///
/// Source tracking is critical for confidence scoring and for understanding
/// the reliability of knowledge an agent is acting on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeSource {
    /// Human-authored entry, typically highest confidence.
    Manual,

    /// Discovered by an agent during execution.
    ///
    /// These entries start at moderate confidence (0.5) and can be
    /// promoted through verification.
    AgentDiscovered,

    /// Imported from an external source (e.g., Constella databases,
    /// project documentation, CI/CD pipelines).
    Imported,

    /// Derived by combining or reasoning over multiple existing entries.
    ///
    /// Inferred knowledge has variable confidence depending on the
    /// quality of the source entries.
    Inferred,
}

impl std::fmt::Display for KnowledgeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manual => write!(f, "Manual"),
            Self::AgentDiscovered => write!(f, "Agent Discovered"),
            Self::Imported => write!(f, "Imported"),
            Self::Inferred => write!(f, "Inferred"),
        }
    }
}

// ============================================================================
// Knowledge Query — Structured search against the knowledge store
// ============================================================================

/// A structured query against the knowledge layer.
///
/// Queries support multiple filter dimensions that are combined with AND
/// logic. An empty query (all fields `None`) returns all entries up to
/// the default limit.
///
/// ## Query Composition
///
/// Filters are applied in this order:
/// 1. `domains` — restrict to specific knowledge domains
/// 2. `tags` — filter by tag intersection
/// 3. `entry_types` — restrict to specific entry types
/// 4. `min_confidence` — exclude low-confidence entries
/// 5. `search_text` — full-text search within title and content
/// 6. `limit` — cap the number of returned results
///
/// ## Example
///
/// ```rust,ignore
/// let query = KnowledgeQuery::new()
///     .with_domains(vec!["security".into()])
///     .with_entry_types(vec![EntryType::Rule])
///     .with_min_confidence(0.8);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KnowledgeQuery {
    /// Filter to entries in these domains. `None` means all domains.
    pub domains: Option<Vec<String>>,

    /// Filter to entries containing ALL of these tags. `None` means no tag filter.
    pub tags: Option<Vec<String>>,

    /// Filter to these entry types. `None` means all types.
    pub entry_types: Option<Vec<EntryType>>,

    /// Full-text search within entry title and content.
    ///
    /// The search strategy depends on the implementation:
    /// - `knowledge_local`: case-insensitive substring match
    /// - Future backends: semantic/vector search
    pub search_text: Option<String>,

    /// Minimum confidence threshold (0.0–1.0). Entries below this are excluded.
    pub min_confidence: Option<f32>,

    /// Maximum number of entries to return. Defaults to implementation-specific limit.
    pub limit: Option<usize>,
}

impl KnowledgeQuery {
    /// Create a new empty query (matches everything).
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by knowledge domains (builder pattern).
    pub fn with_domains(mut self, domains: Vec<String>) -> Self {
        self.domains = Some(domains);
        self
    }

    /// Filter by tags — entries must contain ALL specified tags (builder pattern).
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Filter by entry types (builder pattern).
    pub fn with_entry_types(mut self, types: Vec<EntryType>) -> Self {
        self.entry_types = Some(types);
        self
    }

    /// Set full-text search term (builder pattern).
    pub fn with_search_text(mut self, text: impl Into<String>) -> Self {
        self.search_text = Some(text.into());
        self
    }

    /// Set minimum confidence threshold (builder pattern).
    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = Some(confidence.clamp(0.0, 1.0));
        self
    }

    /// Set result limit (builder pattern).
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Check if an entry matches this query's filters.
    ///
    /// This method is used by implementations to evaluate entries against
    /// the query predicates. All non-None filters must match (AND logic).
    pub fn matches(&self, entry: &KnowledgeEntry) -> bool {
        // Domain filter
        if let Some(ref domains) = self.domains {
            if !domains.contains(&entry.domain) {
                return false;
            }
        }

        // Tag filter (intersection: entry must contain ALL query tags)
        if let Some(ref tags) = self.tags {
            if !tags.iter().all(|t| entry.tags.contains(t)) {
                return false;
            }
        }

        // Entry type filter
        if let Some(ref types) = self.entry_types {
            if !types.contains(&entry.entry_type) {
                return false;
            }
        }

        // Confidence filter
        if let Some(min_conf) = self.min_confidence {
            if entry.confidence < min_conf {
                return false;
            }
        }

        // Full-text search (case-insensitive substring)
        if let Some(ref text) = self.search_text {
            let lower_text = text.to_lowercase();
            let title_match = entry.title.to_lowercase().contains(&lower_text);
            let content_match = entry.content.to_lowercase().contains(&lower_text);
            if !title_match && !content_match {
                return false;
            }
        }

        true
    }
}

// ============================================================================
// Domain Context — Compiled knowledge ready for prompt injection
// ============================================================================

/// Compiled domain knowledge ready for injection into an agent's system prompt.
///
/// This is the output of `KnowledgeProvider::compile_context()`. It contains
/// both the raw entries (for programmatic access) and a pre-formatted prompt
/// string (for direct injection into the LLM system message).
///
/// The `token_estimate` field helps the agent framework respect token budgets
/// when deciding how much context to inject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainContext {
    /// The raw knowledge entries that matched the compilation queries.
    pub entries: Vec<KnowledgeEntry>,

    /// Pre-formatted prompt text ready for injection.
    ///
    /// Format example:
    /// ```text
    /// ## Rules
    /// - NEVER pin dependency versions in sub-crate Cargo.toml
    /// - Use workspace dependencies from root Cargo.toml
    ///
    /// ## Architecture Decisions
    /// ### ADR-001: Trait abstraction for external I/O
    /// Every external dependency (HTTP, filesystem, terminal) must be behind a trait...
    /// ```
    pub compiled_prompt: String,

    /// Approximate token count of `compiled_prompt`.
    ///
    /// Uses the rough heuristic of 1 token ≈ 4 characters for English text.
    /// This is sufficient for budget decisions; precise counting would require
    /// a tokenizer dependency we don't want in the trait crate.
    pub token_estimate: usize,
}

impl DomainContext {
    /// Create an empty domain context with no knowledge entries.
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
            compiled_prompt: String::new(),
            token_estimate: 0,
        }
    }

    /// Compile a set of knowledge entries into a formatted prompt.
    ///
    /// Groups entries by type and formats them for optimal LLM comprehension:
    /// - Rules are presented as bullet-point constraints
    /// - ADRs get title + content blocks
    /// - Patterns and observations are listed with their titles
    ///
    /// The `max_tokens` parameter limits the compiled output size.
    /// Entries are prioritized by type (Rules > ADRs > Patterns > others)
    /// and confidence score within each type.
    pub fn compile(mut entries: Vec<KnowledgeEntry>, max_tokens: Option<usize>) -> Self {
        if entries.is_empty() {
            return Self::empty();
        }

        // Sort: Rules first, then ADRs, then by confidence descending
        entries.sort_by(|a, b| {
            let type_priority = |e: &KnowledgeEntry| -> u8 {
                match e.entry_type {
                    EntryType::Rule => 0,
                    EntryType::ArchitectureDecision => 1,
                    EntryType::Pattern => 2,
                    EntryType::Decision => 3,
                    EntryType::Observation => 4,
                    EntryType::History => 5,
                }
            };
            type_priority(a).cmp(&type_priority(b)).then(
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });

        let max_chars = max_tokens.unwrap_or(usize::MAX / 4) * 4;
        let mut prompt = String::new();
        let mut current_type: Option<&EntryType> = None;
        let mut included_entries = Vec::new();

        for entry in &entries {
            // Check if adding this entry would exceed the budget
            let entry_text = Self::format_entry(entry);
            let section_header = if current_type != Some(&entry.entry_type) {
                format!("\n## {}\n", entry.entry_type)
            } else {
                String::new()
            };

            let addition = format!("{}{}", section_header, entry_text);
            if prompt.len() + addition.len() > max_chars {
                break;
            }

            prompt.push_str(&addition);
            current_type = Some(&entry.entry_type);
            included_entries.push(entry.clone());
        }

        let token_estimate = prompt.len() / 4;
        Self {
            entries: included_entries,
            compiled_prompt: prompt,
            token_estimate,
        }
    }

    /// Format a single entry for prompt inclusion.
    fn format_entry(entry: &KnowledgeEntry) -> String {
        match entry.entry_type {
            EntryType::Rule => {
                format!("- **{}**: {}\n", entry.title, entry.content)
            }
            EntryType::ArchitectureDecision => {
                format!("### {}\n{}\n\n", entry.title, entry.content)
            }
            EntryType::Pattern => {
                format!("- **{}**: {}\n", entry.title, entry.content)
            }
            _ => {
                format!("- {}: {}\n", entry.title, entry.content)
            }
        }
    }

    /// Calculate the average confidence score across all entries.
    ///
    /// Returns 0.0 if there are no entries.
    pub fn average_confidence(&self) -> f32 {
        if self.entries.is_empty() {
            return 0.0;
        }
        let total: f32 = self.entries.iter().map(|e| e.confidence).sum();
        total / self.entries.len() as f32
    }

    /// Get entries with high confidence (above threshold).
    ///
    /// Default threshold is 0.8.
    pub fn high_confidence_entries(&self) -> Vec<&KnowledgeEntry> {
        self.high_confidence_entries_with_threshold(0.8)
    }

    /// Get entries with confidence above a custom threshold.
    pub fn high_confidence_entries_with_threshold(&self, threshold: f32) -> Vec<&KnowledgeEntry> {
        self.entries
            .iter()
            .filter(|e| e.confidence >= threshold)
            .collect()
    }

    /// Determine if deep evaluation should be run for a task.
    ///
    /// Deep evaluation is computationally expensive but provides higher-quality
    /// signal. This method implements difficulty-gated evaluation: only run
    /// deep eval when operating at the capability edge.
    ///
    /// ## Criteria for Deep Evaluation
    ///
    /// Deep evaluation is triggered when EITHER:
    /// 1. Domain confidence is low (< 0.7) — agent lacks expertise
    /// 2. Contract has many blocking criteria (> 3) — task is complex
    ///
    /// This prevents cheap tasks from inflating confidence scores with
    /// low-signal observations.
    ///
    /// ## References
    ///
    /// - Anthropic difficulty-gated evaluation research
    /// - Issue #5: DomainContext and confidence aggregation
    pub fn should_run_deep_evaluation(&self, contract: &ExecutionContract) -> bool {
        let avg_confidence = self.average_confidence();
        let is_low_confidence_domain = avg_confidence < 0.7;
        let is_complex_contract = contract.is_complex();

        is_low_confidence_domain || is_complex_contract
    }

    /// Get the count of entries by type.
    pub fn entry_count_by_type(&self) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();
        for entry in &self.entries {
            *counts.entry(entry.entry_type.to_string()).or_insert(0) += 1;
        }
        counts
    }
}

// ============================================================================
// Knowledge Provider Trait — The core abstraction
// ============================================================================

/// The core knowledge provider trait.
///
/// Every knowledge backend (filesystem, Neo4j, Qdrant, Redis, MCP)
/// implements this trait. The trait is designed to be:
///
/// - **Async**: Knowledge queries may hit databases or network services
/// - **Send + Sync**: Providers are shared across agent tasks via `Arc`
/// - **Backend-agnostic**: The same query works against any implementation
///
/// ## Implementation Contract
///
/// - `query()` MUST apply all non-None filters from `KnowledgeQuery` using AND logic
/// - `store()` MUST set `created_at` and `updated_at` if not already set
/// - `update()` MUST refresh `updated_at` to the current time
/// - `compile_context()` MUST respect the entry priority order (Rules > ADRs > Patterns)
/// - `delete()` MUST be idempotent — deleting a non-existent entry is not an error
///
/// ## Example Implementation
///
/// ```rust,ignore
/// #[async_trait]
/// impl KnowledgeProvider for LocalKnowledge {
///     async fn query(&self, query: KnowledgeQuery) -> Result<Vec<KnowledgeEntry>> {
///         let entries = self.load_all_entries()?;
///         Ok(entries.into_iter().filter(|e| query.matches(e)).collect())
///     }
///     // ...
/// }
/// ```
#[async_trait]
pub trait KnowledgeProvider: Send + Sync {
    /// Query the knowledge store for entries matching the given filters.
    ///
    /// Returns entries sorted by relevance (type priority, then confidence).
    /// An empty query returns all entries up to the implementation's default limit.
    async fn query(&self, query: KnowledgeQuery) -> Result<Vec<KnowledgeEntry>, KnowledgeError>;

    /// Store a new knowledge entry and return its assigned UUID.
    ///
    /// If the entry already has an `id`, it will be preserved. If `id` is nil,
    /// a new UUID will be generated.
    async fn store(&self, entry: KnowledgeEntry) -> Result<Uuid, KnowledgeError>;

    /// Update an existing knowledge entry by ID.
    ///
    /// The `updated_at` field will be automatically refreshed to `Utc::now()`.
    /// Returns an error if the entry does not exist.
    async fn update(&self, id: Uuid, entry: KnowledgeEntry) -> Result<(), KnowledgeError>;

    /// Delete a knowledge entry by ID.
    ///
    /// This operation is idempotent — deleting a non-existent entry succeeds silently.
    async fn delete(&self, id: Uuid) -> Result<(), KnowledgeError>;

    /// Execute multiple queries and compile results into a `DomainContext`.
    ///
    /// This is the primary method called during agent pre-execution.
    /// Results from all queries are merged, deduplicated by ID, and compiled
    /// into a formatted prompt string.
    ///
    /// The `max_tokens` parameter on the returned `DomainContext` should be
    /// respected by the caller (the agent framework) to avoid prompt overflow.
    async fn compile_context(
        &self,
        queries: &[KnowledgeQuery],
        max_tokens: Option<usize>,
    ) -> Result<DomainContext, KnowledgeError>;
}

// ============================================================================
// Errors
// ============================================================================

/// Errors that can occur during knowledge layer operations.
#[derive(Debug, thiserror::Error)]
pub enum KnowledgeError {
    /// The requested knowledge entry was not found.
    #[error("Knowledge entry not found: {0}")]
    NotFound(Uuid),

    /// A storage operation failed (filesystem, database, network).
    #[error("Knowledge storage error: {0}")]
    StorageError(String),

    /// A query was malformed or could not be processed.
    #[error("Invalid knowledge query: {0}")]
    InvalidQuery(String),

    /// Serialization or deserialization failed.
    #[error("Knowledge serialization error: {0}")]
    SerializationError(String),

    /// An I/O error occurred during knowledge operations.
    #[error("Knowledge I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_entry_creation() {
        let entry = KnowledgeEntry::new(
            "architecture",
            "ADR-001: Trait First",
            "All external I/O must be behind a trait.",
            EntryType::ArchitectureDecision,
        );

        assert_eq!(entry.domain, "architecture");
        assert_eq!(entry.title, "ADR-001: Trait First");
        assert_eq!(entry.entry_type, EntryType::ArchitectureDecision);
        assert_eq!(entry.confidence, 0.8);
        assert_eq!(entry.source, KnowledgeSource::Manual);
        assert!(entry.tags.is_empty());
    }

    #[test]
    fn test_knowledge_entry_builder() {
        let entry = KnowledgeEntry::new(
            "security",
            "OWASP SQL Injection",
            "Always use parameterized queries.",
            EntryType::Rule,
        )
        .with_tags(vec!["owasp".into(), "sql".into()])
        .with_confidence(1.0)
        .with_source(KnowledgeSource::Manual);

        assert_eq!(entry.tags, vec!["owasp", "sql"]);
        assert_eq!(entry.confidence, 1.0);
        assert_eq!(entry.source, KnowledgeSource::Manual);
    }

    #[test]
    fn test_confidence_clamping() {
        let entry = KnowledgeEntry::new("test", "title", "content", EntryType::Observation)
            .with_confidence(1.5);
        assert_eq!(entry.confidence, 1.0);

        let entry2 = KnowledgeEntry::new("test", "title", "content", EntryType::Observation)
            .with_confidence(-0.5);
        assert_eq!(entry2.confidence, 0.0);
    }

    #[test]
    fn test_agent_output_entry() {
        let entry = KnowledgeEntry::from_agent_output(
            "rust-patterns",
            "Discovered: async test pattern",
            "All integration tests use #[tokio::test]",
            EntryType::Observation,
        );

        assert_eq!(entry.source, KnowledgeSource::AgentDiscovered);
        assert_eq!(entry.confidence, 0.5);
        assert_eq!(entry.domain, "rust-patterns");
    }

    #[test]
    fn test_entry_type_display() {
        assert_eq!(
            EntryType::ArchitectureDecision.to_string(),
            "Architecture Decision"
        );
        assert_eq!(EntryType::Pattern.to_string(), "Pattern");
        assert_eq!(EntryType::Rule.to_string(), "Rule");
        assert_eq!(EntryType::Observation.to_string(), "Observation");
        assert_eq!(EntryType::Decision.to_string(), "Decision");
        assert_eq!(EntryType::History.to_string(), "History");
    }

    #[test]
    fn test_knowledge_source_display() {
        assert_eq!(KnowledgeSource::Manual.to_string(), "Manual");
        assert_eq!(
            KnowledgeSource::AgentDiscovered.to_string(),
            "Agent Discovered"
        );
        assert_eq!(KnowledgeSource::Imported.to_string(), "Imported");
        assert_eq!(KnowledgeSource::Inferred.to_string(), "Inferred");
    }

    #[test]
    fn test_knowledge_query_matches_domain() {
        let entry = KnowledgeEntry::new("security", "Test", "Content", EntryType::Rule);

        let matching = KnowledgeQuery::new().with_domains(vec!["security".into()]);
        assert!(matching.matches(&entry));

        let not_matching = KnowledgeQuery::new().with_domains(vec!["architecture".into()]);
        assert!(!not_matching.matches(&entry));
    }

    #[test]
    fn test_knowledge_query_matches_tags() {
        let entry = KnowledgeEntry::new("test", "Test", "Content", EntryType::Rule)
            .with_tags(vec!["owasp".into(), "sql".into(), "injection".into()]);

        // Subset match — entry has all query tags
        let matching = KnowledgeQuery::new().with_tags(vec!["owasp".into(), "sql".into()]);
        assert!(matching.matches(&entry));

        // Entry missing a required tag
        let not_matching = KnowledgeQuery::new().with_tags(vec!["owasp".into(), "xss".into()]);
        assert!(!not_matching.matches(&entry));
    }

    #[test]
    fn test_knowledge_query_matches_entry_type() {
        let entry = KnowledgeEntry::new("test", "Test", "Content", EntryType::Rule);

        let matching =
            KnowledgeQuery::new().with_entry_types(vec![EntryType::Rule, EntryType::Pattern]);
        assert!(matching.matches(&entry));

        let not_matching =
            KnowledgeQuery::new().with_entry_types(vec![EntryType::ArchitectureDecision]);
        assert!(!not_matching.matches(&entry));
    }

    #[test]
    fn test_knowledge_query_matches_confidence() {
        let entry =
            KnowledgeEntry::new("test", "Test", "Content", EntryType::Rule).with_confidence(0.7);

        let matching = KnowledgeQuery::new().with_min_confidence(0.5);
        assert!(matching.matches(&entry));

        let not_matching = KnowledgeQuery::new().with_min_confidence(0.9);
        assert!(!not_matching.matches(&entry));
    }

    #[test]
    fn test_knowledge_query_matches_text_search() {
        let entry = KnowledgeEntry::new(
            "test",
            "SQL Injection Prevention",
            "Use parameterized queries to prevent SQL injection.",
            EntryType::Rule,
        );

        // Title match
        let matching = KnowledgeQuery::new().with_search_text("sql injection");
        assert!(matching.matches(&entry));

        // Content match
        let matching2 = KnowledgeQuery::new().with_search_text("parameterized");
        assert!(matching2.matches(&entry));

        // No match
        let not_matching = KnowledgeQuery::new().with_search_text("xss");
        assert!(!not_matching.matches(&entry));
    }

    #[test]
    fn test_knowledge_query_combined_filters() {
        let entry = KnowledgeEntry::new("security", "SQL Rule", "Use params", EntryType::Rule)
            .with_tags(vec!["owasp".into()])
            .with_confidence(0.9);

        let matching = KnowledgeQuery::new()
            .with_domains(vec!["security".into()])
            .with_tags(vec!["owasp".into()])
            .with_entry_types(vec![EntryType::Rule])
            .with_min_confidence(0.8);
        assert!(matching.matches(&entry));

        // Fails on confidence
        let not_matching = KnowledgeQuery::new()
            .with_domains(vec!["security".into()])
            .with_min_confidence(0.95);
        assert!(!not_matching.matches(&entry));
    }

    #[test]
    fn test_empty_query_matches_everything() {
        let entry = KnowledgeEntry::new("any", "Any", "Anything", EntryType::Observation);
        let query = KnowledgeQuery::new();
        assert!(query.matches(&entry));
    }

    #[test]
    fn test_domain_context_empty() {
        let ctx = DomainContext::empty();
        assert!(ctx.entries.is_empty());
        assert!(ctx.compiled_prompt.is_empty());
        assert_eq!(ctx.token_estimate, 0);
    }

    #[test]
    fn test_domain_context_compile() {
        let entries = vec![
            KnowledgeEntry::new(
                "arch",
                "ADR-001",
                "Use traits for I/O",
                EntryType::ArchitectureDecision,
            ),
            KnowledgeEntry::new(
                "rules",
                "No unwrap",
                "Never use unwrap() in lib code",
                EntryType::Rule,
            )
            .with_confidence(1.0),
        ];

        let context = DomainContext::compile(entries, None);

        // Rules should appear before ADRs
        assert!(context.compiled_prompt.contains("No unwrap"));
        assert!(context.compiled_prompt.contains("ADR-001"));
        assert_eq!(context.entries.len(), 2);
        assert!(context.token_estimate > 0);

        // Rules section should come first
        let rule_pos = context.compiled_prompt.find("Rule").unwrap();
        let adr_pos = context
            .compiled_prompt
            .find("Architecture Decision")
            .unwrap();
        assert!(rule_pos < adr_pos);
    }

    #[test]
    fn test_domain_context_compile_with_token_limit() {
        let mut entries = Vec::new();
        for i in 0..100 {
            entries.push(KnowledgeEntry::new(
                "test",
                format!("Entry {}", i),
                "A".repeat(100),
                EntryType::Observation,
            ));
        }

        // Limit to ~50 tokens = ~200 chars
        let context = DomainContext::compile(entries, Some(50));
        assert!(context.compiled_prompt.len() <= 200 + 100); // Some slack for headers
        assert!(context.entries.len() < 100);
    }

    #[test]
    fn test_knowledge_entry_serde_roundtrip() {
        let entry = KnowledgeEntry::new(
            "architecture",
            "ADR-001",
            "All I/O behind traits",
            EntryType::ArchitectureDecision,
        )
        .with_tags(vec!["traits".into(), "io".into()])
        .with_confidence(0.95);

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: KnowledgeEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.domain, entry.domain);
        assert_eq!(deserialized.title, entry.title);
        assert_eq!(deserialized.entry_type, entry.entry_type);
        assert_eq!(deserialized.confidence, entry.confidence);
        assert_eq!(deserialized.tags, entry.tags);
    }

    #[test]
    fn test_knowledge_query_serde_roundtrip() {
        let query = KnowledgeQuery::new()
            .with_domains(vec!["security".into()])
            .with_tags(vec!["owasp".into()])
            .with_min_confidence(0.7)
            .with_limit(10);

        let json = serde_json::to_string(&query).unwrap();
        let deserialized: KnowledgeQuery = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.domains, query.domains);
        assert_eq!(deserialized.tags, query.tags);
        assert_eq!(deserialized.min_confidence, query.min_confidence);
        assert_eq!(deserialized.limit, query.limit);
    }

    #[test]
    fn test_knowledge_error_display() {
        let err = KnowledgeError::NotFound(Uuid::nil());
        assert!(err.to_string().contains("not found"));

        let err2 = KnowledgeError::StorageError("disk full".into());
        assert!(err2.to_string().contains("disk full"));
    }
}
