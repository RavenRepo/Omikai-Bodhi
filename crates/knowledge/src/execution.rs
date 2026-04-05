//! # Pre/Post Execution Queries
//!
//! Integration hooks for the agent execution loop. This module provides:
//!
//! - **Pre-execution queries**: Compile relevant knowledge before execution
//! - **Post-execution captures**: Capture observations after execution
//!
//! ## Execution Flow
//!
//! ```text
//! 1. PreExecutionQuery { domain, task_description, max_tokens }
//!        ↓
//! 2. provider.pre_execution_query(query)
//!        ↓
//! 3. CompiledContext { injected_knowledge, domain_confidence, suggested_criteria }
//!        ↓
//! 4. Generate ExecutionContract from suggested_criteria
//!        ↓
//! 5. Execute task
//!        ↓
//! 6. PostExecutionCapture { contract, result, observations }
//!        ↓
//! 7. Evaluate against contract → PromotionEvaluation
//!        ↓
//! 8. Promote KnowledgeEntry if evaluation passes
//! ```
//!
//! ## References
//!
//! - plan.md Phase 10.3 Agent Features
//! - Issue #7: Pre/post execution queries for knowledge capture

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::contract::{ExecutionContract, VerifiableCriterion, VerificationMethod};
use crate::promotion::PromotionEvaluation;
use crate::{DomainContext, EntryType, KnowledgeEntry, KnowledgeSource};

// ============================================================================
// Pre-Execution Query
// ============================================================================

/// A request to compile relevant knowledge before task execution.
///
/// The agent (or agent framework) constructs this query based on:
/// - The user's task description
/// - The declared domains of interest for the agent type
/// - Token budget constraints from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreExecutionQuery {
    /// The knowledge domain(s) to query.
    ///
    /// Examples: "rust-patterns", "security", "architecture"
    pub domains: Vec<String>,

    /// Description of the task to be executed.
    ///
    /// Used to:
    /// 1. Search for relevant knowledge entries
    /// 2. Suggest appropriate contract criteria
    pub task_description: String,

    /// Maximum tokens to allocate for injected knowledge.
    ///
    /// The compiled context will respect this budget, prioritizing
    /// high-confidence rules and ADRs over lower-priority entries.
    pub max_context_tokens: usize,

    /// Optional task ID if already assigned.
    pub task_id: Option<Uuid>,
}

impl PreExecutionQuery {
    /// Create a new pre-execution query.
    pub fn new(task_description: impl Into<String>) -> Self {
        Self {
            domains: Vec::new(),
            task_description: task_description.into(),
            max_context_tokens: 2000, // Reasonable default
            task_id: None,
        }
    }

    /// Add a domain to query (builder pattern).
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domains.push(domain.into());
        self
    }

    /// Add multiple domains (builder pattern).
    pub fn with_domains(mut self, domains: Vec<String>) -> Self {
        self.domains.extend(domains);
        self
    }

    /// Set the token budget (builder pattern).
    pub fn with_max_tokens(mut self, tokens: usize) -> Self {
        self.max_context_tokens = tokens;
        self
    }

    /// Set the task ID (builder pattern).
    pub fn with_task_id(mut self, id: Uuid) -> Self {
        self.task_id = Some(id);
        self
    }
}

// ============================================================================
// Compiled Context (Pre-Execution Response)
// ============================================================================

/// The result of a pre-execution query.
///
/// Contains everything the agent needs to begin informed execution:
/// - Compiled knowledge for injection into the system prompt
/// - Domain confidence assessment
/// - Suggested contract criteria based on domain patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledExecutionContext {
    /// The compiled domain knowledge ready for prompt injection.
    pub domain_context: DomainContext,

    /// Average confidence score for the queried domains.
    ///
    /// Low confidence (< 0.7) indicates the agent should be more
    /// careful and explicit about verifying its work.
    pub domain_confidence: f32,

    /// Suggested criteria for the execution contract.
    ///
    /// These are inferred from domain patterns and common requirements.
    /// The agent/framework should review and potentially add task-specific
    /// criteria before finalizing the contract.
    pub suggested_criteria: Vec<VerifiableCriterion>,

    /// Whether deep evaluation is recommended for this task.
    ///
    /// True when either:
    /// - Domain confidence is low
    /// - Suggested criteria indicate a complex task
    pub recommend_deep_evaluation: bool,
}

