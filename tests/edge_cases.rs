//! Edge case tests for modules with minimal coverage.

use scxml::export::{dot, json, xml};
use scxml::model::*;
use scxml::{flatten, parse_json, stats, validate};

// ── parse/json edge cases ───────────────────────────────────────────────────

#[test]
fn json_parse_with_guards_and_delay() {
    let json = r#"{
        "initial": "a",
        "states": [{
            "id": "a",
            "kind": "atomic",
            "transitions": [{
                "event": "go",
                "guard": "ready",
                "targets": ["b"],
                "actions": [],
                "delay": "PT1H",
                "quorum": 2
            }],
            "on_entry": [],
            "on_exit": [],
            "children": [],
            "initial": null
        }, {
            "id": "b",
            "kind": "final",
            "transitions": [],
            "on_entry": [],
            "on_exit": [],
            "children": [],
            "initial": null
        }]
    }"#;

    let chart = parse_json(json).unwrap();
    assert_eq!(
        chart.states[0].transitions[0].delay.as_deref(),
        Some("PT1H")
    );
    assert_eq!(chart.states[0].transitions[0].quorum, Some(2));
    assert_eq!(
        chart.states[0].transitions[0].guard.as_deref(),
        Some("ready")
    );
}

#[test]
fn json_parse_invalid_returns_error() {
    let result = parse_json("not valid json");
    assert!(result.is_err());
}

#[test]
fn json_parse_empty_chart() {
    let json = r#"{"initial": "a", "states": [{"id": "a", "kind": "atomic", "transitions": [], "on_entry": [], "on_exit": [], "children": [], "initial": null}]}"#;
    let chart = parse_json(json).unwrap();
    assert_eq!(chart.states.len(), 1);
}

// ── export/xml edge cases ───────────────────────────────────────────────────

#[test]
fn xml_export_with_history_state() {
    let chart = Statechart::new(
        "main",
        vec![State::compound(
            "main",
            "child",
            vec![
                State::history("hist", HistoryKind::Deep),
                {
                    let mut c = State::atomic("child");
                    c.transitions.push(Transition::new("done", "end"));
                    c
                },
                State::final_state("end"),
            ],
        )],
    );

    let output = xml::to_xml(&chart);
    assert!(output.contains("<history id=\"hist\" type=\"deep\""));
}

#[test]
fn xml_export_with_internal_transition() {
    let chart = Statechart::new(
        "s1",
        vec![{
            let mut s = State::compound(
                "s1",
                "child",
                vec![
                    {
                        let mut c = State::atomic("child");
                        c.transitions.push(Transition::new("done", "end"));
                        c
                    },
                    State::final_state("end"),
                ],
            );
            s.transitions
                .push(Transition::new("reset", "child").internal());
            s
        }],
    );

    let output = xml::to_xml(&chart);
    assert!(output.contains("type=\"internal\""));
}

#[test]
fn xml_export_with_actions() {
    let mut s = State::atomic("s1");
    s.on_entry.push(Action::raise("entered"));
    s.on_exit
        .push(Action::log(Some("info".into()), Some("exiting".into())));
    s.transitions
        .push(Transition::new("go", "end").with_action(Action::send("notify")));

    let chart = Statechart::new("s1", vec![s, State::final_state("end")]);
    let output = xml::to_xml(&chart);

    assert!(output.contains("<onentry>"));
    assert!(output.contains("<raise event=\"entered\""));
    assert!(output.contains("<onexit>"));
    assert!(output.contains("<log label=\"info\""));
    assert!(output.contains("<send event=\"notify\""));
}

#[test]
fn xml_export_with_datamodel() {
    let chart = Statechart::new("s1", vec![State::atomic("s1")]).with_datamodel(
        DataModel::new()
            .with_item(DataItem::with_expr("counter", "0"))
            .with_item(DataItem::new("name")),
    );

    let output = xml::to_xml(&chart);
    assert!(output.contains("<datamodel>"));
    assert!(output.contains("<data id=\"counter\" expr=\"0\""));
    assert!(output.contains("<data id=\"name\""));
}

// ── export/dot edge cases ───────────────────────────────────────────────────

