//! W3C SCXML test suite subset.
//!
//! Tests adapted from the W3C SCXML conformance test suite:
//! https://www.w3.org/Voice/2013/scxml-irp/
//!
//! We test the subset of features this crate supports (parsing, structural
//! correctness, state hierarchy). We don't test runtime execution since
//! this crate is not an interpreter.

use scxml::model::StateKind;
use scxml::{parse_xml, validate};

// ── §3.2 <state> ────────────────────────────────────────────────────────────

/// Test 144: A state with no children is treated as atomic.
#[test]
fn w3c_144_atomic_state() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition event="done" target="pass"/>
            </state>
            <final id="pass"/>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    validate(&chart).unwrap();
    assert_eq!(chart.states[0].kind, StateKind::Atomic);
}

/// Test 147: A compound state must have at least one child state.
/// (We parse `<state>` with children as Compound.)
#[test]
fn w3c_147_compound_state() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1" initial="s1_1">
                <state id="s1_1">
                    <transition event="done" target="pass"/>
                </state>
            </state>
            <final id="pass"/>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    validate(&chart).unwrap();
    assert_eq!(chart.states[0].kind, StateKind::Compound);
    assert_eq!(chart.states[0].children.len(), 1);
}

// ── §3.3 <parallel> ────────────────────────────────────────────────────────

/// Test 155: Parallel state. All children are simultaneously active.
#[test]
fn w3c_155_parallel_state() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="p">
            <parallel id="p">
                <state id="r1" initial="r1_a">
                    <state id="r1_a">
                        <transition event="done" target="r1_b"/>
                    </state>
                    <final id="r1_b"/>
                </state>
                <state id="r2" initial="r2_a">
                    <state id="r2_a">
                        <transition event="done" target="r2_b"/>
                    </state>
                    <final id="r2_b"/>
                </state>
            </parallel>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    validate(&chart).unwrap();
    assert_eq!(chart.states[0].kind, StateKind::Parallel);
    assert_eq!(chart.states[0].children.len(), 2);
}

// ── §3.4 <transition> ──────────────────────────────────────────────────────

/// Test 351: Eventless transition (no event attribute).
#[test]
fn w3c_351_eventless_transition() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition target="pass"/>
            </state>
            <final id="pass"/>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    let t = &chart.states[0].transitions[0];
    assert!(t.event.is_none());
    assert_eq!(t.targets[0].as_str(), "pass");
}

/// Test 352: Transition with guard condition.
#[test]
fn w3c_352_guarded_transition() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition event="e" cond="true" target="pass"/>
                <transition event="e" target="fail"/>
            </state>
            <final id="pass"/>
            <final id="fail"/>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    validate(&chart).unwrap();
    assert_eq!(
        chart.states[0].transitions[0].guard.as_deref(),
        Some("true")
    );
    assert!(chart.states[0].transitions[1].guard.is_none());
}

/// Test 355: Multiple targets on a transition (space-separated).
#[test]
fn w3c_355_multiple_targets() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="p">
            <parallel id="p">
                <state id="r1" initial="r1_a">
                    <state id="r1_a"/>
                    <state id="r1_b"/>
                </state>
                <state id="r2" initial="r2_a">
                    <state id="r2_a">
                        <transition event="e" target="r1_b r2_b"/>
                    </state>
                    <state id="r2_b"/>
                </state>
            </parallel>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    let t = &chart.states[0].children[1].children[0].transitions[0];
    assert_eq!(t.targets.len(), 2);
    assert_eq!(t.targets[0].as_str(), "r1_b");
    assert_eq!(t.targets[1].as_str(), "r2_b");
}

/// Test 403: Internal transition (type="internal").
#[test]
fn w3c_403_internal_transition() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1" initial="s1_1">
                <transition event="e" type="internal" target="s1_1"/>
                <state id="s1_1">
                    <transition event="done" target="pass"/>
                </state>
            </state>
            <final id="pass"/>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    let t = &chart.states[0].transitions[0];
    assert_eq!(t.transition_type, scxml::model::TransitionType::Internal);
}

