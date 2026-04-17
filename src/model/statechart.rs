use compact_str::CompactString;
use serde::{Deserialize, Serialize};

use super::datamodel::DataModel;
use super::state::State;

/// A complete statechart document, corresponding to the `<scxml>` root element.
///
/// This is the top-level type produced by parsing and consumed by validation,
/// export, and compilation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
pub struct Statechart {
    /// Optional name for the statechart (`<scxml name="...">`).
    pub name: Option<CompactString>,

    /// The initial state id (`<scxml initial="...">`).
    pub initial: CompactString,

    /// Top-level child states.
    pub states: Vec<State>,

    /// Data model declarations.
    #[serde(default)]
    pub datamodel: DataModel,

    /// Binding type: "early" (default) or "late".
    #[serde(default)]
    pub binding: Binding,

    /// SCXML version (always "1.0" per W3C spec).
    #[serde(default = "default_version")]
    pub version: CompactString,

    /// XML namespace (informational, for roundtrip fidelity).
    #[serde(default = "default_xmlns")]
    pub xmlns: CompactString,

    /// Application-defined version for lifecycle tracking (not part of W3C SCXML).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_version: Option<CompactString>,
}

fn default_version() -> CompactString {
    CompactString::const_new("1.0")
}

fn default_xmlns() -> CompactString {
    CompactString::const_new("http://www.w3.org/2005/07/scxml")
}

/// Data binding mode per W3C SCXML §3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum Binding {
    /// All data items are instantiated at document load time (default per W3C §5.3).
    #[default]
    Early,
    /// Data items are instantiated when the enclosing state is first entered.
    Late,
}

impl Statechart {
    /// Create a new statechart with the given initial state and child states.
    pub fn new(initial: impl Into<CompactString>, states: Vec<State>) -> Self {
        Self {
            name: None,
            initial: initial.into(),
            states,
            datamodel: DataModel::default(),
            binding: Binding::default(),
            version: default_version(),
            xmlns: default_xmlns(),
            definition_version: None,
        }
    }

    /// Set the name of this statechart.
    pub fn with_name(mut self, name: impl Into<CompactString>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the data model.
    pub fn with_datamodel(mut self, datamodel: DataModel) -> Self {
        self.datamodel = datamodel;
        self
    }

    /// Iterate over all states in the chart (recursive depth-first).
    pub fn iter_all_states(&self) -> impl Iterator<Item = &State> {
        self.states.iter().flat_map(|s| s.iter_all())
    }

    /// Collect all state ids in the chart.
    pub fn all_state_ids(&self) -> Vec<&CompactString> {
        // Pre-size: most charts have < 100 states; avoid repeated doubling.
        let estimate = self.states.len() * 4; // rough heuristic for nesting
        let mut ids = Vec::with_capacity(estimate);
        ids.extend(self.iter_all_states().map(|s| &s.id));
        ids
    }

    /// Find a state by id (recursive search).
    pub fn find_state(&self, id: &str) -> Option<&State> {
        self.iter_all_states().find(|s| s.id.as_str() == id)
    }
}

impl std::fmt::Display for Statechart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.name.as_deref().unwrap_or("unnamed");
        let mut total = 0usize;
        let mut transitions = 0usize;
        for state in self.iter_all_states() {
            total += 1;
            transitions += state.transitions.len();
        }
        write!(
            f,
            "Statechart({name}, {total} states, {transitions} transitions, initial={initial})",
            initial = self.initial,
        )
    }
}
