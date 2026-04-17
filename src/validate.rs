/// Liveness checks: reachability and deadlock detection.
pub mod liveness;
/// Structural well-formedness checks.
pub mod structural;

use serde::{Deserialize, Serialize};

use crate::error::{Result, ScxmlError};
use crate::model::{Statechart, TransitionType};

/// A persistable validation report for audit and governance evidence.
///
/// Contains the validation outcome, all errors found, summary statistics,
/// and metadata for tamper-evident record-keeping. Callers can serialize
/// this to JSON and store it alongside the workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ValidationReport {
    /// Whether the chart passed all validation checks.
    pub valid: bool,
    /// All validation errors found (empty if valid).
    pub errors: Vec<String>,
    /// Number of states in the chart.
    pub state_count: usize,
    /// Number of transitions in the chart.
    pub transition_count: usize,
    /// Chart name, if present.
    pub chart_name: Option<String>,
    /// Chart initial state.
    pub chart_initial: String,
    /// Version of the scxml crate that performed the validation.
    pub crate_version: String,
    /// SHA-256 hex digest of the input that was validated, if provided.
    /// Callers should hash the original SCXML/JSON input before parsing
    /// and pass it to [`validate_report_with_hash`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_sha256: Option<String>,
}

/// Validate a chart and produce a [`ValidationReport`].
///
/// For tamper-evident reports, use [`validate_report_with_hash`] and provide
/// the SHA-256 hex digest of the original input.
pub fn validate_report(chart: &Statechart) -> ValidationReport {
    validate_report_with_hash(chart, None)
}

/// Validate a chart and produce a [`ValidationReport`] with an input hash.
///
/// The `input_sha256` should be the hex-encoded SHA-256 digest of the original
/// SCXML or JSON input, computed by the caller before parsing. This binds the
/// report to a specific input for audit purposes.
pub fn validate_report_with_hash(
    chart: &Statechart,
    input_sha256: Option<String>,
) -> ValidationReport {
    let errors = validate_all(chart);
    let stats = crate::stats::stats(chart);
    ValidationReport {
        valid: errors.is_empty(),
        errors: errors.iter().map(|e| e.to_string()).collect(),
        state_count: stats.total_states,
        transition_count: stats.total_transitions,
        chart_name: chart.name.as_ref().map(|n| n.to_string()),
        chart_initial: chart.initial.to_string(),
        crate_version: env!("CARGO_PKG_VERSION").to_string(),
        input_sha256,
    }
}

/// Run all validation passes (structural + liveness). Fails on the first error.
///
/// For governance workflows that need all errors at once, use [`validate_all`].
pub fn validate(chart: &Statechart) -> Result<()> {
    structural::validate_structure(chart)?;
    let index = crate::index::StateIndex::new(chart);
    liveness::validate_liveness_with_index(chart, index.state_count(), &index)?;
    validate_semantics(chart)?;
    Ok(())
}

/// Run all validation passes and collect every error found.
///
/// Returns an empty `Vec` if the chart is valid. Unlike [`validate`], this
/// does not short-circuit on the first error; it reports all structural,
/// liveness, and semantic issues in a single pass.
pub fn validate_all(chart: &Statechart) -> Vec<ScxmlError> {
    let mut errors = Vec::new();

    // Structural checks (collected, not short-circuiting).
    errors.extend(structural::collect_structural_errors(chart));

    // Liveness checks (only if structural passed enough to be meaningful).
    let index = crate::index::StateIndex::new(chart);
    if let Err(e) = liveness::validate_liveness_with_index(chart, index.state_count(), &index) {
        errors.push(e);
    }

    // Semantic checks.
    errors.extend(collect_semantic_errors(chart));

    errors
}

/// Semantic validation checks beyond structural well-formedness.
fn validate_semantics(chart: &Statechart) -> Result<()> {
    for state in chart.iter_all_states() {
        // Empty state IDs.
        if state.id.is_empty() {
            return Err(ScxmlError::Xml("state has empty id".into()));
        }

        for t in &state.transitions {
            // Delay format.
            if let Some(delay) = &t.delay {
                if !delay.starts_with('P') {
                    return Err(ScxmlError::Xml(format!(
                        "transition in state '{}' has invalid delay '{}' (must be ISO 8601 duration starting with 'P')",
                        state.id, delay
                    )));
                }
            }

            // Internal transitions must target descendants of the source.
            if t.transition_type == TransitionType::Internal {
                for target in &t.targets {
                    let is_descendant = state
                        .children
                        .iter()
                        .flat_map(|c| c.iter_all())
                        .any(|d| d.id == *target);
                    if !is_descendant {
                        return Err(ScxmlError::Xml(format!(
                            "internal transition in state '{}' targets '{}' which is not a descendant",
                            state.id, target
                        )));
                    }
                }
            }

            // Quorum = 0.
            if t.quorum == Some(0) {
                return Err(ScxmlError::Xml(format!(
                    "transition in state '{}' has quorum=0 (must be >= 1)",
                    state.id
                )));
            }
        }

        // Conflicting transitions: same event, both guardless, different targets.
        let guardless: Vec<_> = state
            .transitions
            .iter()
            .filter(|t| t.guard.is_none())
            .collect();
        for i in 0..guardless.len() {
            for j in (i + 1)..guardless.len() {
                if guardless[i].event == guardless[j].event {
                    let event_str = guardless[i].event.as_deref().unwrap_or("<eventless>");
                    return Err(ScxmlError::Xml(format!(
                        "state '{}' has conflicting guardless transitions on event '{}'",
                        state.id, event_str
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Collect semantic errors without short-circuiting.
fn collect_semantic_errors(chart: &Statechart) -> Vec<ScxmlError> {
    let mut errors = Vec::new();

    for state in chart.iter_all_states() {
        if state.id.is_empty() {
            errors.push(ScxmlError::Xml("state has empty id".into()));
        }

        for t in &state.transitions {
            if let Some(delay) = &t.delay {
                if !delay.starts_with('P') {
                    errors.push(ScxmlError::Xml(format!(
                        "transition in state '{}' has invalid delay '{}' (must be ISO 8601 duration starting with 'P')",
                        state.id, delay
                    )));
                }
            }

            if t.transition_type == TransitionType::Internal {
                for target in &t.targets {
                    let is_descendant = state
                        .children
                        .iter()
                        .flat_map(|c| c.iter_all())
                        .any(|d| d.id == *target);
                    if !is_descendant {
                        errors.push(ScxmlError::Xml(format!(
                            "internal transition in state '{}' targets '{}' which is not a descendant",
                            state.id, target
                        )));
                    }
                }
            }

            if t.quorum == Some(0) {
                errors.push(ScxmlError::Xml(format!(
                    "transition in state '{}' has quorum=0 (must be >= 1)",
                    state.id
                )));
            }
        }

        let guardless: Vec<_> = state
            .transitions
            .iter()
            .filter(|t| t.guard.is_none())
            .collect();
        for i in 0..guardless.len() {
            for j in (i + 1)..guardless.len() {
                if guardless[i].event == guardless[j].event {
                    let event_str = guardless[i].event.as_deref().unwrap_or("<eventless>");
                    errors.push(ScxmlError::Xml(format!(
                        "state '{}' has conflicting guardless transitions on event '{}'",
                        state.id, event_str
                    )));
                }
            }
        }
    }

    errors
}
