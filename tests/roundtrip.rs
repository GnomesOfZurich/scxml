use scxml::export::{dot, json, xml};
use scxml::flatten::flatten;
use scxml::{parse_xml, validate};

const NPA_WORKFLOW: &str = r##"
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       name="npa-approval" initial="draft">

    <datamodel>
        <data id="product_type"/>
        <data id="variant_id"/>
    </datamodel>

    <state id="draft">
        <onentry>
            <raise event="lifecycle.entered_draft"/>
        </onentry>
        <transition event="submit" target="quant_review" cond="has_required_documents"/>
    </state>

    <state id="quant_review">
        <transition event="approve" target="legal_review" cond="quant_approved"/>
        <transition event="reject" target="draft"/>
    </state>

    <state id="legal_review">
        <transition event="approve" target="risk_review" cond="legal_approved"/>
        <transition event="reject" target="draft"/>
    </state>

    <state id="risk_review">
        <transition event="approve" target="final_approval" cond="risk_approved"/>
        <transition event="reject" target="draft"/>
    </state>

    <state id="final_approval">
        <transition event="approve" target="approved" cond="committee_approved"/>
        <transition event="reject" target="draft"/>
    </state>

    <state id="approved">
        <transition event="issue" target="issued"/>
    </state>

    <final id="issued"/>
</scxml>
"##;

#[test]
fn parse_validate_npa_workflow() {
    let chart = parse_xml(NPA_WORKFLOW).unwrap();
    validate(&chart).unwrap();

    assert_eq!(chart.name.as_deref(), Some("npa-approval"));
    assert_eq!(chart.initial.as_str(), "draft");
    assert_eq!(chart.states.len(), 7);
    assert_eq!(chart.datamodel.items.len(), 2);

    // Check guard on first transition.
    let draft = &chart.states[0];
    assert_eq!(
        draft.transitions[0].guard.as_deref(),
        Some("has_required_documents")
    );
    assert_eq!(draft.on_entry.len(), 1);
}

#[test]
fn xml_roundtrip() {
    let chart = parse_xml(NPA_WORKFLOW).unwrap();
    let xml_out = xml::to_xml(&chart);

    // Re-parse the exported XML.
    let chart2 = parse_xml(&xml_out).unwrap();
    validate(&chart2).unwrap();

    // Verify structural equivalence.
    assert_eq!(chart.initial, chart2.initial);
    assert_eq!(chart.states.len(), chart2.states.len());
    assert_eq!(chart.datamodel.items.len(), chart2.datamodel.items.len());

    for (s1, s2) in chart.states.iter().zip(chart2.states.iter()) {
        assert_eq!(s1.id, s2.id);
        assert_eq!(s1.kind, s2.kind);
        assert_eq!(s1.transitions.len(), s2.transitions.len());
    }
}

#[test]
fn json_roundtrip() {
    let chart = parse_xml(NPA_WORKFLOW).unwrap();
    let json_str = json::to_json_string(&chart).unwrap();

    // Re-parse the exported JSON.
    let chart2 = scxml::parse_json(&json_str).unwrap();
    validate(&chart2).unwrap();

    assert_eq!(chart.initial, chart2.initial);
    assert_eq!(chart.states.len(), chart2.states.len());
}

#[test]
fn dot_export_contains_all_states() {
    let chart = parse_xml(NPA_WORKFLOW).unwrap();
    let dot_out = dot::to_dot(&chart);

    assert!(dot_out.contains("digraph statechart"));
    assert!(dot_out.contains("\"draft\""));
    assert!(dot_out.contains("\"quant_review\""));
    assert!(dot_out.contains("\"legal_review\""));
    assert!(dot_out.contains("\"risk_review\""));
    assert!(dot_out.contains("\"final_approval\""));
    assert!(dot_out.contains("\"approved\""));
    assert!(dot_out.contains("\"issued\""));
    // Guards should appear as labels.
    assert!(dot_out.contains("[has_required_documents]"));
    assert!(dot_out.contains("[quant_approved]"));
}

#[test]
fn flatten_produces_correct_counts() {
    let chart = parse_xml(NPA_WORKFLOW).unwrap();
    let (states, transitions) = flatten(&chart);

    // 7 states.
    assert_eq!(states.len(), 7);
    // draft→quant_review, quant_review→legal_review, quant_review→draft,
    // legal_review→risk_review, legal_review→draft, risk_review→final_approval,
    // risk_review→draft, final_approval→approved, final_approval→draft,
    // approved→issued = 10 transitions.
    assert_eq!(transitions.len(), 10);

    // draft is initial.
    assert!(
        states
            .iter()
            .find(|s| s.id.as_str() == "draft")
            .unwrap()
            .initial
    );
}

