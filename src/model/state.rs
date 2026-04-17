use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use super::action::Action;
use super::transition::Transition;

/// A single state in a statechart. Maps to `<state>`, `<parallel>`, `<final>`,
/// or `<history>` in W3C SCXML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[cfg_attr(
    feature = "rkyv",
    rkyv(
        serialize_bounds(__S: rkyv::ser::Writer + rkyv::ser::Allocator),
        deserialize_bounds(__D::Error: rkyv::rancor::Source),
        bytecheck(bounds(
            __C: rkyv::validation::ArchiveContext,
            <__C as rkyv::rancor::Fallible>::Error: rkyv::rancor::Source,
        )),
    )
)]
#[non_exhaustive]
pub struct State {
    /// Unique identifier within the statechart (W3C `id` attribute).
    pub id: CompactString,

    /// What kind of state this is.
    pub kind: StateKind,

    /// Outgoing transitions from this state.
    #[serde(default)]
    pub transitions: Vec<Transition>,

    /// Actions executed on entry.
    #[serde(default)]
    pub on_entry: Vec<Action>,

    /// Actions executed on exit.
    #[serde(default)]
    pub on_exit: Vec<Action>,

    /// Child states (non-empty for Compound and Parallel).
    #[serde(default)]
    #[cfg_attr(feature = "rkyv", rkyv(omit_bounds))]
    pub children: Vec<State>,

    /// Initial child state id (for Compound states).
    #[serde(default)]
    pub initial: Option<CompactString>,
}

/// The kind of state, following W3C SCXML semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum StateKind {
    /// Leaf state with no children.
    Atomic,
    /// Has children; exactly one child is active at a time.
    Compound,
    /// All children are active simultaneously (orthogonal regions).
    Parallel,
    /// Terminal state. No outgoing transitions.
    Final,
    /// Pseudo-state that remembers the last active child of its parent.
    History(HistoryKind),
}

/// Shallow vs deep history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum HistoryKind {
    /// Remembers only the immediate child.
    Shallow,
    /// Remembers the full descendant configuration.
    Deep,
}

impl State {
    /// Create a new atomic state with the given id.
    pub fn atomic(id: impl Into<CompactString>) -> Self {
        Self {
            id: id.into(),
            kind: StateKind::Atomic,
            transitions: Vec::new(),
            on_entry: Vec::new(),
            on_exit: Vec::new(),
            children: Vec::new(),
            initial: None,
        }
    }

    /// Create a new compound state.
    pub fn compound(
        id: impl Into<CompactString>,
        initial: impl Into<CompactString>,
        children: Vec<State>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: StateKind::Compound,
            transitions: Vec::new(),
            on_entry: Vec::new(),
            on_exit: Vec::new(),
            children,
            initial: Some(initial.into()),
        }
    }

    /// Create a new parallel state.
    pub fn parallel(id: impl Into<CompactString>, children: Vec<State>) -> Self {
        Self {
            id: id.into(),
            kind: StateKind::Parallel,
            transitions: Vec::new(),
            on_entry: Vec::new(),
            on_exit: Vec::new(),
            children,
            initial: None,
        }
    }

    /// Create a new final state.
    pub fn final_state(id: impl Into<CompactString>) -> Self {
        Self {
            id: id.into(),
            kind: StateKind::Final,
            transitions: Vec::new(),
            on_entry: Vec::new(),
            on_exit: Vec::new(),
            children: Vec::new(),
            initial: None,
        }
    }

    /// Create a history pseudo-state.
    pub fn history(id: impl Into<CompactString>, kind: HistoryKind) -> Self {
        Self {
            id: id.into(),
            kind: StateKind::History(kind),
            transitions: Vec::new(),
            on_entry: Vec::new(),
            on_exit: Vec::new(),
            children: Vec::new(),
            initial: None,
        }
    }

    /// Returns `true` if this state is a leaf (Atomic or Final).
    pub fn is_leaf(&self) -> bool {
        matches!(self.kind, StateKind::Atomic | StateKind::Final)
    }

    /// Returns `true` if this state has children.
    pub fn is_composite(&self) -> bool {
        matches!(self.kind, StateKind::Compound | StateKind::Parallel)
    }

    /// Recursively iterate over this state and all descendants.
    pub fn iter_all(&self) -> impl Iterator<Item = &State> {
        StateIter { stack: vec![self] }
    }
}

/// Depth-first iterator over a state and all its descendants.
struct StateIter<'a> {
    stack: Vec<&'a State>,
}

impl<'a> Iterator for StateIter<'a> {
    type Item = &'a State;

    fn next(&mut self) -> Option<Self::Item> {
        let state = self.stack.pop()?;
        // Push children in reverse so leftmost is visited first.
        for child in state.children.iter().rev() {
            self.stack.push(child);
        }
        Some(state)
    }
}
