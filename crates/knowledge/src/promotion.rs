//! # Knowledge Promotion Evaluation
//!
//! Multi-dimensional scoring for knowledge promotion decisions.
//!
//! This module replaces binary promotion (verified = confidence up) with
//! graded evaluation across four dimensions:
//!
//! - **Correctness**: Did it solve the stated problem?
//! - **Generalizability**: Does this apply beyond this specific case?
//! - **Completeness**: Edge cases handled, not a stub?
//! - **Independence**: Worked across N distinct contexts?
//!
//! ## Key Insight: Entropy Collapse Prevention
//!
//! The `independence` score prevents entropy collapse — a `KnowledgeEntry`
//! cannot cross a high-confidence threshold unless it has been verified
//! in **multiple distinct contexts**, not just repeatedly in the same one.
//!
//! ## Scoring Rubrics
//!
//! | Dimension | 0.0 | 0.5 | 1.0 |
//! |-----------|-----|-----|-----|
//! | Correctness | Wrong result | Partial/edge case failures | Fully correct |
//! | Generalizability | Only works for exact case | Works for similar cases | Applies broadly |
//! | Completeness | Stub/placeholder | Happy path only | All edge cases |
//! | Independence | Single context | 2-3 contexts | 4+ distinct contexts |
//!
//! ## References
//!
//! - Anthropic design quality criteria research
//! - Kimi entropy-collapse mitigation patterns
//! - Issue #6: PromotionEvaluation for graded knowledge promotion

use serde::{Deserialize, Serialize};

// ============================================================================
// PromotionEvaluation — 4-Dimensional Scoring
// ============================================================================

/// Multi-dimensional evaluation for knowledge promotion decisions.
///
/// Each dimension is scored from 0.0 to 1.0. The weighted combination
/// determines whether a `KnowledgeEntry` should be promoted (confidence
/// increased) based on task execution results.
///
/// ## Weights
///
/// Generalizability is weighted highest (0.40) because it's what makes
/// knowledge compound — specific fixes have limited value, but patterns
/// that apply broadly create exponential returns.
///
/// ```text
/// correctness:      0.25  (did it work?)
/// generalizability: 0.40  (will it work elsewhere?)
/// completeness:     0.20  (did it handle edge cases?)
/// independence:     0.15  (has it been verified in multiple contexts?)
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PromotionEvaluation {
    /// Did it solve the stated problem? (0.0–1.0)
    ///
    /// - 0.0: Completely wrong result
    /// - 0.5: Partial solution, some edge cases fail
    /// - 1.0: Fully correct solution
    pub correctness: f32,

    /// Does this apply beyond this specific case? (0.0–1.0)
    ///
    /// - 0.0: Only works for the exact case at hand
    /// - 0.5: Works for similar cases with minor adaptation
    /// - 1.0: Applies broadly to the entire problem class
    ///
    /// This is the highest-weighted dimension because generalizable
    /// knowledge compounds — it improves agent performance on future
    /// tasks beyond just this one.
    pub generalizability: f32,

    /// Were edge cases handled? (0.0–1.0)
    ///
    /// - 0.0: Stub or placeholder only
    /// - 0.5: Happy path works, edge cases ignored
    /// - 1.0: All edge cases handled appropriately
    pub completeness: f32,

    /// Has this been verified in multiple distinct contexts? (0.0–1.0)
    ///
    /// This is the entropy-collapse prevention mechanism. A high
    /// independence score means the knowledge has been validated
    /// across different situations, not just repeated in the same one.
    ///
    /// - 0.0: Single context only
    /// - 0.5: 2-3 distinct contexts
    /// - 1.0: 4+ distinct contexts
    ///
    /// The independence score increases over time as the knowledge
    /// is successfully applied in new contexts.
    pub independence: f32,
}

impl PromotionEvaluation {
    /// Create a new evaluation with all scores at zero.
    pub fn new() -> Self {
        Self { correctness: 0.0, generalizability: 0.0, completeness: 0.0, independence: 0.0 }
    }

    /// Create an evaluation with specific scores.
    pub fn with_scores(
        correctness: f32,
        generalizability: f32,
        completeness: f32,
        independence: f32,
    ) -> Self {
        Self {
            correctness: correctness.clamp(0.0, 1.0),
            generalizability: generalizability.clamp(0.0, 1.0),
            completeness: completeness.clamp(0.0, 1.0),
            independence: independence.clamp(0.0, 1.0),
        }
    }