impl CompiledExecutionContext {
    /// Create a compiled context from a domain context.
    pub fn from_domain_context(ctx: DomainContext) -> Self {
        let domain_confidence = ctx.average_confidence();
        let recommend_deep_evaluation = domain_confidence < 0.7;

        Self {
            domain_context: ctx,
            domain_confidence,
            suggested_criteria: Vec::new(),
            recommend_deep_evaluation,
        }
    }

    /// Add suggested contract criteria.
    pub fn with_suggested_criteria(mut self, criteria: Vec<VerifiableCriterion>) -> Self {
        self.suggested_criteria = criteria;
        // Update recommendation based on criteria complexity
        if self.suggested_criteria.iter().filter(|c| c.is_blocking).count() > 3 {
            self.recommend_deep_evaluation = true;
        }
        self
    }

    /// Build an execution contract from this context.
    ///
    /// Uses the suggested criteria and allows additional customization.
    pub fn build_contract(&self, task_id: Uuid) -> ExecutionContract {
        let mut contract =
            ExecutionContract::new(task_id).with_criteria(self.suggested_criteria.clone());

        // Set confidence floor based on domain maturity
        if self.domain_confidence >= 0.8 {
            contract = contract.with_confidence_floor(0.8);
        } else if self.domain_confidence >= 0.5 {
            contract = contract.with_confidence_floor(0.7);
        } else {
            contract = contract.with_confidence_floor(0.6);
        }

        contract
    }
}

// ============================================================================
// Post-Execution Capture
// ============================================================================

/// Captured data from a completed task execution.
///
/// This is submitted after execution to:
/// 1. Evaluate the contract fulfillment
/// 2. Capture observations as potential knowledge entries
/// 3. Update domain confidence based on results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostExecutionCapture {
    /// The task ID that was executed.
    pub task_id: Uuid,

    /// The contract this execution was evaluated against.
    pub contract: ExecutionContract,

    /// The execution result status.
    pub result: TaskResult,

    /// Observations captured during execution.
    ///
    /// These are candidates for promotion to `KnowledgeEntry` if
    /// the promotion evaluation passes.
    pub observations: Vec<Observation>,

    /// When the execution completed.
    pub completed_at: DateTime<Utc>,
}

impl PostExecutionCapture {
    /// Create a new post-execution capture.
    pub fn new(task_id: Uuid, contract: ExecutionContract, result: TaskResult) -> Self {
        Self { task_id, contract, result, observations: Vec::new(), completed_at: Utc::now() }
    }

    /// Add an observation (builder pattern).
    pub fn with_observation(mut self, observation: Observation) -> Self {
        self.observations.push(observation);
        self
    }

    /// Add multiple observations (builder pattern).
    pub fn with_observations(mut self, observations: Vec<Observation>) -> Self {
        self.observations.extend(observations);
        self
    }

    /// Convert observations to knowledge entries for potential promotion.
    ///
    /// The returned entries have:
    /// - Source set to `AgentDiscovered`
    /// - Confidence set based on the task result
    /// - Entry type based on the observation type
    pub fn to_knowledge_entries(&self) -> Vec<KnowledgeEntry> {
        let base_confidence = match &self.result {
            TaskResult::Success { .. } => 0.6,
            TaskResult::PartialSuccess { .. } => 0.4,
            TaskResult::Failure { .. } => 0.2,
        };

        self.observations
            .iter()
            .map(|obs| {
                KnowledgeEntry::new(
                    &obs.suggested_domain,
                    obs.title(),
                    &obs.content,
                    obs.to_entry_type(),
                )
                .with_confidence(base_confidence)
                .with_source(KnowledgeSource::AgentDiscovered)
            })
            .collect()
    }
}

// ============================================================================
// Task Result
// ============================================================================

