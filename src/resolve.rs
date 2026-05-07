//! Semantic resolution: pre-computed effective transitions and event catalogs.
//!
//! [`resolve()`] transforms a [`Statechart`] into a [`ResolvedChart`] where all
//! implicit semantics are made explicit: inherited transitions from ancestor
//! states, resolved initial children for compound regions, per-state event
//! catalogs, and hierarchy metadata.
//!
//! This is the canonical consumption layer for code generators, runtimes,
//! AI agents, and static analysis tools. The `Statechart` is the source-level
//! representation (parser output); the `ResolvedChart` is the resolved IR
//! (compiler frontend output).
//!
//! ```rust
//! use scxml::{parse_xml, resolve};
//!
//! let chart = parse_xml(r#"
//!     <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="idle">
//!         <state id="parent" initial="idle">
//!             <transition event="reset" target="idle"/>
//!             <state id="idle">
//!                 <transition event="start" target="running"/>
//!             </state>
//!             <state id="running">
//!                 <transition event="stop" target="idle"/>
//!             </state>
//!         </state>
//!     </scxml>
//! "#).unwrap();
//!
//! let resolved = resolve(&chart);
//!
//! // "running" inherits "reset" from its parent
//! let running = resolved.states.iter().find(|s| s.id == "running").unwrap();
//! assert_eq!(running.transitions.len(), 2); // own "stop" + inherited "reset"
//! assert_eq!(running.transitions[0].event.as_deref(), Some("stop"));
//! assert_eq!(running.transitions[1].event.as_deref(), Some("reset"));
//! assert_eq!(running.transitions[1].defined_in.as_str(), "parent");
//! ```

use compact_str::CompactString;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::index::StateIndex;
use crate::model::action::Action;
use crate::model::state::StateKind;
use crate::model::transition::TransitionType;
use crate::model::{DataModel, State, Statechart};

// ── Types ──────────────────────────────────────────────────────────────────────

/// A resolved view of a Statechart with pre-computed effective transitions.
///
/// Produced by [`resolve()`]. All implicit semantics are made explicit:
/// inherited transitions from ancestor states, resolved initial children
/// for compound regions, per-state event catalogs. Hierarchy is preserved.
///
/// Serializes cleanly to JSON via serde — the canonical machine-consumable
/// projection of a statechart for code generators, runtimes, and tooling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct ResolvedChart {
    /// Chart name (from source).
    pub name: Option<CompactString>,
    /// Initial state ID (from source).
    pub initial: CompactString,
    /// All states with resolved transitions.
    pub states: Vec<ResolvedState>,
    /// Complete event catalog: every event name in the chart, sorted.
    pub events: Vec<CompactString>,
    /// Data model (from source).
    pub datamodel: DataModel,
}

/// A state with its effective transitions (own + inherited from ancestors).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct ResolvedState {
    /// State identifier.
    pub id: CompactString,
    /// State kind (atomic, compound, parallel, final, history).
    pub kind: StateKind,
    /// Parent state ID (`None` for top-level states).
    pub parent: Option<CompactString>,
    /// For compound states: the declared initial child ID.
    /// For other kinds: `None`.
    pub initial_child: Option<CompactString>,
    /// Direct child state IDs (hierarchy preserved).
    pub children: Vec<CompactString>,
    /// Nesting depth (0 = top level).
    pub depth: u32,
    /// Entry actions (from source).
    pub on_entry: Vec<Action>,
    /// Exit actions (from source).
    pub on_exit: Vec<Action>,
    /// All effective transitions for this state, ordered by priority:
    /// own transitions first (document order), then parent's transitions,
    /// then grandparent's, etc. This matches the W3C transition selection
    /// algorithm where more-specific (descendant) transitions take priority.
    pub transitions: Vec<ResolvedTransition>,
}

/// A transition tagged with the state that originally defined it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct ResolvedTransition {
    /// Event trigger (`None` = eventless / always-enabled).
    pub event: Option<CompactString>,
    /// Named guard predicate (caller resolves to boolean).
    pub guard: Option<CompactString>,
    /// Target state IDs (resolved; empty = self-transition).
    pub targets: Vec<CompactString>,
    /// External (default) or internal transition.
    pub transition_type: TransitionType,
    /// Actions to execute during the transition.
    pub actions: Vec<Action>,
    /// ISO 8601 delay for timed transitions (gnomes extension).
    pub delay: Option<CompactString>,
    /// Approval quorum (gnomes extension).
    pub quorum: Option<u32>,
    /// The state that originally defined this transition.
    /// Same as the containing `ResolvedState`'s id if the transition is own;
    /// an ancestor's id if the transition is inherited.
    pub defined_in: CompactString,
}