    /// Set correctness score (builder pattern).
    pub fn correctness(mut self, score: f32) -> Self {
        self.correctness = score.clamp(0.0, 1.0);
        self
    }

    /// Set generalizability score (builder pattern).
    pub fn generalizability(mut self, score: f32) -> Self {
        self.generalizability = score.clamp(0.0, 1.0);
        self
    }

    /// Set completeness score (builder pattern).
    pub fn completeness(mut self, score: f32) -> Self {
        self.completeness = score.clamp(0.0, 1.0);
        self
    }

    /// Set independence score (builder pattern).
    pub fn independence(mut self, score: f32) -> Self {
        self.independence = score.clamp(0.0, 1.0);
        self
    }

    /// Calculate the weighted score across all dimensions.
    ///
    /// Weights:
    /// - Correctness: 0.25
    /// - Generalizability: 0.40 (highest — generalizable knowledge compounds)
    /// - Completeness: 0.20
    /// - Independence: 0.15
    pub fn weighted_score(&self) -> f32 {
        self.correctness * 0.25
            + self.generalizability * 0.40
            + self.completeness * 0.20
            + self.independence * 0.15
    }

    /// Determine if this evaluation warrants knowledge promotion.
    ///
    /// Promotion requires BOTH:
    /// 1. Weighted score above threshold (default: 0.72)
    /// 2. Independence score above minimum (default: 0.5)
    ///
    /// The independence requirement prevents entropy collapse —
    /// knowledge cannot be promoted based on repetition in a single
    /// context, no matter how correct it is.
    pub fn should_promote(&self) -> bool {
        self.should_promote_with_thresholds(0.72, 0.5)
    }

    /// Determine promotion with custom thresholds.
    ///
    /// # Arguments
    ///
    /// * `score_threshold` - Minimum weighted score for promotion
    /// * `independence_threshold` - Minimum independence score for promotion
    pub fn should_promote_with_thresholds(
        &self,
        score_threshold: f32,
        independence_threshold: f32,
    ) -> bool {
        self.weighted_score() > score_threshold && self.independence > independence_threshold
    }

    /// Calculate the confidence delta to apply if promoted.
    ///
    /// Higher weighted scores result in larger confidence increases.
    /// The delta is scaled to avoid rapid confidence inflation.
    ///
    /// Returns a value between 0.0 and 0.15.
    pub fn confidence_delta(&self) -> f32 {
        if !self.should_promote() {
            return 0.0;
        }
        // Scale weighted score to a reasonable delta
        // Max delta is 0.15 for perfect scores
        (self.weighted_score() - 0.72) * 0.5
    }
}

impl Default for PromotionEvaluation {
    fn default() -> Self {
        Self::new()
    }
}

impl PromotionEvaluation {
    /// Create an evaluation from a simple success/failure result.
    ///
    /// This is a convenience method for quick evaluation without full
    /// contract-based scoring. It provides reasonable defaults:
    ///
    /// - Success: correctness=0.8, generalizability=0.5, completeness=0.6
    /// - Failure: correctness=0.2, generalizability=0.3, completeness=0.3
    ///
    /// Independence starts at 0.25 (single context).
    ///
    /// For more precise evaluation, use `EvaluationBuilder` with contract results.
    pub fn from_success_result(success: bool, _output: &str) -> Self {
        if success {
            Self {
                correctness: 0.8,
                generalizability: 0.5,
                completeness: 0.6,
                independence: 0.25, // Single context
            }
        } else {
            Self { correctness: 0.2, generalizability: 0.3, completeness: 0.3, independence: 0.25 }
        }
    }
}

// ============================================================================
// Independence Tracking
// ============================================================================

/// Tracks cross-context verification for a knowledge entry.
///
/// Used to calculate the `independence` score over time. Each time
/// knowledge is successfully applied in a new context, the context
/// is recorded here.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndependenceTracker {
    /// Unique context identifiers where this knowledge was verified.
    ///
    /// A "context" is defined by:
    /// - Different task type (e.g., "refactor" vs "implement")
    /// - Different domain (e.g., "auth" vs "database")
    /// - Different codebase region
    ///
    /// Repeated verifications in the same context don't increase
    /// independence — only genuinely distinct contexts count.
    pub verified_contexts: Vec<ContextRecord>,

    /// Total verification count (including same-context repeats).
    ///
    /// This is informational only — the independence score is based
    /// on unique contexts, not total verifications.
    pub total_verifications: u32,
}