/// The outcome of a task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
    /// Task completed successfully.
    Success {
        /// Summary of what was accomplished.
        summary: String,
        /// Any artifacts produced (file paths, URLs, etc.).
        artifacts: Vec<String>,
    },

    /// Task partially completed — some goals met, some not.
    PartialSuccess {
        /// What was accomplished.
        completed: String,
        /// What wasn't accomplished and why.
        incomplete: String,
    },

    /// Task failed to complete.
    Failure {
        /// What went wrong.
        error: String,
        /// Whether the failure was recoverable.
        recoverable: bool,
    },
}

impl TaskResult {
    /// Check if this is a successful result.
    pub fn is_success(&self) -> bool {
        matches!(self, TaskResult::Success { .. })
    }

    /// Check if this is at least a partial success.
    pub fn is_at_least_partial(&self) -> bool {
        !matches!(self, TaskResult::Failure { .. })
    }
}

// ============================================================================
// Observations
// ============================================================================

/// An observation captured during task execution.
///
/// Observations are candidates for promotion to `KnowledgeEntry`.
/// They represent things the agent learned or discovered while
/// working on the task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// The content of the observation in Markdown.
    pub content: String,

    /// Classification of the observation.
    pub observation_type: ObservationType,

    /// Suggested domain for this observation.
    ///
    /// The agent infers this from the task context, but it can be
    /// overridden during promotion.
    pub suggested_domain: String,

    /// Tags to apply if promoted to a knowledge entry.
    pub tags: Vec<String>,

    /// Optional reference to source (file path, URL, etc.).
    pub source_reference: Option<String>,
}

impl Observation {
    /// Create a new observation.
    pub fn new(content: impl Into<String>, observation_type: ObservationType) -> Self {
        Self {
            content: content.into(),
            observation_type,
            suggested_domain: String::new(),
            tags: Vec::new(),
            source_reference: None,
        }
    }

    /// Set the suggested domain (builder pattern).
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.suggested_domain = domain.into();
        self
    }

    /// Add tags (builder pattern).
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add a source reference (builder pattern).
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source_reference = Some(source.into());
        self
    }

    /// Generate a title for this observation when converted to KnowledgeEntry.
    pub fn title(&self) -> String {
        // Extract first line or first 60 chars
        let first_line = self.content.lines().next().unwrap_or(&self.content);
        if first_line.len() <= 60 {
            first_line.to_string()
        } else {
            format!("{}...", &first_line[..57])
        }
    }

    /// Convert observation type to entry type.
    pub fn to_entry_type(&self) -> EntryType {
        match self.observation_type {
            ObservationType::Pattern => EntryType::Pattern,
            ObservationType::Pitfall => EntryType::Rule, // Pitfalls become "don't do X" rules
            ObservationType::Dependency => EntryType::Observation,
            ObservationType::Optimization => EntryType::Pattern,
            ObservationType::Workaround => EntryType::Decision,
            ObservationType::Discovery => EntryType::Observation,
        }
    }
}

/// Classification of observations.
///
/// Each type maps to a different `EntryType` and has different
/// implications for how the knowledge is used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationType {
    /// A reusable approach or technique.
    ///
    /// Patterns have high generalizability potential and are
    /// weighted favorably in promotion evaluation.
    Pattern,

    /// Something to avoid — a mistake or anti-pattern.
    ///
    /// Pitfalls are converted to Rule entries (prohibitions)
    /// when promoted.
    Pitfall,

    /// An external requirement or dependency discovered.
    ///
    /// Dependencies are important for task planning but have
    /// lower generalizability (they're often project-specific).
    Dependency,

    /// A performance insight or optimization.
    ///
    /// Optimizations are treated as patterns — they may apply
    /// broadly or only to specific contexts.
    Optimization,

    /// A workaround for a known limitation.
    ///
    /// Workarounds become Decision entries — they document
    /// a choice made to address a constraint.
    Workaround,

    /// A general discovery about the codebase or project.
    ///
    /// The default type for observations that don't fit other
    /// categories. Low initial confidence.
    Discovery,
}

impl std::fmt::Display for ObservationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pattern => write!(f, "Pattern"),
            Self::Pitfall => write!(f, "Pitfall"),
            Self::Dependency => write!(f, "Dependency"),
            Self::Optimization => write!(f, "Optimization"),
            Self::Workaround => write!(f, "Workaround"),
            Self::Discovery => write!(f, "Discovery"),
        }
    }
}

// ============================================================================
// Knowledge Capture Pipeline
// ============================================================================