// ── Resolution ─────────────────────────────────────────────────────────────────

/// Resolve a Statechart into a [`ResolvedChart`].
///
/// Pre-computes effective transitions (including inherited from ancestors),
/// resolved initial children, and the event catalog. Single tree walk,
/// O(n × d) where n = number of states and d = maximum depth.
///
/// Works on any `Statechart` (validated or not). Consumers should
/// [`validate`](crate::validate) first for well-formedness guarantees.
pub fn resolve(chart: &Statechart) -> ResolvedChart {
    let index = StateIndex::new(chart);
    let mut resolved_states = Vec::new();
    let mut all_events = BTreeSet::new();
    let limit = crate::max_depth();

    resolve_states(
        &chart.states,
        &index,
        None,
        0,
        limit,
        &mut resolved_states,
        &mut all_events,
    );

    ResolvedChart {
        name: chart.name.clone(),
        initial: chart.initial.clone(),
        states: resolved_states,
        events: all_events.into_iter().collect(),
        datamodel: chart.datamodel.clone(),
    }
}

fn resolve_states(
    states: &[State],
    index: &StateIndex<'_>,
    parent_id: Option<&str>,
    depth: u32,
    limit: usize,
    out: &mut Vec<ResolvedState>,
    events: &mut BTreeSet<CompactString>,
) {
    if depth as usize > limit {
        return;
    }

    for state in states {
        // Collect effective transitions: own first, then ancestors
        let mut transitions = Vec::new();

        // Own transitions (document order)
        for t in &state.transitions {
            if let Some(ref e) = t.event {
                events.insert(e.clone());
            }
            transitions.push(ResolvedTransition {
                event: t.event.clone(),
                guard: t.guard.clone(),
                targets: t.targets.clone(),
                transition_type: t.transition_type,
                actions: t.actions.clone(),
                delay: t.delay.clone(),
                quorum: t.quorum,
                defined_in: state.id.clone(),
            });
        }

        // Inherited transitions from ancestors (parent first, then grandparent, etc.)
        let mut ancestor_id = parent_id;
        while let Some(aid) = ancestor_id {
            if let Some(ancestor) = index.get(aid) {
                for t in &ancestor.transitions {
                    if let Some(ref e) = t.event {
                        events.insert(e.clone());
                    }
                    transitions.push(ResolvedTransition {
                        event: t.event.clone(),
                        guard: t.guard.clone(),
                        targets: t.targets.clone(),
                        transition_type: t.transition_type,
                        actions: t.actions.clone(),
                        delay: t.delay.clone(),
                        quorum: t.quorum,
                        defined_in: CompactString::from(aid),
                    });
                }
            }
            ancestor_id = index.parent(aid);
        }

        let initial_child = match state.kind {
            StateKind::Compound => state.initial.clone(),
            _ => None,
        };

        let children: Vec<CompactString> = state.children.iter().map(|c| c.id.clone()).collect();

        out.push(ResolvedState {
            id: state.id.clone(),
            kind: state.kind,
            parent: parent_id.map(CompactString::from),
            initial_child,
            children: children.clone(),
            depth,
            on_entry: state.on_entry.clone(),
            on_exit: state.on_exit.clone(),
            transitions,
        });

        // Recurse into children
        if !state.children.is_empty() {
            resolve_states(
                &state.children,
                index,
                Some(state.id.as_str()),
                depth + 1,
                limit,
                out,
                events,
            );
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::state::State;
    use crate::model::transition::Transition;

    fn simple_chart() -> Statechart {
        Statechart::new(
            "a",
            vec![
                {
                    let mut s = State::atomic("a");
                    s.transitions.push(Transition::new("go", "b"));
                    s
                },
                {
                    let mut s = State::atomic("b");
                    s.transitions.push(Transition::new("next", "c"));
                    s
                },
                State::final_state("c"),
            ],
        )
    }

    #[test]
    fn resolve_simple_linear() {
        let chart = simple_chart();
        let resolved = resolve(&chart);

        assert_eq!(resolved.states.len(), 3);
        assert_eq!(resolved.initial.as_str(), "a");

        let a = &resolved.states[0];
        assert_eq!(a.id.as_str(), "a");
        assert_eq!(a.transitions.len(), 1);
        assert_eq!(a.transitions[0].event.as_deref(), Some("go"));
        assert_eq!(a.transitions[0].defined_in.as_str(), "a");

        let c = &resolved.states[2];
        assert_eq!(c.kind, StateKind::Final);
        assert!(c.transitions.is_empty());
    }

    #[test]
    fn resolve_inherited_transitions() {
        let chart = Statechart::new(
            "idle",
            vec![{
                let mut parent = State::compound(
                    "parent",
                    "idle",
                    vec![
                        {
                            let mut s = State::atomic("idle");
                            s.transitions.push(Transition::new("start", "running"));
                            s
                        },
                        State::atomic("running"),
                    ],
                );
                parent.transitions.push(Transition::new("reset", "idle"));
                parent
            }],
        );

        let resolved = resolve(&chart);

        // "running" should inherit "reset" from parent
        let running = resolved.states.iter().find(|s| s.id == "running").unwrap();
        assert_eq!(running.transitions.len(), 1); // inherited "reset"
        assert_eq!(running.transitions[0].event.as_deref(), Some("reset"));
        assert_eq!(running.transitions[0].defined_in.as_str(), "parent");
    }

    #[test]
    fn resolve_own_before_inherited() {
        let chart = Statechart::new(
            "idle",
            vec![{
                let mut parent = State::compound(
                    "parent",
                    "idle",
                    vec![
                        {
                            let mut s = State::atomic("idle");
                            s.transitions.push(Transition::new("start", "running"));
                            s
                        },
                        {
                            let mut s = State::atomic("running");
                            s.transitions.push(Transition::new("stop", "idle"));
                            s
                        },
                    ],
                );
                parent.transitions.push(Transition::new("reset", "idle"));
                parent
            }],
        );

        let resolved = resolve(&chart);

        let running = resolved.states.iter().find(|s| s.id == "running").unwrap();
        assert_eq!(running.transitions.len(), 2);
        // Own first
        assert_eq!(running.transitions[0].event.as_deref(), Some("stop"));
        assert_eq!(running.transitions[0].defined_in.as_str(), "running");
        // Inherited second
        assert_eq!(running.transitions[1].event.as_deref(), Some("reset"));
        assert_eq!(running.transitions[1].defined_in.as_str(), "parent");
    }

    #[test]
    fn resolve_initial_child() {
        let chart = Statechart::new(
            "idle",
            vec![State::compound(
                "wrapper",
                "idle",
                vec![State::atomic("idle"), State::atomic("active")],
            )],
        );

        let resolved = resolve(&chart);
        let wrapper = resolved.states.iter().find(|s| s.id == "wrapper").unwrap();
        assert_eq!(wrapper.initial_child.as_deref(), Some("idle"));
        assert_eq!(wrapper.children.len(), 2);
    }

    #[test]
    fn resolve_parallel_children() {
        let chart = Statechart::new(
            "main",
            vec![State::parallel(
                "main",
                vec![State::atomic("region_a"), State::atomic("region_b")],
            )],
        );

        let resolved = resolve(&chart);
        let main = &resolved.states[0];
        assert_eq!(main.kind, StateKind::Parallel);
        assert_eq!(main.children.len(), 2);
        assert!(main.initial_child.is_none()); // parallel, not compound
    }

    #[test]
    fn resolve_event_catalog() {
        let chart = simple_chart();
        let resolved = resolve(&chart);

        assert_eq!(resolved.events, vec!["go", "next"]);
    }

    #[test]
    fn resolve_defined_in_tag() {
        let chart = Statechart::new(
            "child",
            vec![{
                let mut outer = State::compound("outer", "child", vec![State::atomic("child")]);
                outer.transitions.push(Transition::new("escape", "child"));
                outer
            }],
        );

        let resolved = resolve(&chart);
        let child = resolved.states.iter().find(|s| s.id == "child").unwrap();
        assert_eq!(child.transitions.len(), 1);
        assert_eq!(child.transitions[0].defined_in.as_str(), "outer");
    }

    #[test]
    fn resolve_depth() {
        let chart = Statechart::new(
            "a",
            vec![State::compound(
                "level0",
                "a",
                vec![State::compound("level1", "a", vec![State::atomic("a")])],
            )],
        );

        let resolved = resolve(&chart);
        let level0 = resolved.states.iter().find(|s| s.id == "level0").unwrap();
        let level1 = resolved.states.iter().find(|s| s.id == "level1").unwrap();
        let a = resolved.states.iter().find(|s| s.id == "a").unwrap();

        assert_eq!(level0.depth, 0);
        assert_eq!(level1.depth, 1);
        assert_eq!(a.depth, 2);
    }
}
