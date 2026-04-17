//! XState v5 JSON import/export integration tests.

#![cfg(feature = "xstate")]

use scxml::model::state::StateKind;
use scxml::xstate::{parse_xstate, to_xstate};

/// Parse XState JSON, export back, re-parse, and verify structural equivalence.
fn roundtrip(json: &str) {
    let chart1 = parse_xstate(json).unwrap();
    let exported = to_xstate(&chart1).unwrap();
    let chart2 = parse_xstate(&exported).unwrap();

    assert_eq!(chart1.initial, chart2.initial);
    assert_eq!(chart1.states.len(), chart2.states.len());
    assert_eq!(chart1.name, chart2.name);

    // Verify all state ids present.
    let ids1: Vec<_> = chart1.all_state_ids();
    let ids2: Vec<_> = chart2.all_state_ids();
    assert_eq!(ids1.len(), ids2.len());
}

#[test]
fn roundtrip_traffic_light() {
    let json = r#"{
        "id": "traffic",
        "initial": "green",
        "states": {
            "green":  { "on": { "TIMER": "yellow" } },
            "yellow": { "on": { "TIMER": "red" } },
            "red":    { "on": { "TIMER": "green" } }
        }
    }"#;
    roundtrip(json);
}

#[test]
fn roundtrip_with_guards() {
    let json = r#"{
        "id": "auth",
        "initial": "idle",
        "states": {
            "idle": {
                "on": {
                    "LOGIN": {
                        "target": "checking",
                        "guard": "hasCredentials"
                    }
                }
            },
            "checking": {
                "on": {
                    "OK": "done",
                    "FAIL": "idle"
                }
            },
            "done": { "type": "final" }
        }
    }"#;
    roundtrip(json);
}

#[test]
fn roundtrip_compound_states() {
    let json = r#"{
        "id": "player",
        "initial": "stopped",
        "states": {
            "stopped": {
                "on": { "PLAY": "playing" }
            },
            "playing": {
                "initial": "normal",
                "states": {
                    "normal": { "on": { "FF": "fast" } },
                    "fast": { "on": { "NORMAL": "normal" } }
                },
                "on": { "STOP": "stopped" }
            }
        }
    }"#;
    let chart = parse_xstate(json).unwrap();
    let playing = chart.find_state("playing").unwrap();
    assert_eq!(playing.kind, StateKind::Compound);
    assert_eq!(playing.initial.as_deref(), Some("normal"));
    assert_eq!(playing.children.len(), 2);
    roundtrip(json);
}

#[test]
fn roundtrip_parallel() {
    let json = r#"{
        "id": "form",
        "initial": "editing",
        "states": {
            "editing": {
                "type": "parallel",
                "states": {
                    "name": {
                        "initial": "empty",
                        "states": {
                            "empty": { "on": { "TYPE": "filled" } },
                            "filled": { "type": "final" }
                        }
                    },
                    "email": {
                        "initial": "empty",
                        "states": {
                            "empty": { "on": { "TYPE": "filled" } },
                            "filled": { "type": "final" }
                        }
                    }
                }
            }
        }
    }"#;
    let chart = parse_xstate(json).unwrap();
    let editing = chart.find_state("editing").unwrap();
    assert_eq!(editing.kind, StateKind::Parallel);
    roundtrip(json);
}

#[test]
fn roundtrip_context() {
    let json = r#"{
        "id": "counter",
        "initial": "active",
        "context": {
            "count": 0,
            "name": "test"
        },
        "states": {
            "active": {
                "on": { "INC": "active" }
            }
        }
    }"#;
    let chart = parse_xstate(json).unwrap();
    assert_eq!(chart.datamodel.items.len(), 2);
    roundtrip(json);
}

#[test]
fn roundtrip_delayed_transitions() {
    let json = r#"{
        "id": "timeout",
        "initial": "waiting",
        "states": {
            "waiting": {
                "after": {
                    "3000": "expired"
                }
            },
            "expired": { "type": "final" }
        }
    }"#;
    let chart = parse_xstate(json).unwrap();
    let waiting = chart.find_state("waiting").unwrap();
    assert_eq!(waiting.transitions[0].delay.as_deref(), Some("PT3S"));
    // Roundtrip preserves the delay (as ISO 8601).
    let exported = to_xstate(&chart).unwrap();
    assert!(exported.contains("PT3S"));
}

#[test]
fn roundtrip_always_transitions() {
    let json = r#"{
        "id": "check",
        "initial": "gate",
        "states": {
            "gate": {
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
    let gate = chart.find_state("gate").unwrap();
    assert_eq!(gate.transitions.len(), 2);
    assert!(gate.transitions[0].event.is_none());
    roundtrip(json);
}

#[test]
fn roundtrip_multiple_transitions_per_event() {
    let json = r#"{
        "id": "router",
        "initial": "idle",
        "states": {
            "idle": {
                "on": {
                    "ROUTE": [
                        { "target": "admin", "guard": "isAdmin" },
                        { "target": "user", "guard": "isLoggedIn" },
                        { "target": "login" }
                    ]
                }
            },
            "admin": { "type": "final" },
            "user": { "type": "final" },
            "login": { "type": "final" }
        }
    }"#;
    let chart = parse_xstate(json).unwrap();
    let idle = chart.find_state("idle").unwrap();
    assert_eq!(idle.transitions.len(), 3);
    roundtrip(json);
}

#[test]
fn roundtrip_entry_exit_actions() {
    let json = r#"{
        "id": "modal",
        "initial": "closed",
        "states": {
            "closed": {
                "on": { "OPEN": "open" }
            },
            "open": {
                "entry": ["focusTrap", "announceOpen"],
                "exit": ["releaseFocus"],
                "on": { "CLOSE": "closed" }
            }
        }
    }"#;
    let chart = parse_xstate(json).unwrap();
    let open = chart.find_state("open").unwrap();
    assert_eq!(open.on_entry.len(), 2);
    assert_eq!(open.on_exit.len(), 1);
    roundtrip(json);
}

#[test]
fn xstate_to_scxml_xml_roundtrip() {
    // Parse XState JSON, export to SCXML XML, re-parse, verify.
    let json = r#"{
        "id": "simple",
        "initial": "a",
        "states": {
            "a": { "on": { "GO": "b" } },
            "b": { "type": "final" }
        }
    }"#;
    let chart = parse_xstate(json).unwrap();

    // Export to SCXML XML.
    let xml = scxml::export::xml::to_xml(&chart);
    assert!(xml.contains("<scxml"));
    assert!(xml.contains("initial=\"a\""));

    // Re-parse from XML.
    let chart2 = scxml::parse_xml(&xml).unwrap();
    assert_eq!(chart2.initial.as_str(), "a");
    assert_eq!(chart2.states.len(), 2);
}

#[test]
fn parse_xstate_missing_initial() {
    let json = r#"{
        "id": "bad",
        "states": {
            "a": {}
        }
    }"#;
    let result = parse_xstate(json);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("initial"));
}

#[test]
fn parse_xstate_invalid_json() {
    let result = parse_xstate("not json at all");
    assert!(result.is_err());
}

#[test]
fn parse_xstate_unknown_state_type() {
    let json = r#"{
        "id": "bad",
        "initial": "a",
        "states": {
            "a": { "type": "imaginary" }
        }
    }"#;
    let result = parse_xstate(json);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("imaginary"));
}