/// A record of a verification in a specific context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRecord {
    /// Unique identifier for this context.
    pub context_id: String,

    /// Human-readable description of the context.
    pub description: String,

    /// When the verification occurred.
    pub verified_at: chrono::DateTime<chrono::Utc>,

    /// The task ID that triggered this verification.
    pub task_id: uuid::Uuid,
}

impl IndependenceTracker {
    /// Create a new, empty independence tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a verification in a context.
    ///
    /// If this is a new context (by `context_id`), it increases the
    /// distinct context count. Either way, it increments total verifications.
    pub fn record_verification(&mut self, record: ContextRecord) {
        self.total_verifications += 1;

        // Only add if this is a new context
        if !self.verified_contexts.iter().any(|c| c.context_id == record.context_id) {
            self.verified_contexts.push(record);
        }
    }

    /// Get the number of distinct contexts verified.
    pub fn distinct_context_count(&self) -> usize {
        self.verified_contexts.len()
    }

    /// Calculate the independence score based on verified contexts.
    ///
    /// - 0 contexts: 0.0
    /// - 1 context: 0.25
    /// - 2-3 contexts: 0.5
    /// - 4+ contexts: 1.0
    pub fn independence_score(&self) -> f32 {
        match self.distinct_context_count() {
            0 => 0.0,
            1 => 0.25,
            2 | 3 => 0.5,
            _ => 1.0,
        }
    }
}

// ============================================================================
// Evaluation Builder
// ============================================================================

/// Builder for constructing a `PromotionEvaluation` from contract results.
pub struct EvaluationBuilder {
    correctness: Option<f32>,
    generalizability: Option<f32>,
    completeness: Option<f32>,
    tracker: Option<IndependenceTracker>,
}

impl EvaluationBuilder {
    pub fn new() -> Self {
        Self { correctness: None, generalizability: None, completeness: None, tracker: None }
    }

    /// Set correctness from contract fulfillment score.
    pub fn from_contract_fulfillment(mut self, score: f32) -> Self {
        self.correctness = Some(score);
        self
    }

    /// Set generalizability based on how domain-specific the solution is.
    pub fn with_generalizability(mut self, score: f32) -> Self {
        self.generalizability = Some(score);
        self
    }

    /// Set completeness based on edge case coverage.
    pub fn with_completeness(mut self, score: f32) -> Self {
        self.completeness = Some(score);
        self
    }

    /// Set independence from a tracker.
    pub fn with_independence_tracker(mut self, tracker: &IndependenceTracker) -> Self {
        self.tracker = Some(tracker.clone());
        self
    }

    /// Build the evaluation.
    pub fn build(self) -> PromotionEvaluation {
        let independence = self.tracker.map(|t| t.independence_score()).unwrap_or(0.0);

        PromotionEvaluation::with_scores(
            self.correctness.unwrap_or(0.0),
            self.generalizability.unwrap_or(0.0),
            self.completeness.unwrap_or(0.0),
            independence,
        )
    }
}

impl Default for EvaluationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_score_calculation() {
        let eval = PromotionEvaluation::with_scores(1.0, 1.0, 1.0, 1.0);
        assert!((eval.weighted_score() - 1.0).abs() < 0.001);

        let eval_zero = PromotionEvaluation::new();
        assert_eq!(eval_zero.weighted_score(), 0.0);

