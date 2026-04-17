//! Convert a [`Statechart`] into XState v5 machine JSON.

use std::collections::BTreeMap;

use crate::model::action::{Action, ActionKind};
use crate::model::state::{HistoryKind, State, StateKind};
use crate::model::statechart::Statechart;
use crate::model::transition::Transition;

use super::types::*;

/// Export a [`Statechart`] to an XState v5 JSON string (pretty-printed).
///
/// ```rust
/// use scxml::xstate::{parse_xstate, to_xstate};
///
/// let json = r#"{
///   "id": "light",
///   "initial": "green",
///   "states": {
///     "green":  { "on": { "TIMER": "yellow" } },
///     "yellow": { "on": { "TIMER": "red" } },
///     "red":    { "on": { "TIMER": "green" } }
///   }
/// }"#;
///
/// let chart = parse_xstate(json).unwrap();
/// let output = to_xstate(&chart).unwrap();
/// assert!(output.contains("\"green\""));
/// ```
pub fn to_xstate(chart: &Statechart) -> Result<String, serde_json::Error> {
    let machine = to_xstate_value(chart);
    serde_json::to_string_pretty(&machine)
}

/// Export a [`Statechart`] to an XState v5 machine value.
pub fn to_xstate_value(chart: &Statechart) -> XStateMachine {
    let limit = crate::max_depth();
    let states = export_states(&chart.states, 0, limit);

    let context = if chart.datamodel.items.is_empty() {
        None
    } else {
        let map: serde_json::Map<String, serde_json::Value> = chart
            .datamodel
            .items
            .iter()
            .map(|item| {
                let value = item
                    .expr
                    .as_ref()
                    .and_then(|e| serde_json::from_str(e.as_str()).ok())
                    .unwrap_or(serde_json::Value::Null);
                (item.id.to_string(), value)
            })
            .collect();
        Some(serde_json::Value::Object(map))
    };

    XStateMachine {
        id: chart.name.as_ref().map(|n| n.to_string()),
        initial: Some(chart.initial.to_string()),
        state_type: None,
        states,
        on: BTreeMap::new(),
        always: None,
        after: BTreeMap::new(),
        entry: Vec::new(),
        exit: Vec::new(),
        context,
        history: None,
        description: None,
    }
}

fn export_states(states: &[State], depth: usize, limit: usize) -> BTreeMap<String, XStateNode> {
    if depth > limit {
        return BTreeMap::new();
    }
    states
        .iter()
        .map(|s| (s.id.to_string(), export_state_node(s, depth, limit)))
        .collect()
}

fn export_state_node(state: &State, depth: usize, limit: usize) -> XStateNode {
    let state_type = match state.kind {
        StateKind::Final => Some("final".to_string()),
        StateKind::Parallel => Some("parallel".to_string()),
        StateKind::History(_) => Some("history".to_string()),
        _ => None,
    };

    let history = match state.kind {
        StateKind::History(HistoryKind::Deep) => Some("deep".to_string()),
        StateKind::History(HistoryKind::Shallow) => Some("shallow".to_string()),
        _ => None,
    };

    let children = if state.children.is_empty() {
        BTreeMap::new()
    } else {
        export_states(&state.children, depth + 1, limit)
    };

    let initial = state.initial.as_ref().map(|i| i.to_string());

    // Partition transitions into event-based, always, and after.
    let mut on: BTreeMap<String, Vec<XTransitionItem>> = BTreeMap::new();
    let mut always_items: Vec<XTransitionItem> = Vec::new();
    let mut after: BTreeMap<String, Vec<XTransitionItem>> = BTreeMap::new();

    for t in &state.transitions {
        let item = export_transition_item(t);
        if let Some(delay) = &t.delay {
            after.entry(delay.to_string()).or_default().push(item);
        } else if t.event.is_none() {
            always_items.push(item);
        } else if let Some(event) = &t.event {
            on.entry(event.to_string()).or_default().push(item);
        }
    }

    let on = on
        .into_iter()
        .map(|(k, v)| (k, simplify_transition_value(v)))
        .collect();

    let always = if always_items.is_empty() {
        None
    } else {
        Some(simplify_transition_value(always_items))
    };

    let after = after
        .into_iter()
        .map(|(k, v)| (k, simplify_transition_value(v)))
        .collect();

    XStateNode {
        initial,
        state_type,
        states: children,
        on,
        always,
        after,
        entry: export_actions(&state.on_entry),
        exit: export_actions(&state.on_exit),
        history,
        description: None,
    }
}

fn export_transition_item(t: &Transition) -> XTransitionItem {
    let has_guard = t.guard.is_some();
    let has_actions = !t.actions.is_empty();

    if !has_guard && !has_actions {
        // Simple string transition.
        if let Some(target) = t.targets.first() {
            return XTransitionItem::Simple(target.to_string());
        }
    }

    XTransitionItem::Object(XTransitionObject {
        target: t.targets.first().map(|t| t.to_string()),
        guard: t.guard.as_ref().map(|g| XGuardValue::Simple(g.to_string())),
        actions: export_actions_to_xaction(&t.actions),
        description: None,
    })
}

/// Simplify a list of transition items into the most compact XState form.
fn simplify_transition_value(items: Vec<XTransitionItem>) -> XTransitionValue {
    if items.len() == 1 {
        match items.into_iter().next().unwrap() {
            XTransitionItem::Simple(s) => XTransitionValue::Simple(s),
            XTransitionItem::Object(o) => XTransitionValue::Object(o),
        }
    } else {
        XTransitionValue::Array(items)
    }
}

