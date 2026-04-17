use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::{Result, ScxmlError};
use crate::model::{State, StateKind, Statechart};

/// Perform liveness validation on a structurally-valid statechart.
///
/// Checks:
/// - All states are reachable from the initial configuration (BFS over transitions)
/// - Non-final, non-history states without outgoing transitions are flagged as deadlocks
///
/// Call [`validate_structure`](super::structural::validate_structure) first.
pub fn validate_liveness(chart: &Statechart, state_count: usize) -> Result<()> {
    let index = crate::index::StateIndex::new(chart);
    validate_liveness_with_index(chart, state_count, &index)
}

/// Liveness validation using a pre-built [`StateIndex`](crate::index::StateIndex).
pub fn validate_liveness_with_index(
    chart: &Statechart,
    state_count: usize,
    index: &crate::index::StateIndex<'_>,
) -> Result<()> {
    check_reachability(chart, state_count, index.state_map())?;
    check_deadlocks(chart, state_count)?;
    Ok(())
}

/// BFS from initial state following transitions and parent→child edges.
fn check_reachability(
    chart: &Statechart,
    state_count: usize,
    state_index: &HashMap<&str, &State>,
) -> Result<()> {
    let mut reachable: HashSet<&str> = HashSet::with_capacity(state_count);
    let mut queue: VecDeque<&str> = VecDeque::with_capacity(state_count);

    // Seed with initial state.
    queue.push_back(chart.initial.as_str());

    while let Some(id) = queue.pop_front() {
        if !reachable.insert(id) {
            continue;
        }

        if let Some(&state) = state_index.get(id) {
            // Follow transitions.
            for t in &state.transitions {
                for target in &t.targets {
                    queue.push_back(target.as_str());
                }
            }

            // Follow parent→child edges (entering a compound/parallel enters children).
            match state.kind {
                StateKind::Compound => {
                    if let Some(ref init) = state.initial {
                        queue.push_back(init.as_str());
                    } else if let Some(first) = state.children.first() {
                        queue.push_back(first.id.as_str());
                    }
                }
                StateKind::Parallel => {
                    // Entering a parallel state enters all children.
                    for child in &state.children {
                        queue.push_back(child.id.as_str());
                    }
                }
                _ => {}
            }

            // Children of reachable compound/parallel states may have their
            // own initial children.
            for child in &state.children {
                if child.kind == StateKind::Compound || child.kind == StateKind::Parallel {
                    queue.push_back(child.id.as_str());
                }
            }
        }
    }

    // Check all states are reachable.
    for state in chart.iter_all_states() {
        // History states are reachable if their parent is.
        if matches!(state.kind, StateKind::History(_)) {
            continue;
        }
        if !reachable.contains(state.id.as_str()) {
            return Err(ScxmlError::Unreachable(state.id.to_string()));
        }
    }

    Ok(())
}

fn check_deadlocks(chart: &Statechart, state_count: usize) -> Result<()> {
    // Build parent map: child_id → whether any ancestor has transitions.
    // In W3C SCXML, transitions on a compound/parallel state are inherited
    // by all descendants. A child without its own transitions is NOT a
    // deadlock if an ancestor has outgoing transitions.
    let mut parent_transitions: HashMap<&str, bool> = HashMap::with_capacity(state_count);
    let limit = crate::max_depth();
    build_parent_has_transitions(&chart.states, &mut parent_transitions, false, 0, limit);

    for state in chart.iter_all_states() {
        match state.kind {
            StateKind::Final
            | StateKind::History(_)
            | StateKind::Compound
            | StateKind::Parallel => {}
            StateKind::Atomic => {
                if state.transitions.is_empty() {
                    // Check if any ancestor has transitions.
                    let inherited = parent_transitions
                        .get(state.id.as_str())
                        .copied()
                        .unwrap_or(false);
                    if !inherited {
                        return Err(ScxmlError::Deadlock(state.id.to_string()));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Recursively mark children with whether any ancestor has transitions.
fn build_parent_has_transitions<'a>(
    states: &'a [crate::model::State],
    map: &mut HashMap<&'a str, bool>,
    ancestor_has_transitions: bool,
    depth: usize,
    limit: usize,
) {
    if depth > limit {
        return;
    }
    for state in states {
        let this_has = ancestor_has_transitions || !state.transitions.is_empty();
        for child in &state.children {
            map.insert(child.id.as_str(), this_has);
        }
        // Recurse into children's descendants (not siblings).
        if !state.children.is_empty() {
            build_parent_has_transitions(&state.children, map, this_has, depth + 1, limit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{State, Transition};

    #[test]
    fn all_reachable() {
        let chart = Statechart::new(
            "a",
            vec![
                {
                    let mut s = State::atomic("a");
                    s.transitions.push(Transition::new("go", "b"));
                    s
                },
                {
                    let mut s = State::atomic("b");
                    s.transitions.push(Transition::new("done", "end"));
                    s
                },
                State::final_state("end"),
            ],
        );
        assert!(validate_liveness(&chart, chart.iter_all_states().count()).is_ok());
    }

    #[test]
    fn unreachable_state() {
        let chart = Statechart::new(
            "a",
            vec![
                {
                    let mut s = State::atomic("a");
                    s.transitions.push(Transition::new("done", "end"));
                    s
                },
                State::atomic("orphan"), // no transition leads here
                State::final_state("end"),
            ],
        );
        let err = validate_liveness(&chart, chart.iter_all_states().count());
        // orphan is unreachable, but it also has no transitions, so deadlock
        // might fire first depending on iteration order. Either error is valid.
        assert!(err.is_err());
    }

    #[test]
    fn deadlock_detected() {
        let chart = Statechart::new(
            "a",
            vec![
                {
                    let mut s = State::atomic("a");
                    s.transitions.push(Transition::new("go", "b"));
                    s
                },
                State::atomic("b"), // reachable but no transitions and not final
            ],
        );
        let err = validate_liveness(&chart, chart.iter_all_states().count()).unwrap_err();
        assert!(err.to_string().contains("deadlock") || err.to_string().contains("no outgoing"));
    }

    #[test]
    fn child_with_parent_transition_not_deadlock() {
        // Parent compound state has a transition; children inherit it.
        let chart = Statechart::new(
            "wrapper",
            vec![
                {
                    let mut wrapper = State::compound(
                        "wrapper",
                        "child",
                        vec![
                            State::atomic("child"), // no own transitions
                        ],
                    );
                    wrapper.transitions.push(Transition::new("escape", "done"));
                    wrapper
                },
                State::final_state("done"),
            ],
        );
        // "child" has no transitions but parent "wrapper" does; not a deadlock.
        assert!(validate_liveness(&chart, chart.iter_all_states().count()).is_ok());
    }

    #[test]
    fn compound_children_reachable() {
        let chart = Statechart::new(
            "main",
            vec![State::compound(
                "main",
                "child_a",
                vec![
                    {
                        let mut s = State::atomic("child_a");
                        s.transitions.push(Transition::new("next", "child_b"));
                        s
                    },
                    {
                        let mut s = State::atomic("child_b");
                        s.transitions.push(Transition::new("done", "end"));
                        s
                    },
                    State::final_state("end"),
                ],
            )],
        );
        assert!(validate_liveness(&chart, chart.iter_all_states().count()).is_ok());
    }
}
