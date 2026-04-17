//! Statechart metrics: state counts, transition density, nesting depth.

use serde::{Deserialize, Serialize};

use crate::model::{StateKind, Statechart};

/// Summary metrics for a statechart.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StatechartStats {
    /// Total number of states (including nested).
    pub total_states: usize,
    /// Number of atomic (leaf) states.
    pub atomic_states: usize,
    /// Number of compound states.
    pub compound_states: usize,
    /// Number of parallel states.
    pub parallel_states: usize,
    /// Number of final states.
    pub final_states: usize,
    /// Number of history pseudo-states.
    pub history_states: usize,
    /// Total number of transitions.
    pub total_transitions: usize,
    /// Number of guarded transitions.
    pub guarded_transitions: usize,
    /// Number of transitions with a delay.
    pub deadline_transitions: usize,
    /// Maximum nesting depth (0 = flat, 1 = one level of compound/parallel).
    pub max_depth: usize,
    /// Number of data model declarations.
    pub data_items: usize,
    /// Total number of actions across all states (entry, exit, transition actions).
    pub total_actions: usize,
}

/// Compute summary metrics for a statechart.
///
/// ```rust
/// use scxml::{parse_xml, stats};
///
/// let xml = r#"
///     <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
///         <state id="a"><transition event="go" target="b"/></state>
///         <final id="b"/>
///     </scxml>
/// "#;
/// let chart = parse_xml(xml).unwrap();
/// let s = stats(&chart);
/// assert_eq!(s.total_states, 2);
/// assert_eq!(s.final_states, 1);
/// ```
pub fn stats(chart: &Statechart) -> StatechartStats {
    let mut s = StatechartStats {
        total_states: 0,
        atomic_states: 0,
        compound_states: 0,
        parallel_states: 0,
        final_states: 0,
        history_states: 0,
        total_transitions: 0,
        guarded_transitions: 0,
        deadline_transitions: 0,
        max_depth: 0,
        data_items: chart.datamodel.items.len(),
        total_actions: 0,
    };

    let limit = crate::max_depth();
    for state in &chart.states {
        collect_stats(state, 0, limit, &mut s);
    }

    s
}

fn collect_stats(state: &crate::model::State, depth: usize, limit: usize, s: &mut StatechartStats) {
    if depth > limit {
        return;
    }
    s.total_states += 1;
    if depth > s.max_depth {
        s.max_depth = depth;
    }

    match state.kind {
        StateKind::Atomic => s.atomic_states += 1,
        StateKind::Compound => s.compound_states += 1,
        StateKind::Parallel => s.parallel_states += 1,
        StateKind::Final => s.final_states += 1,
        StateKind::History(_) => s.history_states += 1,
    }

    for t in &state.transitions {
        s.total_transitions += 1;
        if t.guard.is_some() {
            s.guarded_transitions += 1;
        }
        if t.delay.is_some() {
            s.deadline_transitions += 1;
        }
        count_actions(&t.actions, s);
    }
    count_actions(&state.on_entry, s);
    count_actions(&state.on_exit, s);

    for child in &state.children {
        collect_stats(child, depth + 1, limit, s);
    }
}

fn count_actions(actions: &[crate::model::Action], s: &mut StatechartStats) {
    for action in actions {
        s.total_actions += 1;
        match &action.kind {
            crate::model::ActionKind::If { actions, .. } => count_actions(actions, s),
            crate::model::ActionKind::Foreach { actions, .. } => count_actions(actions, s),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{State, Transition};

    #[test]
    fn stats_simple_chart() {
        let chart = Statechart::new(
            "a",
            vec![
                {
                    let mut s = State::atomic("a");
                    s.transitions
                        .push(Transition::new("go", "b").with_guard("ready"));
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

        let s = stats(&chart);
        assert_eq!(s.total_states, 3);
        assert_eq!(s.atomic_states, 2);
        assert_eq!(s.final_states, 1);
        assert_eq!(s.total_transitions, 2);
        assert_eq!(s.guarded_transitions, 1);
        assert_eq!(s.max_depth, 0);
    }

    #[test]
    fn stats_nested_chart() {
        let chart = Statechart::new(
            "main",
            vec![State::compound(
                "main",
                "child",
                vec![
                    {
                        let mut s = State::atomic("child");
                        s.transitions.push(Transition::new("done", "end"));
                        s
                    },
                    State::final_state("end"),
                ],
            )],
        );

        let s = stats(&chart);
        assert_eq!(s.total_states, 3); // main + child + end
        assert_eq!(s.compound_states, 1);
        assert_eq!(s.max_depth, 1);
    }
}
