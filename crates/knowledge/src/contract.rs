//! # Execution Contracts
//!
//! Contracts define testable success criteria before agent execution begins.
//! This module implements the contract-based evaluation system inspired by
//! Anthropic's research on improving evaluator signal quality.
//!
//! ## Key Insight
//!
//! A `KnowledgeEntry` captured against a pre-committed contract is 10x higher
//! signal than one from freeform observation, because you know *exactly* what
//! the agent set out to do and whether it succeeded.
//!
//! ## Execution Flow
//!
//! ```text
//! pre_execution_queries
//!     → compile_context()
//!     → generate ExecutionContract     ← THIS MODULE
//!     → execute against contract
//!     → post_execution_captures scored against contract criteria
//!     → promote KnowledgeEntry only if contract was met
//! ```
//!
//! ## References
//!
//! - Anthropic sprint contract research findings
//! - Issue #3: ExecutionContract and graded knowledge promotion

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// ExecutionContract — Pre-committed success criteria
// ============================================================================

/// A contract defining testable success criteria for a task.
///
/// Contracts are generated BEFORE execution begins, forcing the agent to
/// commit to specific, verifiable outcomes. This dramatically improves
/// the quality of post-execution evaluation and knowledge promotion.
///
/// ## Contract Generation
///
/// Contracts should be generated from:
/// 1. The task description and user intent
/// 2. Domain knowledge (what patterns/rules apply)
/// 3. Historical success criteria for similar tasks
///
/// ## Example
///
/// ```rust
/// use theasus_knowledge::contract::{ExecutionContract, VerifiableCriterion, VerificationMethod};
/// use uuid::Uuid;
///
/// let task_id = Uuid::new_v4();
/// let contract = ExecutionContract::new(task_id)
///     .with_criterion(
///         VerifiableCriterion::new("Code compiles without errors")
///             .with_method(VerificationMethod::Build)
///             .blocking()
///     )
///     .with_criterion(
///         VerifiableCriterion::new("All tests pass")
///             .with_method(VerificationMethod::Test)
///             .blocking()
///     )
///     .with_criterion(
///         VerifiableCriterion::new("No new clippy warnings")
///             .with_method(VerificationMethod::Lint)
///     )
///     .with_scope_boundary("Do not modify unrelated modules")
///     .with_confidence_floor(0.7);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContract {
    /// Unique identifier linking this contract to a task execution.
    pub task_id: Uuid,

    /// Concrete, testable success criteria.
    ///
    /// Each criterion defines what success looks like for one aspect
    /// of the task. Blocking criteria must all pass for the contract
    /// to be considered fulfilled.
    pub success_criteria: Vec<VerifiableCriterion>,

    /// Explicit scope boundaries — what's out of scope.
    ///
    /// Boundaries help the evaluator distinguish between intentional
    /// limitations and failures. If something wasn't attempted because
    /// it was out of scope, that's not a failure.
    ///
    /// Examples:
    /// - "Do not modify the database schema"
    /// - "Performance optimization is out of scope"
    /// - "Only address the specific file mentioned"
    pub scope_boundaries: Vec<String>,

    /// Minimum confidence threshold for promoting knowledge.
    ///
    /// Knowledge entries captured from this task will only be promoted
    /// if their evaluation score exceeds this floor. Higher floors
    /// mean stricter quality requirements.
    ///
    /// Default: 0.7
    pub confidence_floor: f32,

    /// When this contract was created.
    pub created_at: DateTime<Utc>,
}

impl ExecutionContract {
    /// Create a new execution contract for a task.
    pub fn new(task_id: Uuid) -> Self {
        Self {
            task_id,
            success_criteria: Vec::new(),
            scope_boundaries: Vec::new(),
            confidence_floor: 0.7,
            created_at: Utc::now(),
        }
    }

    /// Add a success criterion (builder pattern).
    pub fn with_criterion(mut self, criterion: VerifiableCriterion) -> Self {
        self.success_criteria.push(criterion);
        self
    }

    /// Add multiple criteria at once (builder pattern).
    pub fn with_criteria(mut self, criteria: Vec<VerifiableCriterion>) -> Self {
        self.success_criteria.extend(criteria);
        self
    }

    /// Add a scope boundary (builder pattern).
    pub fn with_scope_boundary(mut self, boundary: impl Into<String>) -> Self {
        self.scope_boundaries.push(boundary.into());
        self
    }

