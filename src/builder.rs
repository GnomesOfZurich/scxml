//! Ergonomic builder for constructing [`Statechart`]s programmatically.
//!
//! ```rust
//! use scxml::builder::StatechartBuilder;
//!
//! let chart = StatechartBuilder::new("draft")
//!     .name("npa-approval")
//!     .state("draft", |s| {
//!         s.on_event("submit", "review").set_guard("has_documents");
//!     })
//!     .state("review", |s| {
//!         s.on_event("approve", "approved").set_guard("approval.committee");
//!         s.on_event("reject", "draft");
//!     })
//!     .final_state("approved")
//!     .build();
//!
//! assert_eq!(chart.states.len(), 3);
//! ```

use compact_str::CompactString;

use crate::model::{Action, DataItem, DataModel, State, Statechart, Transition};

/// Builder for constructing a [`Statechart`].
pub struct StatechartBuilder {
    initial: CompactString,
    name: Option<CompactString>,
    states: Vec<State>,
    datamodel: DataModel,
}

/// Builder for adding transitions and actions to a state.
pub struct StateBuilder {
    state: State,
}

impl StatechartBuilder {
    /// Start building a statechart with the given initial state id.
    pub fn new(initial: impl Into<CompactString>) -> Self {
        Self {
            initial: initial.into(),
            name: None,
            states: Vec::new(),
            datamodel: DataModel::default(),
        }
    }

    /// Set the statechart name.
    pub fn name(mut self, name: impl Into<CompactString>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add an atomic state with transitions configured via the closure.
    pub fn state(
        mut self,
        id: impl Into<CompactString>,
        f: impl FnOnce(&mut StateBuilder),
    ) -> Self {
        let mut sb = StateBuilder {
            state: State::atomic(id),
        };
        f(&mut sb);
        self.states.push(sb.state);
        self
    }

    /// Add a final (terminal) state.
    pub fn final_state(mut self, id: impl Into<CompactString>) -> Self {
        self.states.push(State::final_state(id));
        self
    }

    /// Add a compound state with children configured via the closure.
    pub fn compound(
        mut self,
        id: impl Into<CompactString>,
        initial_child: impl Into<CompactString>,
        f: impl FnOnce(&mut CompoundBuilder),
    ) -> Self {
        let mut cb = CompoundBuilder { states: Vec::new() };
        f(&mut cb);
        self.states
            .push(State::compound(id, initial_child, cb.states));
        self
    }

    /// Add a parallel state with regions configured via the closure.
    pub fn parallel(
        mut self,
        id: impl Into<CompactString>,
        f: impl FnOnce(&mut CompoundBuilder),
    ) -> Self {
        let mut cb = CompoundBuilder { states: Vec::new() };
        f(&mut cb);
        self.states.push(State::parallel(id, cb.states));
        self
    }

    /// Add a data item declaration.
    pub fn data(mut self, id: impl Into<CompactString>) -> Self {
        self.datamodel.items.push(DataItem::new(id));
        self
    }

    /// Build the statechart.
    pub fn build(self) -> Statechart {
        let mut chart = Statechart::new(self.initial, self.states);
        chart.name = self.name;
        chart.datamodel = self.datamodel;
        chart
    }
}

/// Builder for child states inside a compound or parallel state.
pub struct CompoundBuilder {
    states: Vec<State>,
}

impl CompoundBuilder {
    /// Add an atomic child state.
    pub fn state(&mut self, id: impl Into<CompactString>, f: impl FnOnce(&mut StateBuilder)) {
        let mut sb = StateBuilder {
            state: State::atomic(id),
        };
        f(&mut sb);
        self.states.push(sb.state);
    }

    /// Add a final child state.
    pub fn final_state(&mut self, id: impl Into<CompactString>) {
        self.states.push(State::final_state(id));
    }

    /// Add a nested compound child state.
    pub fn compound(
        &mut self,
        id: impl Into<CompactString>,
        initial_child: impl Into<CompactString>,
        f: impl FnOnce(&mut CompoundBuilder),
    ) {
        let mut cb = CompoundBuilder { states: Vec::new() };
        f(&mut cb);
        self.states
            .push(State::compound(id, initial_child, cb.states));
    }
}

impl StateBuilder {
    /// Add a transition triggered by an event to a target state.
    /// Returns the builder so you can chain `.with_guard()` etc.
    pub fn on_event(
        &mut self,
        event: impl Into<CompactString>,
        target: impl Into<CompactString>,
    ) -> &mut Transition {
        self.state.transitions.push(Transition::new(event, target));
        self.state.transitions.last_mut().unwrap()
    }

    /// Add an eventless transition to a target state.
    pub fn always(&mut self, target: impl Into<CompactString>) -> &mut Transition {
        self.state.transitions.push(Transition::eventless(target));
        self.state.transitions.last_mut().unwrap()
    }

    /// Add an entry action.
    pub fn on_entry(&mut self, action: Action) -> &mut Self {
        self.state.on_entry.push(action);
        self
    }

    /// Add an exit action.
    pub fn on_exit(&mut self, action: Action) -> &mut Self {
        self.state.on_exit.push(action);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::StateKind;
    use crate::validate;

    #[test]
    fn builder_simple_workflow() {
        let chart = StatechartBuilder::new("draft")
            .name("test")
            .state("draft", |s| {
                s.on_event("submit", "review").set_guard("has_documents");
            })
            .state("review", |s| {
                s.on_event("approve", "done");
                s.on_event("reject", "draft");
            })
            .final_state("done")
            .build();

        validate(&chart).unwrap();
        assert_eq!(chart.name.as_deref(), Some("test"));
        assert_eq!(chart.states.len(), 3);
        assert_eq!(chart.states[0].transitions.len(), 1);
        assert_eq!(
            chart.states[0].transitions[0].guard.as_deref(),
            Some("has_documents")
        );
    }

    #[test]
    fn builder_with_compound() {
        let chart = StatechartBuilder::new("main")
            .compound("main", "a", |c| {
                c.state("a", |s| {
                    s.on_event("next", "b");
                });
                c.state("b", |s| {
                    s.on_event("done", "end");
                });
                c.final_state("end");
            })
            .build();

        validate(&chart).unwrap();
        assert_eq!(chart.states[0].kind, StateKind::Compound);
        assert_eq!(chart.states[0].children.len(), 3);
    }

    #[test]
    fn builder_with_delay_and_quorum() {
        let chart = StatechartBuilder::new("pending")
            .state("pending", |s| {
                s.on_event("approve", "done")
                    .set_guard("approval.committee")
                    .set_quorum(3);
                s.on_event("timeout", "expired").set_delay("PT48H");
            })
            .final_state("done")
            .final_state("expired")
            .build();

        validate(&chart).unwrap();
        assert_eq!(chart.states[0].transitions[0].quorum, Some(3));
        assert_eq!(
            chart.states[0].transitions[1].delay.as_deref(),
            Some("PT48H")
        );
    }
}
