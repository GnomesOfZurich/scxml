//! Convert XState v5 machine JSON into a [`Statechart`].

use std::collections::BTreeMap;

use compact_str::CompactString;

use crate::error::{Result, ScxmlError};
use crate::model::action::{Action, ActionKind};
use crate::model::datamodel::{DataItem, DataModel};
use crate::model::state::{HistoryKind, State, StateKind};
use crate::model::statechart::Statechart;
use crate::model::transition::{Transition, TransitionType};

use super::types::*;

/// Parse an XState v5 machine JSON string into a [`Statechart`].
///
/// ```rust
/// use scxml::xstate::parse_xstate;
///
/// let json = r#"{
///   "id": "traffic",
///   "initial": "green",
///   "states": {
///     "green":  { "on": { "TIMER": "yellow" } },
///     "yellow": { "on": { "TIMER": "red" } },
///     "red":    { "on": { "TIMER": "green" } }
///   }
/// }"#;
///
/// let chart = parse_xstate(json).unwrap();
/// assert_eq!(chart.initial.as_str(), "green");
/// assert_eq!(chart.states.len(), 3);
/// ```
pub fn parse_xstate(json: &str) -> Result<Statechart> {
    let machine: XStateMachine =
        serde_json::from_str(json).map_err(|e| ScxmlError::XState(e.to_string()))?;
    convert_machine(machine)
}

/// Parse an XState v5 machine from a [`serde_json::Value`].
pub fn parse_xstate_value(value: serde_json::Value) -> Result<Statechart> {
    let machine: XStateMachine =
        serde_json::from_value(value).map_err(|e| ScxmlError::XState(e.to_string()))?;
    convert_machine(machine)
}

fn convert_machine(machine: XStateMachine) -> Result<Statechart> {
    let initial = machine
        .initial
        .ok_or_else(|| ScxmlError::XState("root machine must have an \"initial\" field".into()))?;

    let states = convert_states(&machine.states)?;

    let datamodel = match machine.context {
        Some(serde_json::Value::Object(map)) => {
            let items = map
                .into_iter()
                .map(|(k, v)| {
                    let expr = match v {
                        serde_json::Value::Null => None,
                        other => Some(CompactString::from(other.to_string())),
                    };
                    DataItem {
                        id: CompactString::from(k),
                        expr,
                        src: None,
                    }
                })
                .collect();
            DataModel { items }
        }
        _ => DataModel::default(),
    };

    let mut chart = Statechart::new(initial, states).with_datamodel(datamodel);
    if let Some(id) = machine.id {
        chart = chart.with_name(id);
    }
    Ok(chart)
}

fn convert_states(states: &BTreeMap<String, XStateNode>) -> Result<Vec<State>> {
    states
        .iter()
        .map(|(name, node)| convert_state_node(name, node))
        .collect()
}

fn convert_state_node(name: &str, node: &XStateNode) -> Result<State> {
    let (kind, children, initial) = determine_state_kind(name, node)?;

    let mut transitions = Vec::new();

    // Event-triggered transitions from `on`.
    for (event, tv) in &node.on {
        collect_transitions(Some(event), tv, None, &mut transitions);
    }

    // Eventless transitions from `always`.
    if let Some(tv) = &node.always {
        collect_transitions(None, tv, None, &mut transitions);
    }

    // Delayed transitions from `after`.
    for (delay, tv) in &node.after {
        let delay_str = normalize_delay(delay);
        collect_transitions(None, tv, Some(&delay_str), &mut transitions);
    }

    let on_entry = convert_actions(&node.entry);
    let on_exit = convert_actions(&node.exit);

    Ok(State {
        id: CompactString::from(name),
        kind,
        transitions,
        on_entry,
        on_exit,
        children,
        initial: initial.map(CompactString::from),
    })
}

fn determine_state_kind(
    name: &str,
    node: &XStateNode,
) -> Result<(StateKind, Vec<State>, Option<String>)> {
    match node.state_type.as_deref() {
        Some("final") => Ok((StateKind::Final, Vec::new(), None)),
        Some("parallel") => {
            let children = convert_states(&node.states)?;
            Ok((StateKind::Parallel, children, None))
        }
        Some("history") => {
            let hk = match node.history.as_deref() {
                Some("deep") => HistoryKind::Deep,
                _ => HistoryKind::Shallow,
            };
            Ok((StateKind::History(hk), Vec::new(), None))
        }
        Some(other) => Err(ScxmlError::XState(format!(
            "unknown state type \"{other}\" on state \"{name}\""
        ))),
        None => {
            if node.states.is_empty() {
                // Atomic state.
                Ok((StateKind::Atomic, Vec::new(), None))
            } else {
                // Compound state: must have initial.
                let children = convert_states(&node.states)?;
                let initial = node.initial.clone().or_else(|| {
                    // XState defaults to first defined child.
                    node.states.keys().next().cloned()
                });
                Ok((StateKind::Compound, children, initial))
            }
        }
    }
}