/// Result of processing a post-execution capture.
#[derive(Debug, Clone)]
pub struct CaptureProcessingResult {
    /// Entries that were promoted to the knowledge store.
    pub promoted_entries: Vec<Uuid>,

    /// Entries that were captured but not promoted (below threshold).
    pub pending_entries: Vec<KnowledgeEntry>,

    /// The promotion evaluation for this capture.
    pub evaluation: PromotionEvaluation,

    /// Whether the contract was fulfilled.
    pub contract_fulfilled: bool,
}

impl CaptureProcessingResult {
    /// Create a result indicating no promotion occurred.
    pub fn no_promotion(evaluation: PromotionEvaluation, contract_fulfilled: bool) -> Self {
        Self {
            promoted_entries: Vec::new(),
            pending_entries: Vec::new(),
            evaluation,
            contract_fulfilled,
        }
    }
}

// ============================================================================
// Suggested Criteria Generators
// ============================================================================

/// Generate suggested contract criteria for common task types.
pub mod criteria_suggestions {
    use super::*;

    /// Criteria for code modification tasks.
    pub fn code_modification() -> Vec<VerifiableCriterion> {
        vec![
            VerifiableCriterion::new("Code compiles without errors")
                .with_method(VerificationMethod::Build)
                .blocking(),
            VerifiableCriterion::new("All existing tests pass")
                .with_method(VerificationMethod::Test)
                .blocking(),
            VerifiableCriterion::new("No new clippy warnings introduced")
                .with_method(VerificationMethod::Lint),
            VerifiableCriterion::new("Code is formatted with rustfmt")
                .with_method(VerificationMethod::Lint),
        ]
    }

    /// Criteria for refactoring tasks.
    pub fn refactoring() -> Vec<VerifiableCriterion> {
        vec![
            VerifiableCriterion::new("Code compiles without errors")
                .with_method(VerificationMethod::Build)
                .blocking(),
            VerifiableCriterion::new("All existing tests pass")
                .with_method(VerificationMethod::Test)
                .blocking(),
            VerifiableCriterion::new("No behavior changes introduced")
                .with_method(VerificationMethod::Manual)
                .blocking(),
            VerifiableCriterion::new("Code complexity reduced or unchanged")
                .with_method(VerificationMethod::Manual),
        ]
    }

    /// Criteria for bug fix tasks.
    pub fn bug_fix() -> Vec<VerifiableCriterion> {
        vec![
            VerifiableCriterion::new("Bug is fixed (issue no longer reproduces)")
                .with_method(VerificationMethod::Test)
                .blocking(),
            VerifiableCriterion::new("Regression test added")
                .with_method(VerificationMethod::Assertion)
                .blocking(),
            VerifiableCriterion::new("All existing tests pass")
                .with_method(VerificationMethod::Test)
                .blocking(),
            VerifiableCriterion::new("No new issues introduced")
                .with_method(VerificationMethod::Test),
        ]
    }

    /// Criteria for documentation tasks.
    pub fn documentation() -> Vec<VerifiableCriterion> {
        vec![
            VerifiableCriterion::new("Documentation is accurate")
                .with_method(VerificationMethod::Manual)
                .blocking(),
            VerifiableCriterion::new("Code examples compile and run")
                .with_method(VerificationMethod::Build),
            VerifiableCriterion::new("No broken links").with_method(VerificationMethod::Command),
        ]
    }

