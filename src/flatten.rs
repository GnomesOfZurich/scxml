use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use crate::model::state::StateKind;
use crate::model::{State, Statechart};

/// A flat state representation for frontend rendering.
///
/// Matches the shape expected by statechart visualization components
/// (e.g. `scxmlTypes.ts::flattenMachine()`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub struct FlatState {
    /// State identifier.
    pub id: CompactString,
    /// What kind of state this is.
    pub kind: StateKind,
    /// Parent state id, if nested.
    pub parent: Option<CompactString>,
    /// Whether this is the chart's initial state.
    pub initial: bool,
    /// Nesting depth (0 = top level).
    pub depth: u32,
}

/// A flat transition representation for frontend rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub struct FlatTransition {
    /// Source state id.
    pub source: CompactString,
    /// Target state id.
    pub target: CompactString,
    /// Triggering event, if any.
    pub event: Option<CompactString>,
    /// Guard condition name, if any.
    pub guard: Option<CompactString>,
}

/// Flatten a statechart into lists of states and transitions suitable for
/// frontend rendering.
///
/// ```rust
/// use scxml::{parse_xml, flatten};
///
/// let xml = r#"
///     <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
///         <state id="a"><transition event="go" target="b"/></state>
///         <final id="b"/>
///     </scxml>
/// "#;
/// let chart = parse_xml(xml).unwrap();
/// let (states, transitions) = flatten(&chart);
/// assert_eq!(states.len(), 2);
/// assert_eq!(transitions.len(), 1);
/// ```
///
/// Walks the state tree depth-first, producing flat lists with parent
/// references and depth indicators. Transitions are denormalized to
/// source→target pairs (one per target for multi-target transitions).
pub fn flatten(chart: &Statechart) -> (Vec<FlatState>, Vec<FlatTransition>) {
    let (state_count, trans_count) = {
        let mut sc = 0;
        let mut tc = 0;
        for s in chart.iter_all_states() {
            sc += 1;
            tc += s.transitions.len();
        }
        (sc, tc)
    };
    let mut states = Vec::with_capacity(state_count);
    let mut transitions = Vec::with_capacity(trans_count);

    let limit = crate::max_depth();
    for state in &chart.states {
        flatten_state(
            state,
            None,
            0,
            &chart.initial,
            &mut states,
            &mut transitions,
            limit,
        );
    }

    (states, transitions)
}

fn flatten_state(
    state: &State,
    parent: Option<&CompactString>,
    depth: u32,
    chart_initial: &CompactString,
    states: &mut Vec<FlatState>,
    transitions: &mut Vec<FlatTransition>,
    limit: usize,
) {
    if depth as usize > limit {
        return;
    }
    let is_initial = state.id == *chart_initial;

    states.push(FlatState {
        id: state.id.clone(),
        kind: state.kind,
        parent: parent.cloned(),
        initial: is_initial,
        depth,
    });

    // Flatten transitions.
    for t in &state.transitions {
        for target in &t.targets {
            transitions.push(FlatTransition {
                source: state.id.clone(),
                target: target.clone(),
                event: t.event.clone(),
                guard: t.guard.clone(),
            });
        }
    }

    // Recurse into children.
    for child in &state.children {
        flatten_state(
            child,
            Some(&state.id),
            depth + 1,
            chart_initial,
            states,
            transitions,
            limit,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{State, Transition};

    #[test]
    fn flatten_simple_chart() {
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

        let (states, transitions) = flatten(&chart);
        assert_eq!(states.len(), 3);
        assert_eq!(transitions.len(), 2);
        assert!(states[0].initial);
        assert_eq!(states[0].depth, 0);
    }

    #[test]
    fn flatten_nested_chart() {
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
                    State::atomic("child_b"),
                ],
            )],
        );

        let (states, _) = flatten(&chart);
        assert_eq!(states.len(), 3); // main + child_a + child_b
        assert_eq!(states[0].depth, 0);
        assert_eq!(states[1].depth, 1);
        assert_eq!(states[1].parent.as_deref(), Some("main"));
    }
}
