use std::collections::HashSet;

use crate::error::{Result, ScxmlError};
use crate::model::{State, StateKind, Statechart};

/// Collect all structural errors without short-circuiting.
pub fn collect_structural_errors(chart: &Statechart) -> Vec<ScxmlError> {
    let mut errors = Vec::new();
    let all_ids: Vec<_> = chart.all_state_ids();

    // Duplicate IDs.
    let mut seen: HashSet<&str> = HashSet::with_capacity(all_ids.len());
    for id in &all_ids {
        if !seen.insert(id.as_str()) {
            errors.push(ScxmlError::DuplicateStateId(id.to_string()));
        }
    }

    // Initial state.
    if !seen.contains(chart.initial.as_str()) {
        errors.push(ScxmlError::InvalidInitial(chart.initial.to_string()));
    }

    // Per-state checks.
    let limit = crate::max_depth();
    for state in &chart.states {
        collect_state_errors(state, false, 0, limit, &mut errors);
    }

    // Transition targets.
    for state in chart.iter_all_states() {
        for t in &state.transitions {
            for target in &t.targets {
                if !seen.contains(target.as_str()) {
                    errors.push(ScxmlError::UnknownTarget {
                        src: state.id.to_string(),
                        target: target.to_string(),
                    });
                }
            }
        }
    }

    errors
}

fn collect_state_errors(
    state: &State,
    is_child: bool,
    depth: usize,
    limit: usize,
    errors: &mut Vec<ScxmlError>,
) {
    if depth > limit {
        errors.push(ScxmlError::DepthLimitExceeded {
            state: state.id.to_string(),
            limit,
        });
        return;
    }
    match state.kind {
        StateKind::Final if !state.transitions.is_empty() => {
            errors.push(ScxmlError::FinalHasTransitions(state.id.to_string()));
        }
        StateKind::Compound => {
            if let Some(ref init) = state.initial {
                if !state.children.iter().any(|c| c.id == *init) {
                    errors.push(ScxmlError::CompoundNoInitial(state.id.to_string()));
                }
            } else if state.children.is_empty() {
                errors.push(ScxmlError::CompoundNoInitial(state.id.to_string()));
            }
        }
        StateKind::Parallel if state.children.len() < 2 => {
            errors.push(ScxmlError::ParallelTooFewRegions(state.id.to_string()));
        }
        StateKind::History(_) if !is_child => {
            errors.push(ScxmlError::OrphanHistory(state.id.to_string()));
        }
        StateKind::Atomic if !state.children.is_empty() => {
            errors.push(ScxmlError::Xml(format!(
                "atomic state '{}' has children (should be compound or parallel)",
                state.id
            )));
        }
        _ => {}
    }
    for child in &state.children {
        collect_state_errors(child, true, depth + 1, limit, errors);
    }
}

/// Perform structural validation on a statechart.
///
/// Checks:
/// - All transition targets reference existing state ids
/// - No duplicate state ids
/// - Initial state exists
/// - Final states have no outgoing transitions
/// - Compound states have an initial child
/// - Parallel states have ≥2 child regions
/// - History states have a parent (are not top-level)
pub fn validate_structure(chart: &Statechart) -> Result<()> {
    let all_ids: Vec<_> = chart.all_state_ids();

    // Check for duplicates.
    let mut seen: HashSet<&str> = HashSet::with_capacity(all_ids.len());
    for id in &all_ids {
        if !seen.insert(id.as_str()) {
            return Err(ScxmlError::DuplicateStateId(id.to_string()));
        }
    }

    // Check initial state exists.
    if !seen.contains(chart.initial.as_str()) {
        return Err(ScxmlError::InvalidInitial(chart.initial.to_string()));
    }

    // Validate each state (only top-level; recursion handles children).
    let limit = crate::max_depth();
    for state in &chart.states {
        validate_state(state, false, 0, limit)?;
    }

    // Check all transition targets.
    for state in chart.iter_all_states() {
        for t in &state.transitions {
            for target in &t.targets {
                if !seen.contains(target.as_str()) {
                    return Err(ScxmlError::UnknownTarget {
                        src: state.id.to_string(),
                        target: target.to_string(),
                    });
                }
            }
        }
    }

    Ok(())
}

fn validate_state(state: &State, is_child: bool, depth: usize, limit: usize) -> Result<()> {
    if depth > limit {
        return Err(ScxmlError::DepthLimitExceeded {
            state: state.id.to_string(),
            limit,
        });
    }
    match state.kind {
        StateKind::Final if !state.transitions.is_empty() => {
            return Err(ScxmlError::FinalHasTransitions(state.id.to_string()));
        }
        StateKind::Compound => {
            // Must have an initial child (either explicit or first child).
            if let Some(ref init) = state.initial {
                if !state.children.iter().any(|c| c.id == *init) {
                    return Err(ScxmlError::CompoundNoInitial(state.id.to_string()));
                }
            } else if state.children.is_empty() {
                // Shouldn't happen: a compound with no children should be atomic.
                return Err(ScxmlError::CompoundNoInitial(state.id.to_string()));
            }
            // If no explicit initial, first child is the implicit initial. Valid.
        }
        StateKind::Parallel if state.children.len() < 2 => {
            return Err(ScxmlError::ParallelTooFewRegions(state.id.to_string()));
        }
        StateKind::History(_) if !is_child => {
            return Err(ScxmlError::OrphanHistory(state.id.to_string()));
        }
        StateKind::Atomic if !state.children.is_empty() => {
            return Err(ScxmlError::Xml(format!(
                "atomic state '{}' has children (should be compound or parallel)",
                state.id
            )));
        }
        _ => {}
    }

    // Recurse into children, marking them as children.
    for child in &state.children {
        validate_state(child, true, depth + 1, limit)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{State, Transition};

    fn simple_chart() -> Statechart {
        Statechart::new(
            "draft",
            vec![
                {
                    let mut s = State::atomic("draft");
                    s.transitions.push(Transition::new("submit", "review"));
                    s
                },
                {
                    let mut s = State::atomic("review");
                    s.transitions.push(Transition::new("approve", "done"));
                    s
                },
                State::final_state("done"),
            ],
        )
    }

    #[test]
    fn valid_chart_passes() {
        assert!(validate_structure(&simple_chart()).is_ok());
    }

    #[test]
    fn unknown_target_fails() {
        let mut chart = simple_chart();
        chart.states[0].transitions[0].targets = vec!["nonexistent".into()];
        let err = validate_structure(&chart).unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn duplicate_id_fails() {
        let chart = Statechart::new("a", vec![State::atomic("a"), State::atomic("a")]);
        let err = validate_structure(&chart).unwrap_err();
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn invalid_initial_fails() {
        let chart = Statechart::new("missing", vec![State::atomic("a")]);
        let err = validate_structure(&chart).unwrap_err();
        assert!(err.to_string().contains("missing"));
    }

    #[test]
    fn final_with_transitions_fails() {
        let mut chart = Statechart::new("done", vec![State::final_state("done")]);
        chart.states[0]
            .transitions
            .push(Transition::new("x", "done"));
        let err = validate_structure(&chart).unwrap_err();
        assert!(err.to_string().contains("final"));
    }

    #[test]
    fn parallel_one_region_fails() {
        let chart = Statechart::new("p", vec![State::parallel("p", vec![State::atomic("r1")])]);
        let err = validate_structure(&chart).unwrap_err();
        assert!(err.to_string().contains("at least 2"));
    }
}
