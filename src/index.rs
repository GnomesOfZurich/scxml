//! Pre-built index over a statechart's state tree.
//!
//! Building the index walks the tree once. The resulting [`StateIndex`] provides
//! O(1) lookups by state ID and parent ID, used by validation, simulation, and
//! any code that needs fast random access into the state tree.

use std::collections::HashMap;

use crate::model::{State, Statechart};

/// Pre-built index over a statechart's state tree.
///
/// Borrows from the `Statechart`; the index is valid as long as the chart is.
///
/// ```rust
/// use scxml::{parse_xml, StateIndex};
///
/// let chart = parse_xml(r#"
///     <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
///         <state id="a"><transition event="go" target="b"/></state>
///         <final id="b"/>
///     </scxml>
/// "#).unwrap();
///
/// let index = StateIndex::new(&chart);
/// assert_eq!(index.state_count(), 2);
/// assert!(index.get("a").is_some());
/// assert!(index.get("missing").is_none());
/// ```
pub struct StateIndex<'a> {
    /// State ID → &State for O(1) lookup.
    states: HashMap<&'a str, &'a State>,
    /// Child state ID → parent state ID.
    parents: HashMap<&'a str, &'a str>,
}

impl<'a> StateIndex<'a> {
    /// Build an index over all states in the chart (single tree walk).
    pub fn new(chart: &'a Statechart) -> Self {
        let estimated = chart.states.len() * 4;
        let mut states = HashMap::with_capacity(estimated);
        let mut parents = HashMap::with_capacity(estimated);
        let limit = crate::max_depth();
        build(&chart.states, &mut states, &mut parents, None, 0, limit);
        Self { states, parents }
    }

    /// Look up a state by ID.
    pub fn get(&self, id: &str) -> Option<&&'a State> {
        self.states.get(id)
    }

    /// Look up the parent state ID of a given state.
    pub fn parent(&self, id: &str) -> Option<&'a str> {
        self.parents.get(id).copied()
    }

    /// Total number of indexed states.
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Returns the inner state map (for validation/liveness).
    pub(crate) fn state_map(&self) -> &HashMap<&'a str, &'a State> {
        &self.states
    }

    /// Returns the inner parent map (for simulation).
    pub(crate) fn parent_map(&self) -> &HashMap<&'a str, &'a str> {
        &self.parents
    }

    /// Check whether a state ID exists.
    pub fn contains(&self, id: &str) -> bool {
        self.states.contains_key(id)
    }

    /// Iterate over all (id, state) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&'a str, &&'a State)> {
        self.states.iter().map(|(&k, v)| (k, v))
    }
}

fn build<'a>(
    states: &'a [State],
    state_map: &mut HashMap<&'a str, &'a State>,
    parent_map: &mut HashMap<&'a str, &'a str>,
    parent_id: Option<&'a str>,
    depth: usize,
    limit: usize,
) {
    if depth > limit {
        return;
    }
    for state in states {
        state_map.insert(state.id.as_str(), state);
        if let Some(pid) = parent_id {
            parent_map.insert(state.id.as_str(), pid);
        }
        if !state.children.is_empty() {
            build(
                &state.children,
                state_map,
                parent_map,
                Some(state.id.as_str()),
                depth + 1,
                limit,
            );
        }
    }
}