#[test]
fn parallel_state_workflow() {
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="processing">
            <parallel id="processing">
                <state id="credit_check" initial="pending_credit">
                    <state id="pending_credit">
                        <transition event="credit_ok" target="credit_passed"/>
                    </state>
                    <final id="credit_passed"/>
                </state>
                <state id="compliance_check" initial="pending_compliance">
                    <state id="pending_compliance">
                        <transition event="compliance_ok" target="compliance_passed"/>
                    </state>
                    <final id="compliance_passed"/>
                </state>
            </parallel>
        </scxml>
    "#;

    let chart = parse_xml(xml).unwrap();
    validate(&chart).unwrap();

    assert_eq!(chart.states[0].kind, scxml::StateKind::Parallel);
    assert_eq!(chart.states[0].children.len(), 2);

    let (flat_states, _) = flatten(&chart);
    // processing + credit_check + pending_credit + credit_passed +
    // compliance_check + pending_compliance + compliance_passed = 7
    assert_eq!(flat_states.len(), 7);
}

#[test]
fn delay_and_quorum_roundtrip() {
    let xml = r##"
        <scxml xmlns="http://www.w3.org/2005/07/scxml"
               xmlns:gnomes="http://gnomes.dev/scxml"
               version="1.0" initial="pending">
            <state id="pending">
                <transition event="approve" target="approved"
                            cond="approval.committee" gnomes:quorum="3"/>
                <transition event="timeout" target="expired" delay="PT48H"/>
            </state>
            <final id="approved"/>
            <final id="expired"/>
        </scxml>
    "##;

    let chart = parse_xml(xml).unwrap();
    validate(&chart).unwrap();

    let pending = &chart.states[0];
    // Quorum on first transition.
    assert_eq!(pending.transitions[0].quorum, Some(3));
    assert_eq!(
        pending.transitions[0].guard.as_deref(),
        Some("approval.committee")
    );
    // Delay on second transition.
    assert_eq!(pending.transitions[1].delay.as_deref(), Some("PT48H"));

    // XML roundtrip preserves delay and quorum.
    let xml_out = xml::to_xml(&chart);
    assert!(xml_out.contains("delay=\"PT48H\""));
    assert!(xml_out.contains("gnomes:quorum=\"3\""));

    let chart2 = parse_xml(&xml_out).unwrap();
    assert_eq!(chart2.states[0].transitions[0].quorum, Some(3));
    assert_eq!(
        chart2.states[0].transitions[1].delay.as_deref(),
        Some("PT48H")
    );
}

// ── New action kinds roundtrip tests ───────────────────────────────────────

const ACTIONS_XML: &str = r#"
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
    <state id="s1">
        <onentry>
            <cancel sendid="timer1"/>
            <if cond="x > 0">
                <log label="positive"/>
            <elseif cond="x == 0"/>
                <log label="zero"/>
            <else/>
                <log label="negative"/>
            </if>
            <foreach array="items" item="x" index="i">
                <raise event="item_processed"/>
            </foreach>
            <script>console.log(hello)</script>
        </onentry>
        <transition event="go" target="s2"/>
    </state>
    <state id="s2">
        <invoke type="scxml" src="child.scxml" id="child1"/>
        <transition event="done" target="end"/>
    </state>
    <final id="end"/>
</scxml>
"#;

