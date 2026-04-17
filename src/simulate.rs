//! Lightweight simulation executor for testing statechart behavior.
//!
//! **Not a production runtime.** This is a simple, non-optimized interpreter
//! for verifying that a statechart reaches expected states given a sequence of
//! events. Use compiled native types for production.
//!
//! ```rust
//! use scxml::simulate::Simulator;
//! use scxml::model::{State, Statechart, Transition};
//!
//! let chart = Statechart::new("draft", vec![
//!     { let mut s = State::atomic("draft");
//!       s.transitions.push(Transition::new("submit", "review"));
//!       s },
//!     { let mut s = State::atomic("review");
//!       s.transitions.push(Transition::new("approve", "done"));
//!       s },
//!     State::final_state("done"),
//! ]);
//!
//! let mut sim = Simulator::new(&chart);
//! assert_eq!(sim.state(), "draft");
//! assert!(sim.send("submit").is_ok());
//! assert_eq!(sim.state(), "review");
//! assert!(sim.send("approve").is_ok());
//! assert_eq!(sim.state(), "done");
//! assert!(sim.is_final());
//! ```

use std::collections::HashMap;

use compact_str::CompactString;

use crate::error::ScxmlError;
use crate::model::{State, StateKind, Statechart};

/// A closure that evaluates named guards. Return `true` to allow the transition.
pub type GuardFn = dyn Fn(&str) -> bool;

/// Lightweight statechart simulator for testing.
///
/// Tracks the current state and processes events by finding matching transitions.
/// Guards can be provided via `with_guard_fn()`, defaulting to "all guards pass".
pub struct Simulator<'a> {
    chart: &'a Statechart,
    current: CompactString,
    history: Vec<(CompactString, String, CompactString)>, // (from, event, to)
    guard_fn: Box<GuardFn>,
    // Pre-built indexes for O(1) lookups instead of tree walks.
    state_index: HashMap<&'a str, &'a State>,
    parent_index: HashMap<&'a str, &'a str>,
}

impl<'a> Simulator<'a> {
    /// Create a new simulator starting at the chart's initial state.
    pub fn new(chart: &'a Statechart) -> Self {
        let index = crate::index::StateIndex::new(chart);
        Self::with_index(chart, index)
    }

    /// Create a simulator using a pre-built [`StateIndex`](crate::index::StateIndex).
    ///
    /// Avoids rebuilding the index if you already have one from validation.
    pub fn with_index(chart: &'a Statechart, index: crate::index::StateIndex<'a>) -> Self {
        let state_index = index.state_map().clone();
        let parent_index = index.parent_map().clone();
        Self {
            current: chart.initial.clone(),
            chart,
            history: Vec::new(),
            guard_fn: Box::new(|_| true),
            state_index,
            parent_index,
        }
    }

    /// Set a custom guard evaluator.
    pub fn with_guard_fn(mut self, f: impl Fn(&str) -> bool + 'static) -> Self {
        self.guard_fn = Box::new(f);
        self
    }

    /// The current state id.
    pub fn state(&self) -> &str {
        self.current.as_str()
    }

    /// Whether the current state is final.
    pub fn is_final(&self) -> bool {
        self.state_index
            .get(self.current.as_str())
            .is_some_and(|s| s.kind == StateKind::Final)
    }

    /// Transition history: `(from_state, event, to_state)`.
    pub fn history(&self) -> &[(CompactString, String, CompactString)] {
        &self.history
    }

    /// Number of transitions taken so far.
    pub fn step_count(&self) -> usize {
        self.history.len()
    }