fn export_actions(actions: &[Action]) -> Vec<XActionValue> {
    actions.iter().map(action_to_xaction).collect()
}

fn export_actions_to_xaction(actions: &[Action]) -> Vec<XActionValue> {
    actions.iter().map(action_to_xaction).collect()
}

fn action_to_xaction(action: &Action) -> XActionValue {
    match &action.kind {
        ActionKind::Raise { event } => XActionValue::Object(XActionObject {
            action_type: format!("raise.{event}"),
            params: None,
        }),
        ActionKind::Send {
            event,
            target,
            delay,
        } => {
            let mut params = serde_json::Map::new();
            params.insert("event".into(), serde_json::Value::String(event.to_string()));
            if let Some(t) = target {
                params.insert("target".into(), serde_json::Value::String(t.to_string()));
            }
            if let Some(d) = delay {
                params.insert("delay".into(), serde_json::Value::String(d.to_string()));
            }
            XActionValue::Object(XActionObject {
                action_type: "send".into(),
                params: Some(serde_json::Value::Object(params)),
            })
        }
        ActionKind::Assign { location, expr } => {
            let mut params = serde_json::Map::new();
            params.insert(
                "location".into(),
                serde_json::Value::String(location.to_string()),
            );
            params.insert("expr".into(), serde_json::Value::String(expr.to_string()));
            XActionValue::Object(XActionObject {
                action_type: "assign".into(),
                params: Some(serde_json::Value::Object(params)),
            })
        }
        ActionKind::Log { label, expr } => {
            let mut params = serde_json::Map::new();
            if let Some(l) = label {
                params.insert("label".into(), serde_json::Value::String(l.to_string()));
            }
            if let Some(e) = expr {
                params.insert("expr".into(), serde_json::Value::String(e.to_string()));
            }
            XActionValue::Object(XActionObject {
                action_type: "log".into(),
                params: if params.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Object(params))
                },
            })
        }
        ActionKind::Cancel { sendid } => XActionValue::Object(XActionObject {
            action_type: "cancel".into(),
            params: Some(serde_json::json!({ "sendid": sendid.to_string() })),
        }),
        ActionKind::If { .. } => XActionValue::Object(XActionObject {
            action_type: "if".into(),
            params: None,
        }),
        ActionKind::Foreach { array, item, .. } => {
            let mut params = serde_json::Map::new();
            params.insert("array".into(), serde_json::Value::String(array.to_string()));
            params.insert("item".into(), serde_json::Value::String(item.to_string()));
            XActionValue::Object(XActionObject {
                action_type: "foreach".into(),
                params: Some(serde_json::Value::Object(params)),
            })
        }
        ActionKind::Script { content } => XActionValue::Object(XActionObject {
            action_type: "script".into(),
            params: Some(serde_json::json!({ "content": content.to_string() })),
        }),
        ActionKind::Invoke {
            invoke_type,
            src,
            id,
        } => {
            let mut params = serde_json::Map::new();
            if let Some(t) = invoke_type {
                params.insert("type".into(), serde_json::Value::String(t.to_string()));
            }
            if let Some(s) = src {
                params.insert("src".into(), serde_json::Value::String(s.to_string()));
            }
            if let Some(i) = id {
                params.insert("id".into(), serde_json::Value::String(i.to_string()));
            }
            XActionValue::Object(XActionObject {
                action_type: "invoke".into(),
                params: if params.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Object(params))
                },
            })
        }
        ActionKind::Custom { name, .. } => XActionValue::Simple(name.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::state::State;
    use crate::model::statechart::Statechart;
    use crate::model::transition::Transition;

    #[test]
    fn export_simple_chart() {
        let chart = Statechart::new(
            "green",
            vec![
                State {
                    id: "green".into(),
                    kind: StateKind::Atomic,
                    transitions: vec![Transition::new("TIMER", "yellow")],
                    on_entry: vec![],
                    on_exit: vec![],
                    children: vec![],
                    initial: None,
                },
                State {
                    id: "yellow".into(),
                    kind: StateKind::Atomic,
                    transitions: vec![Transition::new("TIMER", "red")],
                    on_entry: vec![],
                    on_exit: vec![],
                    children: vec![],
                    initial: None,
                },
                State::final_state("red"),
            ],
        )
        .with_name("light");

        let json = to_xstate(&chart).unwrap();
        assert!(json.contains("\"id\": \"light\""));
        assert!(json.contains("\"initial\": \"green\""));
        assert!(json.contains("\"TIMER\""));
    }

    #[test]
    fn export_final_state() {
        let chart = Statechart::new(
            "a",
            vec![
                State {
                    id: "a".into(),
                    kind: StateKind::Atomic,
                    transitions: vec![Transition::new("GO", "b")],
                    on_entry: vec![],
                    on_exit: vec![],
                    children: vec![],
                    initial: None,
                },
                State::final_state("b"),
            ],
        );
        let json = to_xstate(&chart).unwrap();
        assert!(json.contains("\"type\": \"final\""));
    }

    #[test]
    fn export_with_guard() {
        let chart = Statechart::new(
            "idle",
            vec![
                State {
                    id: "idle".into(),
                    kind: StateKind::Atomic,
                    transitions: vec![Transition::new("GO", "done").with_guard("isReady")],
                    on_entry: vec![],
                    on_exit: vec![],
                    children: vec![],
                    initial: None,
                },
                State::final_state("done"),
            ],
        );
        let json = to_xstate(&chart).unwrap();
        assert!(json.contains("\"guard\": \"isReady\""));
    }
}