    /// Set the confidence floor for knowledge promotion (builder pattern).
    pub fn with_confidence_floor(mut self, floor: f32) -> Self {
        self.confidence_floor = floor.clamp(0.0, 1.0);
        self
    }

    /// Count the number of blocking criteria.
    pub fn blocking_criteria_count(&self) -> usize {
        self.success_criteria
            .iter()
            .filter(|c| c.is_blocking)
            .count()
    }

    /// Check if this is a complex contract (many blocking criteria).
    ///
    /// Complex contracts warrant deeper evaluation. This threshold
    /// is used by `DomainContext::should_run_deep_evaluation()`.
    pub fn is_complex(&self) -> bool {
        self.blocking_criteria_count() > 3
    }

    /// Evaluate the contract against execution results.
    ///
    /// Returns a `ContractEvaluation` with detailed pass/fail status
    /// for each criterion and an overall fulfillment score.
    pub fn evaluate(&self, results: &[CriterionResult]) -> ContractEvaluation {
        let mut criterion_results = Vec::new();
        let mut blocking_passed = 0;
        let mut blocking_total = 0;
        let mut total_passed = 0;

        for criterion in &self.success_criteria {
            let result = results
                .iter()
                .find(|r| {
                    r.criterion_index < self.success_criteria.len()
                        && self.success_criteria[r.criterion_index].description
                            == criterion.description
                })
                .cloned()
                .unwrap_or_else(|| CriterionResult {
                    criterion_index: 0,
                    passed: false,
                    evidence: None,
                    notes: Some("No evaluation result provided".into()),
                });

            if criterion.is_blocking {
                blocking_total += 1;
                if result.passed {
                    blocking_passed += 1;
                }
            }

            if result.passed {
                total_passed += 1;
            }

            criterion_results.push(result);
        }

        let blocking_fulfilled = blocking_total == 0 || blocking_passed == blocking_total;
        let fulfillment_score = if self.success_criteria.is_empty() {
            1.0
        } else {
            total_passed as f32 / self.success_criteria.len() as f32
        };

        ContractEvaluation {
            contract_id: self.task_id,
            criterion_results,
            blocking_fulfilled,
            fulfillment_score,
            evaluated_at: Utc::now(),
        }
    }
}

// ============================================================================
// VerifiableCriterion — A single testable requirement
// ============================================================================

/// A single, testable success criterion within a contract.
///
/// Criteria should be:
/// - **Specific**: Clear enough to evaluate unambiguously
/// - **Verifiable**: Can be checked via automated or manual methods
/// - **Independent**: Evaluable without reference to other criteria
///
/// ## Blocking vs Non-Blocking
///
/// - **Blocking**: Must pass for the contract to be fulfilled. These are
///   the hard requirements (e.g., "code compiles", "tests pass").
/// - **Non-blocking**: Nice to have. Failing these doesn't fail the contract,
///   but affects the overall quality score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiableCriterion {
    /// Human-readable description of what must be true.
    ///
    /// Should be specific enough to evaluate unambiguously.
    /// Good: "All unit tests in the modified module pass"
    /// Bad: "Code quality is good"
    pub description: String,

    /// How this criterion should be verified.
    pub verification_method: VerificationMethod,

    /// Whether this criterion must pass for contract fulfillment.
    ///
    /// Blocking criteria are hard requirements. Non-blocking criteria
    /// affect quality scoring but don't fail the contract.
    pub is_blocking: bool,

    /// Optional expected value or pattern for automated verification.
    ///
    /// For `VerificationMethod::Assertion`, this is the expected value.
    /// For `VerificationMethod::Test`, this might be a test name pattern.
    pub expected: Option<String>,
}

impl VerifiableCriterion {
    /// Create a new criterion with the given description.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            verification_method: VerificationMethod::Manual,
            is_blocking: false,
            expected: None,
        }
    }

    /// Set the verification method (builder pattern).
    pub fn with_method(mut self, method: VerificationMethod) -> Self {
        self.verification_method = method;
        self
    }

    /// Mark this criterion as blocking (builder pattern).
    pub fn blocking(mut self) -> Self {
        self.is_blocking = true;
        self
    }

    /// Set the expected value for automated verification (builder pattern).
    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self
    }
}