// ── §3.5 <final> ───────────────────────────────────────────────────────────

/// Test 372: Final state is recognized.
#[test]
fn w3c_372_final_state() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition event="done" target="pass"/>
            </state>
            <final id="pass"/>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    validate(&chart).unwrap();
    assert_eq!(chart.states[1].kind, StateKind::Final);
}

// ── §3.6 <history> ─────────────────────────────────────────────────────────

/// Test 387: Shallow history state.
#[test]
fn w3c_387_shallow_history() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1" initial="s1_1">
                <history id="h" type="shallow">
                    <transition target="s1_1"/>
                </history>
                <state id="s1_1">
                    <transition event="next" target="s1_2"/>
                </state>
                <state id="s1_2">
                    <transition event="done" target="pass"/>
                </state>
            </state>
            <final id="pass"/>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    validate(&chart).unwrap();
    let hist = &chart.states[0].children[0];
    assert_eq!(
        hist.kind,
        StateKind::History(scxml::model::HistoryKind::Shallow)
    );
    assert_eq!(hist.transitions.len(), 1);
}

/// Test 388: Deep history state.
#[test]
fn w3c_388_deep_history() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1" initial="s1_1">
                <history id="h" type="deep">
                    <transition target="s1_1"/>
                </history>
                <state id="s1_1">
                    <transition event="done" target="pass"/>
                </state>
            </state>
            <final id="pass"/>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    let hist = &chart.states[0].children[0];
    assert_eq!(
        hist.kind,
        StateKind::History(scxml::model::HistoryKind::Deep)
    );
}

// ── §3.9 <datamodel> ──────────────────────────────────────────────────────

/// Test 487: Datamodel with data declarations.
#[test]
fn w3c_487_datamodel() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1"
               datamodel="null">
            <datamodel>
                <data id="x" expr="1"/>
                <data id="y"/>
            </datamodel>
            <state id="s1">
                <transition event="done" target="pass"/>
            </state>
            <final id="pass"/>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    assert_eq!(chart.datamodel.items.len(), 2);
    assert_eq!(chart.datamodel.items[0].id.as_str(), "x");
    assert_eq!(chart.datamodel.items[0].expr.as_deref(), Some("1"));
    assert!(chart.datamodel.items[1].expr.is_none());
}

// ── §3.12 <raise> ──────────────────────────────────────────────────────────

/// Test 550: Raise action stored as descriptor.
#[test]
fn w3c_550_raise_action() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry>
                    <raise event="internal_event"/>
                </onentry>
                <transition event="done" target="pass"/>
            </state>
            <final id="pass"/>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    assert_eq!(chart.states[0].on_entry.len(), 1);
    match &chart.states[0].on_entry[0].kind {
        scxml::model::ActionKind::Raise { event } => {
            assert_eq!(event.as_str(), "internal_event");
        }
        other => panic!("expected Raise, got {other:?}"),
    }
}

// ── §4.1 initial attribute ─────────────────────────────────────────────────

/// The initial attribute on <scxml> must reference an existing state.
#[test]
fn w3c_initial_attribute_must_exist() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="nonexistent">
            <state id="s1"/>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    let result = validate(&chart);
    assert!(result.is_err());
}

// ── Executable content elements stored as descriptors ──────────────────────

/// <script> is stored as a descriptor (not executed).
#[test]
fn w3c_script_as_descriptor() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry>
                    <script>var x = 1;</script>
                </onentry>
            </state>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    assert!(matches!(
        &chart.states[0].on_entry[0].kind,
        scxml::ActionKind::Script { content } if content == "var x = 1;"
    ));
}

/// <invoke> is stored as a descriptor (not executed).
#[test]
fn w3c_invoke_as_descriptor() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <invoke type="scxml" src="child.scxml"/>
            </state>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();
    assert!(matches!(
        &chart.states[0].on_entry[0].kind,
        scxml::ActionKind::Invoke { invoke_type: Some(t), src: Some(s), .. }
        if t == "scxml" && s == "child.scxml"
    ));
}