        // Verify weights: 0.25 + 0.40 + 0.20 + 0.15 = 1.0
        let eval_half = PromotionEvaluation::with_scores(0.5, 0.5, 0.5, 0.5);
        assert!((eval_half.weighted_score() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_should_promote_requires_both_thresholds() {
        // High score but low independence — should NOT promote
        let high_score_low_independence = PromotionEvaluation::with_scores(1.0, 1.0, 1.0, 0.3);
        assert!(!high_score_low_independence.should_promote());

        // Low score but high independence — should NOT promote
        let low_score_high_independence = PromotionEvaluation::with_scores(0.3, 0.3, 0.3, 1.0);
        assert!(!low_score_high_independence.should_promote());

        // High score AND high independence — SHOULD promote
        let promote_worthy = PromotionEvaluation::with_scores(0.9, 0.9, 0.9, 0.8);
        assert!(promote_worthy.should_promote());
    }

    #[test]
    fn test_score_clamping() {
        let eval = PromotionEvaluation::with_scores(1.5, -0.5, 2.0, -1.0);
        assert_eq!(eval.correctness, 1.0);
        assert_eq!(eval.generalizability, 0.0);
        assert_eq!(eval.completeness, 1.0);
        assert_eq!(eval.independence, 0.0);
    }

    #[test]
    fn test_builder_pattern() {
        let eval = PromotionEvaluation::new()
            .correctness(0.8)
            .generalizability(0.9)
            .completeness(0.7)
            .independence(0.6);

        assert_eq!(eval.correctness, 0.8);
        assert_eq!(eval.generalizability, 0.9);
        assert_eq!(eval.completeness, 0.7);
        assert_eq!(eval.independence, 0.6);
    }

    #[test]
    fn test_independence_tracker() {
        let mut tracker = IndependenceTracker::new();

        // First context
        tracker.record_verification(ContextRecord {
            context_id: "auth-module".into(),
            description: "Applied in auth module refactor".into(),
            verified_at: chrono::Utc::now(),
            task_id: uuid::Uuid::new_v4(),
        });
        assert_eq!(tracker.distinct_context_count(), 1);
        assert_eq!(tracker.independence_score(), 0.25);

        // Same context again — doesn't increase distinct count
        tracker.record_verification(ContextRecord {
            context_id: "auth-module".into(),
            description: "Applied again in auth module".into(),
            verified_at: chrono::Utc::now(),
            task_id: uuid::Uuid::new_v4(),
        });
        assert_eq!(tracker.distinct_context_count(), 1);
        assert_eq!(tracker.total_verifications, 2);
        assert_eq!(tracker.independence_score(), 0.25);

        // New context
        tracker.record_verification(ContextRecord {
            context_id: "database-module".into(),
            description: "Applied in database module".into(),
            verified_at: chrono::Utc::now(),
            task_id: uuid::Uuid::new_v4(),
        });
        assert_eq!(tracker.distinct_context_count(), 2);
        assert_eq!(tracker.independence_score(), 0.5);

        // Third context
        tracker.record_verification(ContextRecord {
            context_id: "api-module".into(),
            description: "Applied in API module".into(),
            verified_at: chrono::Utc::now(),
            task_id: uuid::Uuid::new_v4(),
        });
        assert_eq!(tracker.independence_score(), 0.5); // Still 0.5 for 2-3

        // Fourth context — crosses threshold
        tracker.record_verification(ContextRecord {
            context_id: "cli-module".into(),
            description: "Applied in CLI module".into(),
            verified_at: chrono::Utc::now(),
            task_id: uuid::Uuid::new_v4(),
        });
        assert_eq!(tracker.distinct_context_count(), 4);
        assert_eq!(tracker.independence_score(), 1.0);
    }

    #[test]
    fn test_evaluation_builder() {
        let mut tracker = IndependenceTracker::new();
        tracker.record_verification(ContextRecord {
            context_id: "ctx-1".into(),
            description: "Context 1".into(),
            verified_at: chrono::Utc::now(),
            task_id: uuid::Uuid::new_v4(),
        });
        tracker.record_verification(ContextRecord {
            context_id: "ctx-2".into(),
            description: "Context 2".into(),
            verified_at: chrono::Utc::now(),
            task_id: uuid::Uuid::new_v4(),
        });

        let eval = EvaluationBuilder::new()
            .from_contract_fulfillment(0.9)
            .with_generalizability(0.8)
            .with_completeness(0.7)
            .with_independence_tracker(&tracker)
            .build();

        assert_eq!(eval.correctness, 0.9);
        assert_eq!(eval.generalizability, 0.8);
        assert_eq!(eval.completeness, 0.7);
        assert_eq!(eval.independence, 0.5); // 2 contexts
    }

    #[test]
    fn test_confidence_delta() {
        // Below promotion threshold — no delta
        let low = PromotionEvaluation::with_scores(0.5, 0.5, 0.5, 0.5);
        assert_eq!(low.confidence_delta(), 0.0);

        // Above promotion threshold — positive delta
        let high = PromotionEvaluation::with_scores(1.0, 1.0, 1.0, 1.0);
        assert!(high.confidence_delta() > 0.0);
        assert!(high.confidence_delta() <= 0.15);
    }
}