// ============================================================================
// VerificationMethod — How to verify a criterion
// ============================================================================

/// How a criterion should be verified.
///
/// The verification method guides both automated evaluation and human
/// review. Each method has different tooling and confidence implications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationMethod {
    /// Run the test suite and check for passing tests.
    ///
    /// Highest confidence automated verification. If tests pass,
    /// the criterion is fulfilled with high certainty.
    Test,

    /// Run linters (clippy, eslint, etc.) and check for issues.
    ///
    /// Good for code quality criteria. Can be fully automated.
    Lint,

    /// Compile/build the code and check for success.
    ///
    /// Basic sanity check. Very high confidence — if it compiles,
    /// the criterion is met.
    Build,

    /// Requires human review to verify.
    ///
    /// Use for subjective criteria like "code is readable" or
    /// "approach is appropriate for the use case".
    Manual,

    /// Check a specific assertion or condition programmatically.
    ///
    /// For criteria like "file exists" or "output matches pattern".
    /// The `expected` field on the criterion specifies what to check.
    Assertion,

    /// Verify by checking command output or exit code.
    ///
    /// For criteria that depend on running a specific command
    /// and checking its result.
    Command,
}

impl std::fmt::Display for VerificationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Test => write!(f, "Test"),
            Self::Lint => write!(f, "Lint"),
            Self::Build => write!(f, "Build"),
            Self::Manual => write!(f, "Manual"),
            Self::Assertion => write!(f, "Assertion"),
            Self::Command => write!(f, "Command"),
        }
    }
}

// ============================================================================
// Contract Evaluation Results
// ============================================================================

/// The result of evaluating a single criterion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionResult {
    /// Index of the criterion in the contract's `success_criteria` vec.
    pub criterion_index: usize,

    /// Whether the criterion was satisfied.
    pub passed: bool,

    /// Evidence supporting the pass/fail determination.
    ///
    /// For automated methods, this might be command output.
    /// For manual methods, this is the reviewer's notes.
    pub evidence: Option<String>,

    /// Additional notes about the evaluation.
    pub notes: Option<String>,
}

impl CriterionResult {
    /// Create a passing result.
    pub fn pass(index: usize) -> Self {
        Self {
            criterion_index: index,
            passed: true,
            evidence: None,
            notes: None,
        }
    }

    /// Create a failing result.
    pub fn fail(index: usize) -> Self {
        Self {
            criterion_index: index,
            passed: false,
            evidence: None,
            notes: None,
        }
    }

    /// Add evidence to this result (builder pattern).
    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence = Some(evidence.into());
        self
    }

    /// Add notes to this result (builder pattern).
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// The overall evaluation of a contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEvaluation {
    /// The contract that was evaluated.
    pub contract_id: Uuid,

    /// Results for each criterion.
    pub criterion_results: Vec<CriterionResult>,

    /// Whether all blocking criteria passed.
    pub blocking_fulfilled: bool,

    /// Overall fulfillment score (0.0–1.0).
    ///
    /// Calculated as: passing_criteria / total_criteria
    pub fulfillment_score: f32,

    /// When this evaluation was performed.
    pub evaluated_at: DateTime<Utc>,
}

impl ContractEvaluation {
    /// Check if this evaluation meets the contract's confidence floor.
    pub fn meets_confidence_floor(&self, floor: f32) -> bool {
        self.blocking_fulfilled && self.fulfillment_score >= floor
    }

    /// Get the number of passing criteria.
    pub fn passing_count(&self) -> usize {
        self.criterion_results.iter().filter(|r| r.passed).count()
    }