fn collect_transitions(
    event: Option<&str>,
    tv: &XTransitionValue,
    delay: Option<&str>,
    out: &mut Vec<Transition>,
) {
    match tv {
        XTransitionValue::Simple(target) => {
            out.push(make_transition(event, Some(target), None, &[], delay));
        }
        XTransitionValue::Object(obj) => {
            out.push(convert_transition_object(event, obj, delay));
        }
        XTransitionValue::Array(items) => {
            for item in items {
                match item {
                    XTransitionItem::Simple(target) => {
                        out.push(make_transition(event, Some(target), None, &[], delay));
                    }
                    XTransitionItem::Object(obj) => {
                        out.push(convert_transition_object(event, obj, delay));
                    }
                }
            }
        }
    }
}

fn convert_transition_object(
    event: Option<&str>,
    obj: &XTransitionObject,
    delay: Option<&str>,
) -> Transition {
    let guard = obj.guard.as_ref().map(|g| match g {
        XGuardValue::Simple(s) => s.as_str(),
        XGuardValue::Object(o) => o.guard_type.as_str(),
    });
    make_transition(event, obj.target.as_deref(), guard, &obj.actions, delay)
}

fn make_transition(
    event: Option<&str>,
    target: Option<&str>,
    guard: Option<&str>,
    actions: &[XActionValue],
    delay: Option<&str>,
) -> Transition {
    Transition {
        event: event.map(CompactString::from),
        guard: guard.map(CompactString::from),
        targets: target.into_iter().map(CompactString::from).collect(),
        transition_type: TransitionType::External,
        actions: convert_actions(actions),
        delay: delay.map(CompactString::from),
        quorum: None,
    }
}

fn convert_actions(actions: &[XActionValue]) -> Vec<Action> {
    actions
        .iter()
        .map(|a| match a {
            XActionValue::Simple(name) => Action::custom(name.as_str()),
            XActionValue::Object(obj) => Action {
                kind: ActionKind::Custom {
                    name: CompactString::from(obj.action_type.as_str()),
                    params: Vec::new(),
                },
            },
        })
        .collect()
}

/// Normalize a delay key to an ISO 8601 duration if it looks like milliseconds.
/// XState allows `"1000"` (ms) or `"PT1S"` (ISO 8601). We store ISO 8601.
fn normalize_delay(delay: &str) -> String {
    if let Ok(ms) = delay.parse::<u64>() {
        ms_to_iso8601(ms)
    } else {
        delay.to_string()
    }
}

