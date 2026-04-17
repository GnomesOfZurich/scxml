//! Intermediate serde types for XState v5 machine JSON format.
//!
//! XState's JSON is flexible: transitions can be strings, objects, or arrays.
//! These types capture that flexibility, then [`super::import`] and
//! [`super::export`] convert to/from our canonical [`Statechart`] model.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Root XState machine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XStateMachine {
    /// Machine identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Initial child state key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial: Option<String>,

    /// State type: "parallel", "final", "history", or absent (atomic/compound).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub state_type: Option<String>,

    /// Child states keyed by name.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub states: BTreeMap<String, XStateNode>,

    /// Event-keyed transitions.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub on: BTreeMap<String, XTransitionValue>,

    /// Eventless (always) transitions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub always: Option<XTransitionValue>,

    /// Delayed transitions keyed by duration string or milliseconds.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub after: BTreeMap<String, XTransitionValue>,

    /// Entry actions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entry: Vec<XActionValue>,

    /// Exit actions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exit: Vec<XActionValue>,

    /// Context (maps to datamodel).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,

    /// History kind for history states.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<String>,

    /// Description (informational).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A single state node in XState JSON. Same structure as the root machine
/// but without the top-level `id` (state name comes from the parent map key).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XStateNode {
    /// Initial child state key (for compound states).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial: Option<String>,

    /// State type: "parallel", "final", "history", or absent.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub state_type: Option<String>,

    /// Child states.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub states: BTreeMap<String, XStateNode>,

    /// Event-keyed transitions.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub on: BTreeMap<String, XTransitionValue>,

    /// Eventless transitions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub always: Option<XTransitionValue>,

    /// Delayed transitions.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub after: BTreeMap<String, XTransitionValue>,

    /// Entry actions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entry: Vec<XActionValue>,

    /// Exit actions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exit: Vec<XActionValue>,

    /// History kind for history states: "shallow" or "deep".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<String>,

    /// Description (informational).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// XState transitions are polymorphic: a string, an object, or an array.
///
/// - `"target"` → single unconditional transition
/// - `{ target, guard, actions }` → single transition with details
/// - `[{ target, guard }, ...]` → multiple guarded transitions for same event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum XTransitionValue {
    /// Simple target string: `"nextState"`.
    Simple(String),
    /// Single transition object.
    Object(XTransitionObject),
    /// Array of transition objects (multiple transitions for one event).
    Array(Vec<XTransitionItem>),
}

/// A single transition item within an array; can itself be a string or object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum XTransitionItem {
    /// Simple target string.
    Simple(String),
    /// Full transition object.
    Object(XTransitionObject),
}

/// A fully specified transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XTransitionObject {
    /// Target state key. If absent, self-transition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    /// Guard name or guard object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<XGuardValue>,

    /// Actions to execute during the transition.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<XActionValue>,

    /// Description (informational).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Guard can be a simple string or a typed object `{ type: "guardName" }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum XGuardValue {
    /// Simple guard name string.
    Simple(String),
    /// Typed guard object.
    Object(XGuardObject),
}

/// Typed guard: `{ type: "guardName", params: { ... } }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XGuardObject {
    /// Guard type/name.
    #[serde(rename = "type")]
    pub guard_type: String,
}

/// Action can be a string or a typed object.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum XActionValue {
    /// Simple action name string.
    Simple(String),
    /// Typed action object.
    Object(XActionObject),
}

/// Typed action: `{ type: "actionName", params: { ... } }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XActionObject {
    /// Action type/name.
    #[serde(rename = "type")]
    pub action_type: String,

    /// Optional parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}