#[test]
fn dot_export_with_parallel() {
    let chart = Statechart::new(
        "p",
        vec![State::parallel(
            "p",
            vec![
                State::compound(
                    "r1",
                    "r1a",
                    vec![
                        {
                            let mut s = State::atomic("r1a");
                            s.transitions.push(Transition::new("done", "r1b"));
                            s
                        },
                        State::final_state("r1b"),
                    ],
                ),
                State::compound(
                    "r2",
                    "r2a",
                    vec![
                        {
                            let mut s = State::atomic("r2a");
                            s.transitions.push(Transition::new("done", "r2b"));
                            s
                        },
                        State::final_state("r2b"),
                    ],
                ),
            ],
        )],
    );

    let dot_out = dot::to_dot(&chart);
    assert!(dot_out.contains("subgraph \"cluster_p\""));
    assert!(dot_out.contains("style=dashed")); // parallel style
    assert!(dot_out.contains("\"r1a\""));
}

#[test]
fn dot_export_with_delay_shows_dashed() {
    let mut s = State::atomic("a");
    s.transitions
        .push(Transition::new("timeout", "b").with_delay("PT48H"));

    let chart = Statechart::new("a", vec![s, State::final_state("b")]);
    let dot_out = dot::to_dot(&chart);
    assert!(dot_out.contains("style=dashed"));
    assert!(dot_out.contains("#E74C3C")); // red color for deadline
    assert!(dot_out.contains("PT48H"));
}

#[test]
fn dot_export_with_entry_actions() {
    let mut s = State::atomic("s1");
    s.on_entry.push(Action::raise("entered"));
    s.transitions.push(Transition::new("done", "end"));

    let chart = Statechart::new("s1", vec![s, State::final_state("end")]);
    let dot_out = dot::to_dot(&chart);
    assert!(dot_out.contains("entry/ raise(entered)"));
}

// ── flatten edge cases ──────────────────────────────────────────────────────

#[test]
fn flatten_empty_transitions() {
    let chart = Statechart::new("a", vec![State::atomic("a")]);
    let (states, transitions) = flatten::flatten(&chart);
    assert_eq!(states.len(), 1);
    assert!(transitions.is_empty());
}

#[test]
fn flatten_multi_target_transition() {
    let chart = Statechart::new(
        "p",
        vec![State::parallel(
            "p",
            vec![
                State::compound(
                    "r1",
                    "r1a",
                    vec![State::atomic("r1a"), State::atomic("r1b")],
                ),
                State::compound(
                    "r2",
                    "r2a",
                    vec![{
                        let mut s = State::atomic("r2a");
                        let mut t = Transition::new("go", "r1b");
                        t.targets.push("r2a".into());
                        s.transitions.push(t);
                        s
                    }],
                ),
            ],
        )],
    );

    let (_, transitions) = flatten::flatten(&chart);
    // Multi-target transition produces 2 flat transitions.
    assert_eq!(transitions.len(), 2);
}

// ── stats edge cases ────────────────────────────────────────────────────────

#[test]
fn stats_with_history_and_deadline() {
    let mut s = State::atomic("a");
    s.transitions
        .push(Transition::new("go", "b").with_delay("PT1H"));
    s.transitions
        .push(Transition::new("approve", "b").with_guard("ok"));

    let chart = Statechart::new(
        "main",
        vec![State::compound(
            "main",
            "a",
            vec![
                State::history("hist", HistoryKind::Shallow),
                s,
                State::final_state("b"),
            ],
        )],
    );

    let st = stats(&chart);
    assert_eq!(st.history_states, 1);
    assert_eq!(st.deadline_transitions, 1);
    assert_eq!(st.guarded_transitions, 1);
    assert_eq!(st.max_depth, 1);
}

// ── Display impl ────────────────────────────────────────────────────────────

#[test]
fn display_impl() {
    let chart = Statechart::new(
        "a",
        vec![
            {
                let mut s = State::atomic("a");
                s.transitions.push(Transition::new("go", "b"));
                s
            },
            State::final_state("b"),
        ],
    )
    .with_name("test");

    let display = format!("{chart}");
    assert!(display.contains("test"));
    assert!(display.contains("2 states"));
    assert!(display.contains("1 transitions"));
}

// ── json export edge cases ──────────────────────────────────────────────────

#[test]
fn json_export_roundtrip_with_nested() {
    let chart = Statechart::new(
        "outer",
        vec![State::compound(
            "outer",
            "inner",
            vec![
                {
                    let mut s = State::atomic("inner");
                    s.transitions.push(Transition::new("done", "end"));
                    s
                },
                State::final_state("end"),
            ],
        )],
    );

    let json_str = json::to_json_string(&chart).unwrap();
    let reparsed = parse_json(&json_str).unwrap();
    validate(&reparsed).unwrap();
    assert_eq!(reparsed.states[0].children.len(), 2);
}