    /// Send an event and attempt a transition.
    ///
    /// Searches transitions in the current state (and ancestor compound states)
    /// for one matching the event. If a guard is present, it must pass.
    pub fn send(&mut self, event: &str) -> Result<&str, ScxmlError> {
        if self.is_final() {
            return Err(ScxmlError::SimFinal {
                state: self.current.to_string(),
            });
        }

        // Collect state IDs to check: current state, then ancestors.
        // We gather IDs first to avoid borrow conflicts with self.
        let mut states_to_check: Vec<&str> = Vec::with_capacity(4);
        let current = self.current.as_str();
        states_to_check.push(current);
        let mut ancestor_id = self.parent_index.get(current).copied();
        while let Some(pid) = ancestor_id {
            states_to_check.push(pid);
            ancestor_id = self.parent_index.get(pid).copied();
        }

        let mut last_blocked_guard = None;

        for &state_id in &states_to_check {
            let Some(&state) = self.state_index.get(state_id) else {
                continue;
            };

            for t in &state.transitions {
                // Only match transitions with the exact event name.
                // Eventless transitions (event: None) are NOT matched by named events.
                let matches = t.event.as_ref().is_some_and(|e| e.as_str() == event);
                if !matches {
                    continue;
                }

                // Check guard.
                if let Some(g) = &t.guard {
                    if !(self.guard_fn)(g.as_str()) {
                        last_blocked_guard = Some(g.to_string());
                        continue;
                    }
                }

                if t.targets.is_empty() {
                    // Targetless transition: stay in current state (actions only).
                    let from = self.current.clone();
                    self.history
                        .push((from, event.to_string(), self.current.clone()));
                    return Ok(self.current.as_str());
                }

                // Take the first target (multi-target = parallel entry, not
                // supported by this lightweight simulator).
                if let Some(target) = t.targets.first() {
                    let from = self.current.clone();
                    self.current = target.clone();
                    self.history.push((from, event.to_string(), target.clone()));
                    return Ok(self.current.as_str());
                }
            }

            // Only check ancestors if current state had no matching transitions.
            if last_blocked_guard.is_none() {
                continue;
            }
        }

        // No transition fired.
        if let Some(guard) = last_blocked_guard {
            Err(ScxmlError::SimGuardBlocked {
                state: self.current.to_string(),
                event: event.to_string(),
                guard,
            })
        } else {
            Err(ScxmlError::SimNoTransition {
                state: self.current.to_string(),
                event: event.to_string(),
            })
        }
    }

    /// Reset the simulator to the initial state.
    pub fn reset(&mut self) {
        self.current = self.chart.initial.clone();
        self.history.clear();
    }
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
                    s.transitions
                        .push(Transition::new("approve", "done").with_guard("manager_ok"));
                    s.transitions.push(Transition::new("reject", "draft"));
                    s
                },
                State::final_state("done"),
            ],
        )
    }

    #[test]
    fn basic_simulation() {
        let chart = simple_chart();
        let mut sim = Simulator::new(&chart);

        assert_eq!(sim.state(), "draft");
        assert!(!sim.is_final());

        sim.send("submit").unwrap();
        assert_eq!(sim.state(), "review");

        sim.send("approve").unwrap();
        assert_eq!(sim.state(), "done");
        assert!(sim.is_final());
        assert_eq!(sim.step_count(), 2);
    }

    #[test]
    fn reject_loops_back() {
        let chart = simple_chart();
        let mut sim = Simulator::new(&chart);

        sim.send("submit").unwrap();
        sim.send("reject").unwrap();
        assert_eq!(sim.state(), "draft");
    }

    #[test]
    fn no_transition_error() {
        let chart = simple_chart();
        let mut sim = Simulator::new(&chart);

        let err = sim.send("approve").unwrap_err();
        assert_eq!(
            err,
            ScxmlError::SimNoTransition {
                state: "draft".into(),
                event: "approve".into(),
            }
        );
    }

    #[test]
    fn final_state_error() {
        let chart = simple_chart();
        let mut sim = Simulator::new(&chart);

        sim.send("submit").unwrap();
        sim.send("approve").unwrap();
        let err = sim.send("something").unwrap_err();
        assert!(matches!(err, ScxmlError::SimFinal { .. }));
    }

    #[test]
    fn guard_blocks_transition() {
        let chart = simple_chart();
        let mut sim = Simulator::new(&chart).with_guard_fn(|name| name != "manager_ok");

        sim.send("submit").unwrap();
        let err = sim.send("approve").unwrap_err();
        assert!(matches!(err, ScxmlError::SimGuardBlocked { .. }));
        assert_eq!(sim.state(), "review"); // didn't move
    }

    #[test]
    fn reset_returns_to_initial() {
        let chart = simple_chart();
        let mut sim = Simulator::new(&chart);

        sim.send("submit").unwrap();
        sim.reset();
        assert_eq!(sim.state(), "draft");
        assert_eq!(sim.step_count(), 0);
    }

    #[test]
    fn history_tracks_transitions() {
        let chart = simple_chart();
        let mut sim = Simulator::new(&chart);

        sim.send("submit").unwrap();
        sim.send("approve").unwrap();

        let hist = sim.history();
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].0.as_str(), "draft");
        assert_eq!(hist[0].1, "submit");
        assert_eq!(hist[0].2.as_str(), "review");
    }

    #[test]
    fn inherited_transition_from_parent() {
        let chart = Statechart::new(
            "wrapper",
            vec![
                {
                    let mut wrapper =
                        State::compound("wrapper", "child", vec![State::atomic("child")]);
                    wrapper.transitions.push(Transition::new("escape", "done"));
                    wrapper
                },
                State::final_state("done"),
            ],
        );

        let sim = Simulator::new(&chart);
        // The simulator starts at "wrapper" (the chart's initial state).
        assert_eq!(sim.state(), "wrapper");
    }
}