#[test]
fn new_action_kinds_xml_roundtrip() {
    let chart = parse_xml(ACTIONS_XML).unwrap();
    let xml_out = xml::to_xml(&chart);

    // Verify all action elements appear in output.
    assert!(xml_out.contains("<cancel"));
    assert!(xml_out.contains("<if"));
    assert!(xml_out.contains("<elseif"));
    assert!(xml_out.contains("<else/>"));
    assert!(xml_out.contains("<foreach"));
    assert!(xml_out.contains("<script>"));
    assert!(xml_out.contains("<invoke"));

    // Re-parse and verify structure.
    let chart2 = parse_xml(&xml_out).unwrap();
    let s1 = &chart2.states[0];
    assert_eq!(s1.on_entry.len(), 4); // cancel, if, foreach, script

    assert!(
        matches!(&s1.on_entry[0].kind, scxml::ActionKind::Cancel { sendid } if sendid == "timer1")
    );

    if let scxml::ActionKind::If { branches, actions } = &s1.on_entry[1].kind {
        assert_eq!(branches.len(), 3);
        assert_eq!(actions.len(), 3); // 3 log actions
    } else {
        panic!("expected If");
    }

    if let scxml::ActionKind::Foreach {
        array,
        item,
        index,
        actions,
    } = &s1.on_entry[2].kind
    {
        assert_eq!(array.as_str(), "items");
        assert_eq!(item.as_str(), "x");
        assert_eq!(index.as_deref(), Some("i"));
        assert_eq!(actions.len(), 1);
    } else {
        panic!("expected Foreach");
    }

    assert!(
        matches!(&s1.on_entry[3].kind, scxml::ActionKind::Script { content } if content == "console.log(hello)")
    );

    // Invoke stored in on_entry of s2.
    let s2 = &chart2.states[1];
    assert!(matches!(
        &s2.on_entry[0].kind,
        scxml::ActionKind::Invoke { invoke_type: Some(t), src: Some(s), id: Some(i) }
        if t == "scxml" && s == "child.scxml" && i == "child1"
    ));
}

#[test]
fn new_action_kinds_json_roundtrip() {
    let chart = parse_xml(ACTIONS_XML).unwrap();
    let json_str = json::to_json_string(&chart).unwrap();

    let chart2 = scxml::parse_json(&json_str).unwrap();
    let s1 = &chart2.states[0];
    assert_eq!(s1.on_entry.len(), 4);
    assert!(matches!(
        &s1.on_entry[0].kind,
        scxml::ActionKind::Cancel { .. }
    ));
    assert!(matches!(&s1.on_entry[1].kind, scxml::ActionKind::If { .. }));
    assert!(matches!(
        &s1.on_entry[2].kind,
        scxml::ActionKind::Foreach { .. }
    ));
    assert!(matches!(
        &s1.on_entry[3].kind,
        scxml::ActionKind::Script { .. }
    ));

    let s2 = &chart2.states[1];
    assert!(matches!(
        &s2.on_entry[0].kind,
        scxml::ActionKind::Invoke { .. }
    ));
}

#[test]
fn new_action_kinds_in_dot_export() {
    let chart = parse_xml(ACTIONS_XML).unwrap();
    let dot_out = dot::to_dot(&chart);

    // Entry actions should appear as labels.
    assert!(dot_out.contains("cancel(timer1)"));
    assert!(dot_out.contains("if(...)"));
    assert!(dot_out.contains("foreach(items)"));
    assert!(dot_out.contains("script"));
}

// ── Depth limit tests ──────────────────────────────────────────────────────

#[test]
fn depth_limit_truncates_export() {
    // Build a 5-level deep chart.
    let xml = r#"
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="l0">
            <state id="l0" initial="l1">
                <state id="l1" initial="l2">
                    <state id="l2" initial="l3">
                        <state id="l3" initial="l4">
                            <state id="l4">
                                <transition event="go" target="l4"/>
                            </state>
                        </state>
                    </state>
                </state>
            </state>
        </scxml>
    "#;
    let chart = parse_xml(xml).unwrap();

    scxml::set_max_depth(3);
    let xml_out = xml::to_xml(&chart);
    // l0 (depth 1), l1 (depth 2), l2 (depth 3) should appear; l3+ truncated.
    assert!(xml_out.contains("id=\"l0\""));
    assert!(xml_out.contains("id=\"l1\""));
    assert!(xml_out.contains("id=\"l2\""));
    assert!(!xml_out.contains("id=\"l4\""));

    // Reset to default.
    scxml::set_max_depth(scxml::DEFAULT_MAX_DEPTH);
}

#[test]
fn action_depth_limit_rejects_deeply_nested_if() {
    // Build XML with deeply nested <if> blocks (exceeding MAX_ACTION_DEPTH=32).
    let mut xml = String::from(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
        <state id="s"><onentry>"#,
    );
    for i in 0..40 {
        xml.push_str(&format!("<if cond=\"c{i}\">"));
    }
    xml.push_str("<log label=\"deep\"/>");
    for _ in 0..40 {
        xml.push_str("</if>");
    }
    xml.push_str("</onentry></state></scxml>");

    let result = parse_xml(&xml);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("action nesting too deep")
    );
}