fn ms_to_iso8601(ms: u64) -> String {
    if ms == 0 {
        return "PT0S".to_string();
    }
    let total_secs = ms / 1000;
    let remainder_ms = ms % 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    let mut s = String::from("PT");
    if hours > 0 {
        s.push_str(&format!("{hours}H"));
    }
    if minutes > 0 {
        s.push_str(&format!("{minutes}M"));
    }
    if seconds > 0 || remainder_ms > 0 {
        if remainder_ms > 0 {
            s.push_str(&format!("{seconds}.{remainder_ms:03}S"));
        } else {
            s.push_str(&format!("{seconds}S"));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ms_to_iso8601_conversions() {
        assert_eq!(ms_to_iso8601(0), "PT0S");
        assert_eq!(ms_to_iso8601(1000), "PT1S");
        assert_eq!(ms_to_iso8601(60000), "PT1M");
        assert_eq!(ms_to_iso8601(3600000), "PT1H");
        assert_eq!(ms_to_iso8601(3661000), "PT1H1M1S");
        assert_eq!(ms_to_iso8601(1500), "PT1.500S");
        assert_eq!(ms_to_iso8601(500), "PT0.500S");
    }

    #[test]
    fn parse_simple_xstate() {
        let json = r#"{
            "id": "light",
            "initial": "green",
            "states": {
                "green":  { "on": { "TIMER": "yellow" } },
                "yellow": { "on": { "TIMER": "red" } },
                "red":    { "on": { "TIMER": "green" } }
            }
        }"#;
        let chart = parse_xstate(json).unwrap();
        assert_eq!(chart.name.as_deref(), Some("light"));
        assert_eq!(chart.initial.as_str(), "green");
        assert_eq!(chart.states.len(), 3);
    }

    #[test]
    fn parse_with_guards_and_actions() {
        let json = r#"{
            "id": "auth",
            "initial": "idle",
            "states": {
                "idle": {
                    "on": {
                        "LOGIN": {
                            "target": "checking",
                            "guard": "hasCredentials",
                            "actions": ["validateInput"]
                        }
                    }
                },
                "checking": {
                    "on": {
                        "SUCCESS": "done",
                        "FAIL": "idle"
                    },
                    "entry": ["startAuth"],
                    "exit": ["clearAuth"]
                },
                "done": { "type": "final" }
            }
        }"#;
        let chart = parse_xstate(json).unwrap();
        assert_eq!(chart.states.len(), 3);

        let idle = chart.find_state("idle").unwrap();
        assert_eq!(idle.transitions.len(), 1);
        assert_eq!(idle.transitions[0].guard.as_deref(), Some("hasCredentials"));
        assert_eq!(idle.transitions[0].actions.len(), 1);

        let checking = chart.find_state("checking").unwrap();
        assert_eq!(checking.on_entry.len(), 1);
        assert_eq!(checking.on_exit.len(), 1);

        let done = chart.find_state("done").unwrap();
        assert_eq!(done.kind, StateKind::Final);
    }

    #[test]
    fn parse_parallel_state() {
        let json = r#"{
            "id": "upload",
            "initial": "processing",
            "states": {
                "processing": {
                    "type": "parallel",
                    "states": {
                        "upload": { "on": { "DONE": { "target": "complete" } } },
                        "dialog": { "on": { "CLOSE": { "target": "hidden" } } }
                    }
                }
            }
        }"#;
        let chart = parse_xstate(json).unwrap();
        let proc = chart.find_state("processing").unwrap();
        assert_eq!(proc.kind, StateKind::Parallel);
        assert_eq!(proc.children.len(), 2);
    }

    #[test]
    fn parse_compound_with_initial() {
        let json = r#"{
            "id": "player",
            "initial": "playing",
            "states": {
                "playing": {
                    "initial": "normal",
                    "states": {
                        "normal": { "on": { "FF": "fast" } },
                        "fast": { "on": { "NORMAL": "normal" } }
                    }
                }
            }
        }"#;
        let chart = parse_xstate(json).unwrap();
        let playing = chart.find_state("playing").unwrap();
        assert_eq!(playing.kind, StateKind::Compound);
        assert_eq!(playing.initial.as_deref(), Some("normal"));
        assert_eq!(playing.children.len(), 2);
    }

    #[test]
    fn parse_with_context() {
        let json = r#"{
            "id": "counter",
            "initial": "active",
            "context": { "count": 0, "label": "hello" },
            "states": {
                "active": { "on": { "INC": "active" } }
            }
        }"#;
        let chart = parse_xstate(json).unwrap();
        assert_eq!(chart.datamodel.items.len(), 2);
    }

    #[test]
    fn parse_delayed_transition() {
        let json = r#"{
            "id": "timeout",
            "initial": "waiting",
            "states": {
                "waiting": {
                    "after": { "3000": "expired" }
                },
                "expired": { "type": "final" }
            }
        }"#;
        let chart = parse_xstate(json).unwrap();
        let waiting = chart.find_state("waiting").unwrap();
        assert_eq!(waiting.transitions.len(), 1);
        assert_eq!(waiting.transitions[0].delay.as_deref(), Some("PT3S"));
        assert!(waiting.transitions[0].event.is_none());
    }

    #[test]
    fn parse_always_transition() {
        let json = r#"{
            "id": "router",
            "initial": "check",
            "states": {
                "check": {
                    "always": [
                        { "target": "allowed", "guard": "isAdmin" },
                        { "target": "denied" }
                    ]
                },
                "allowed": { "type": "final" },
                "denied": { "type": "final" }
            }
        }"#;
        let chart = parse_xstate(json).unwrap();
        let check = chart.find_state("check").unwrap();
        assert_eq!(check.transitions.len(), 2);
        assert!(check.transitions[0].event.is_none());
        assert_eq!(check.transitions[0].guard.as_deref(), Some("isAdmin"));
    }

    #[test]
    fn parse_multiple_transitions_same_event() {
        let json = r#"{
            "id": "router",
            "initial": "idle",
            "states": {
                "idle": {
                    "on": {
                        "SUBMIT": [
                            { "target": "fast", "guard": "isPriority" },
                            { "target": "normal" }
                        ]
                    }
                },
                "fast": { "type": "final" },
                "normal": { "type": "final" }
            }
        }"#;
        let chart = parse_xstate(json).unwrap();
        let idle = chart.find_state("idle").unwrap();
        assert_eq!(idle.transitions.len(), 2);
        assert_eq!(idle.transitions[0].guard.as_deref(), Some("isPriority"));
        assert!(idle.transitions[1].guard.is_none());
    }

    #[test]
    fn parse_history_state() {
        let json = r#"{
            "id": "editor",
            "initial": "editing",
            "states": {
                "editing": {
                    "initial": "idle",
                    "states": {
                        "idle": {},
                        "hist": { "type": "history", "history": "deep" }
                    }
                }
            }
        }"#;
        let chart = parse_xstate(json).unwrap();
        let hist = chart.find_state("hist").unwrap();
        assert_eq!(hist.kind, StateKind::History(HistoryKind::Deep));
    }
}