    /// Criteria for new feature implementation.
    pub fn new_feature() -> Vec<VerifiableCriterion> {
        vec![
            VerifiableCriterion::new("Feature works as specified")
                .with_method(VerificationMethod::Test)
                .blocking(),
            VerifiableCriterion::new("Code compiles without errors")
                .with_method(VerificationMethod::Build)
                .blocking(),
            VerifiableCriterion::new("Unit tests added for new code")
                .with_method(VerificationMethod::Assertion)
                .blocking(),
            VerifiableCriterion::new("All existing tests pass")
                .with_method(VerificationMethod::Test)
                .blocking(),
            VerifiableCriterion::new("Documentation updated")
                .with_method(VerificationMethod::Manual),
        ]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_execution_query_builder() {
        let query = PreExecutionQuery::new("Refactor the authentication module")
            .with_domain("security")
            .with_domain("architecture")
            .with_max_tokens(3000)
            .with_task_id(Uuid::new_v4());

        assert_eq!(query.task_description, "Refactor the authentication module");
        assert_eq!(query.domains, vec!["security", "architecture"]);
        assert_eq!(query.max_context_tokens, 3000);
        assert!(query.task_id.is_some());
    }

    #[test]
    fn test_compiled_context_confidence_floor() {
        // High confidence domain
        let mut entries = Vec::new();
        for _ in 0..5 {
            entries.push(
                KnowledgeEntry::new("test", "title", "content", EntryType::Rule)
                    .with_confidence(0.9),
            );
        }
        let ctx = DomainContext::compile(entries, None);
        let compiled = CompiledExecutionContext::from_domain_context(ctx);
        let contract = compiled.build_contract(Uuid::new_v4());
        assert_eq!(contract.confidence_floor, 0.8);

        // Low confidence domain
        let mut low_entries = Vec::new();
        for _ in 0..5 {
            low_entries.push(
                KnowledgeEntry::new("test", "title", "content", EntryType::Observation)
                    .with_confidence(0.3),
            );
        }
        let low_ctx = DomainContext::compile(low_entries, None);
        let low_compiled = CompiledExecutionContext::from_domain_context(low_ctx);
        let low_contract = low_compiled.build_contract(Uuid::new_v4());
        assert_eq!(low_contract.confidence_floor, 0.6);
    }

    #[test]
    fn test_observation_to_knowledge_entry() {
        let obs = Observation::new(
            "Always use parameterized queries for database access",
            ObservationType::Pattern,
        )
        .with_domain("security")
        .with_tags(vec!["sql".into(), "injection".into()]);

        let entry_type = obs.to_entry_type();
        assert_eq!(entry_type, EntryType::Pattern);
        assert_eq!(obs.title(), "Always use parameterized queries for database access");
    }

    #[test]
    fn test_pitfall_becomes_rule() {
        let obs = Observation::new(
            "Never use string concatenation for SQL queries",
            ObservationType::Pitfall,
        );
        assert_eq!(obs.to_entry_type(), EntryType::Rule);
    }

    #[test]
    fn test_post_execution_capture_to_entries() {
        let contract = ExecutionContract::new(Uuid::new_v4());
        let capture = PostExecutionCapture::new(
            Uuid::new_v4(),
            contract,
            TaskResult::Success { summary: "Task completed".into(), artifacts: vec![] },
        )
        .with_observation(
            Observation::new("Pattern observed", ObservationType::Pattern)
                .with_domain("test-domain"),
        );

        let entries = capture.to_knowledge_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].confidence, 0.6); // Success = 0.6 base confidence
        assert_eq!(entries[0].source, KnowledgeSource::AgentDiscovered);
    }

    #[test]
    fn test_task_result_checks() {
        let success = TaskResult::Success { summary: "Done".into(), artifacts: vec![] };
        assert!(success.is_success());
        assert!(success.is_at_least_partial());

        let partial =
            TaskResult::PartialSuccess { completed: "Some".into(), incomplete: "Other".into() };
        assert!(!partial.is_success());
        assert!(partial.is_at_least_partial());

        let failure = TaskResult::Failure { error: "Failed".into(), recoverable: true };
        assert!(!failure.is_success());
        assert!(!failure.is_at_least_partial());
    }

    #[test]
    fn test_criteria_suggestions() {
        let code_mod = criteria_suggestions::code_modification();
        assert!(!code_mod.is_empty());
        assert!(code_mod.iter().any(|c| c.is_blocking));

        let bug_fix = criteria_suggestions::bug_fix();
        assert!(bug_fix.len() >= 3); // Should have multiple criteria
    }

    #[test]
    fn test_observation_title_truncation() {
        let short_obs = Observation::new("Short content", ObservationType::Discovery);
        assert_eq!(short_obs.title(), "Short content");

        let long_content = "A".repeat(100);
        let long_obs = Observation::new(&long_content, ObservationType::Discovery);
        assert!(long_obs.title().len() <= 60);
        assert!(long_obs.title().ends_with("..."));
    }
}