    /// Get the number of failing criteria.
    pub fn failing_count(&self) -> usize {
        self.criterion_results.iter().filter(|r| !r.passed).count()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_creation() {
        let task_id = Uuid::new_v4();
        let contract = ExecutionContract::new(task_id)
            .with_criterion(
                VerifiableCriterion::new("Code compiles")
                    .with_method(VerificationMethod::Build)
                    .blocking(),
            )
            .with_criterion(
                VerifiableCriterion::new("Tests pass")
                    .with_method(VerificationMethod::Test)
                    .blocking(),
            )
            .with_scope_boundary("Do not modify database")
            .with_confidence_floor(0.8);

        assert_eq!(contract.task_id, task_id);
        assert_eq!(contract.success_criteria.len(), 2);
        assert_eq!(contract.scope_boundaries.len(), 1);
        assert_eq!(contract.confidence_floor, 0.8);
        assert_eq!(contract.blocking_criteria_count(), 2);
    }

    #[test]
    fn test_criterion_builder() {
        let criterion = VerifiableCriterion::new("All tests pass")
            .with_method(VerificationMethod::Test)
            .blocking()
            .with_expected("0 failures");

        assert_eq!(criterion.description, "All tests pass");
        assert_eq!(criterion.verification_method, VerificationMethod::Test);
        assert!(criterion.is_blocking);
        assert_eq!(criterion.expected, Some("0 failures".into()));
    }

    #[test]
    fn test_contract_evaluation_all_pass() {
        let contract = ExecutionContract::new(Uuid::new_v4())
            .with_criterion(VerifiableCriterion::new("C1").blocking())
            .with_criterion(VerifiableCriterion::new("C2").blocking())
            .with_criterion(VerifiableCriterion::new("C3"));

        let results = vec![
            CriterionResult::pass(0),
            CriterionResult::pass(1),
            CriterionResult::pass(2),
        ];

        let eval = contract.evaluate(&results);
        assert!(eval.blocking_fulfilled);
        assert_eq!(eval.fulfillment_score, 1.0);
        assert_eq!(eval.passing_count(), 3);
        assert_eq!(eval.failing_count(), 0);
    }

    #[test]
    fn test_contract_evaluation_blocking_fail() {
        let contract = ExecutionContract::new(Uuid::new_v4())
            .with_criterion(VerifiableCriterion::new("C1").blocking())
            .with_criterion(VerifiableCriterion::new("C2"))
            .with_confidence_floor(0.7);

        let results = vec![
            CriterionResult::fail(0).with_notes("Build failed"),
            CriterionResult::pass(1),
        ];

        let eval = contract.evaluate(&results);
        assert!(!eval.blocking_fulfilled);
        assert_eq!(eval.fulfillment_score, 0.5);
        assert!(!eval.meets_confidence_floor(0.7));
    }

    #[test]
    fn test_contract_evaluation_non_blocking_fail() {
        let contract = ExecutionContract::new(Uuid::new_v4())
            .with_criterion(VerifiableCriterion::new("C1").blocking())
            .with_criterion(VerifiableCriterion::new("C2")) // non-blocking
            .with_confidence_floor(0.7);

        let results = vec![CriterionResult::pass(0), CriterionResult::fail(1)];

        let eval = contract.evaluate(&results);
        assert!(eval.blocking_fulfilled); // Blocking passed
        assert_eq!(eval.fulfillment_score, 0.5);
        assert!(!eval.meets_confidence_floor(0.7)); // Below floor
    }

    #[test]
    fn test_is_complex() {
        let simple = ExecutionContract::new(Uuid::new_v4())
            .with_criterion(VerifiableCriterion::new("C1").blocking())
            .with_criterion(VerifiableCriterion::new("C2").blocking());

        assert!(!simple.is_complex());

        let complex = ExecutionContract::new(Uuid::new_v4())
            .with_criterion(VerifiableCriterion::new("C1").blocking())
            .with_criterion(VerifiableCriterion::new("C2").blocking())
            .with_criterion(VerifiableCriterion::new("C3").blocking())
            .with_criterion(VerifiableCriterion::new("C4").blocking());

        assert!(complex.is_complex());
    }

    #[test]
    fn test_verification_method_display() {
        assert_eq!(VerificationMethod::Test.to_string(), "Test");
        assert_eq!(VerificationMethod::Build.to_string(), "Build");
        assert_eq!(VerificationMethod::Lint.to_string(), "Lint");
        assert_eq!(VerificationMethod::Manual.to_string(), "Manual");
        assert_eq!(VerificationMethod::Assertion.to_string(), "Assertion");
        assert_eq!(VerificationMethod::Command.to_string(), "Command");
    }

    #[test]
    fn test_confidence_floor_clamping() {
        let contract = ExecutionContract::new(Uuid::new_v4()).with_confidence_floor(1.5);
        assert_eq!(contract.confidence_floor, 1.0);

        let contract2 = ExecutionContract::new(Uuid::new_v4()).with_confidence_floor(-0.5);
        assert_eq!(contract2.confidence_floor, 0.0);
    }
}
