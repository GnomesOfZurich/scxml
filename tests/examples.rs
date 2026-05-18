//! Parse and validate all example SCXML files to ensure they stay valid.

use scxml::{flatten, parse_xml, stats, validate};

#[cfg(feature = "xstate")]
use scxml::xstate::parse_xstate;

fn load_example(name: &str) -> String {
    std::fs::read_to_string(format!("examples/{name}")).unwrap()
}

#[test]
fn example_new_product_approval() {
    let xml = load_example("new_product_approval.scxml");
    let chart = parse_xml(&xml).unwrap();
    validate(&chart).unwrap();

    let s = stats(&chart);
    assert_eq!(s.total_states, 7);
    assert_eq!(s.final_states, 1);
    assert!(s.guarded_transitions > 0);
    assert!(s.deadline_transitions > 0);
    assert_eq!(chart.datamodel.items.len(), 3);

    // Quorum is parsed.
    let committee = &chart.states[4]; // committee_review
    assert_eq!(committee.transitions[0].quorum, Some(3));
}

#[test]
fn example_document_lifecycle() {
    let xml = load_example("document_lifecycle.scxml");
    let chart = parse_xml(&xml).unwrap();
    validate(&chart).unwrap();

    let s = stats(&chart);
    assert_eq!(s.total_states, 5);
    assert_eq!(s.final_states, 1);
}

#[test]
fn example_settlement() {
    let xml = load_example("settlement.scxml");
    let chart = parse_xml(&xml).unwrap();
    validate(&chart).unwrap();

    let s = stats(&chart);
    assert_eq!(s.total_states, 6);
    assert_eq!(s.deadline_transitions, 1);

    // Flatten produces correct counts.
    let (states, _transitions) = flatten::flatten(&chart);
    assert_eq!(states.len(), 6);
}

#[test]
fn example_parallel_checks() {
    let xml = load_example("parallel_checks.scxml");
    let chart = parse_xml(&xml).unwrap();
    validate(&chart).unwrap();

    let s = stats(&chart);
    assert_eq!(s.parallel_states, 1);
    assert_eq!(s.compound_states, 2); // credit_check + aml_check
    assert!(s.max_depth > 0);
}

#[test]
fn example_onboarding_approval() {
    let xml = load_example("onboarding_approval.scxml");
    let chart = parse_xml(&xml).unwrap();
    validate(&chart).unwrap();

    let s = stats(&chart);

    // Top-level: intake, parallel_checks, committee_review, approved, active, rejected
    // Inside parallel: kyc(3), credit(3), aml(4) compound states with children
    assert_eq!(s.parallel_states, 1);
    assert!(s.compound_states >= 3); // kyc, credit, aml
    assert!(s.final_states >= 2); // active + rejected + region finals
    assert!(s.guarded_transitions > 0);
    assert!(s.deadline_transitions > 0); // P14D timeout
    assert_eq!(chart.datamodel.items.len(), 2); // client_id, risk_rating

    // Parallel state has exit transitions (the join pattern).
    let parallel = chart.find_state("parallel_checks").unwrap();
    assert_eq!(parallel.kind, scxml::model::StateKind::Parallel);
    assert_eq!(parallel.children.len(), 3); // kyc, credit, aml
    assert!(
        parallel.transitions.len() >= 2,
        "parallel state should have exit transitions (checks_complete + checks_failed)"
    );

    // Committee review has quorum.
    let committee = chart.find_state("committee_review").unwrap();
    assert_eq!(committee.transitions[0].quorum, Some(2));
    assert_eq!(
        committee.transitions[0].guard.as_deref(),
        Some("approval.committee")
    );

    // Flatten includes all nested states.
    let (flat_states, flat_transitions) = flatten::flatten(&chart);
    assert!(flat_states.len() >= 16, "should flatten all nested states");
    assert!(
        flat_transitions.len() >= 10,
        "should flatten all transitions"
    );

    // DOT export includes the parallel subgraph.
    let dot = scxml::export::dot::to_dot(&chart);
    assert!(dot.contains("cluster_parallel_checks"));
    assert!(dot.contains("cluster_kyc"));
    assert!(dot.contains("cluster_credit"));
    assert!(dot.contains("cluster_aml"));

    // Mermaid export renders the parallel state.
    let mermaid = scxml::export::mermaid::to_mermaid(&chart);
    assert!(mermaid.contains("state parallel_checks"));
    assert!(mermaid.contains("--")); // parallel region separator

    // XML roundtrip preserves structure.
    let xml_out = scxml::export::xml::to_xml(&chart);
    let chart2 = parse_xml(&xml_out).unwrap();
    validate(&chart2).unwrap();
    assert_eq!(
        chart.iter_all_states().count(),
        chart2.iter_all_states().count()
    );
}

#[test]
fn all_examples_produce_valid_dot() {
    for name in [
        "new_product_approval.scxml",
        "document_lifecycle.scxml",
        "settlement.scxml",
        "parallel_checks.scxml",
        "onboarding_approval.scxml",
    ] {
        let xml = load_example(name);
        let chart = parse_xml(&xml).unwrap();
        let dot = scxml::export::dot::to_dot(&chart);
        assert!(
            dot.contains("digraph statechart"),
            "DOT missing header for {name}"
        );
        assert!(dot.contains("__start"), "DOT missing start node for {name}");
    }
}

/// Verify the XState JSON example matches the SCXML version structurally.
#[cfg(feature = "xstate")]
#[test]
fn example_document_lifecycle_xstate() {
    let json = load_example("document_lifecycle.xstate.json");
    let from_xstate = parse_xstate(&json).unwrap();
    validate(&from_xstate).unwrap();

    let xml = load_example("document_lifecycle.scxml");
    let from_scxml = parse_xml(&xml).unwrap();

    // Same structure.
    assert_eq!(from_xstate.states.len(), from_scxml.states.len());
    assert_eq!(from_xstate.initial, from_scxml.initial);

    let s = stats(&from_xstate);
    assert_eq!(s.total_states, 5);
    assert_eq!(s.final_states, 1);
}

/// Verify the XState JSON onboarding example matches the SCXML version.
#[cfg(feature = "xstate")]
#[test]
fn example_onboarding_approval_xstate() {
    let json = load_example("onboarding_approval.xstate.json");
    let from_xstate = parse_xstate(&json).unwrap();
    validate(&from_xstate).unwrap();

    let s = stats(&from_xstate);
    assert_eq!(s.parallel_states, 1);
    assert!(s.compound_states >= 3);
    assert!(s.final_states >= 2);

    // Parallel state has exit transitions.
    let parallel = from_xstate.find_state("parallel_checks").unwrap();
    assert_eq!(parallel.kind, scxml::model::StateKind::Parallel);
    assert_eq!(parallel.children.len(), 3);
    assert!(parallel.transitions.len() >= 2);
}

/// `resolve()` smoke + invariant check across every example.
///
/// Invariants any well-formed `ResolvedChart` must satisfy:
/// - Every source state appears in the resolved chart (preserves hierarchy)
/// - `defined_in` of every transition resolves to a real state id
/// - The `events` catalog is sorted, deduplicated, and a superset of every
///   transition's `event`
/// - Top-level resolved states have `parent == None`; nested ones have `parent
///   == Some(..)`
/// - `initial_child` (when set) references an actual child id
#[test]
fn all_examples_resolve_with_valid_invariants() {
    use std::collections::HashSet;

    for name in [
        "new_product_approval.scxml",
        "document_lifecycle.scxml",
        "settlement.scxml",
        "parallel_checks.scxml",
        "onboarding_approval.scxml",
    ] {
        let xml = load_example(name);
        let chart = parse_xml(&xml).unwrap();
        validate(&chart).unwrap();
        let resolved = scxml::resolve(&chart);

        let source_state_count = chart.iter_all_states().count();
        assert_eq!(
            resolved.states.len(),
            source_state_count,
            "resolved chart drops or invents states for {name}"
        );

        let state_ids: HashSet<&str> = resolved.states.iter().map(|s| s.id.as_str()).collect();
        let event_set: HashSet<&str> = resolved.events.iter().map(|e| e.as_str()).collect();

        // Event catalog sorted + deduplicated.
        let mut sorted = resolved.events.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            resolved.events, sorted,
            "events catalog not sorted+deduped for {name}"
        );

        for state in &resolved.states {
            let children_set: HashSet<&str> = state.children.iter().map(|c| c.as_str()).collect();

            if let Some(ref ic) = state.initial_child {
                assert!(
                    children_set.contains(ic.as_str()),
                    "{name}: state {} initial_child {ic} not in children",
                    state.id
                );
            }

            if state.depth == 0 {
                assert!(
                    state.parent.is_none(),
                    "{name}: depth-0 state {} has parent",
                    state.id
                );
            } else {
                assert!(
                    state.parent.is_some(),
                    "{name}: nested state {} missing parent",
                    state.id
                );
            }

            for t in &state.transitions {
                assert!(
                    state_ids.contains(t.defined_in.as_str()),
                    "{name}: transition in state {} has defined_in {} not in chart",
                    state.id,
                    t.defined_in
                );
                if let Some(ref ev) = t.event {
                    assert!(
                        event_set.contains(ev.as_str()),
                        "{name}: event {ev} missing from catalog",
                    );
                }
            }
        }
    }
}

/// Pin the inheritance contract on the onboarding example: every descendant of
/// `parallel_checks` should inherit its `checks_complete` and `checks_failed`
/// exit transitions. This is the most semantically interesting `resolve()`
/// behavior — making implicit W3C inheritance explicit.
#[test]
fn onboarding_descendants_inherit_parallel_exit_transitions() {
    let xml = load_example("onboarding_approval.scxml");
    let chart = parse_xml(&xml).unwrap();
    let resolved = scxml::resolve(&chart);

    // Every descendant of `parallel_checks` (kyc, credit, aml, and their
    // children) must have `checks_complete` and `checks_failed` in its
    // effective transitions, with `defined_in == "parallel_checks"`.
    let descendant_ids: Vec<&str> = resolved
        .states
        .iter()
        .filter(|s| {
            let mut cursor = s.parent.as_deref();
            while let Some(p) = cursor {
                if p == "parallel_checks" {
                    return true;
                }
                cursor = resolved
                    .states
                    .iter()
                    .find(|x| x.id == p)
                    .and_then(|x| x.parent.as_deref());
            }
            false
        })
        .map(|s| s.id.as_str())
        .collect();

    assert!(
        !descendant_ids.is_empty(),
        "no descendants found under parallel_checks"
    );

    for id in &descendant_ids {
        let state = resolved.states.iter().find(|s| s.id == *id).unwrap();
        let has_complete = state.transitions.iter().any(|t| {
            t.event.as_deref() == Some("checks_complete") && t.defined_in == "parallel_checks"
        });
        let has_failed = state.transitions.iter().any(|t| {
            t.event.as_deref() == Some("checks_failed") && t.defined_in == "parallel_checks"
        });
        assert!(
            has_complete,
            "descendant {id} missing inherited checks_complete from parallel_checks"
        );
        assert!(
            has_failed,
            "descendant {id} missing inherited checks_failed from parallel_checks"
        );
    }
}

#[test]
fn all_examples_roundtrip_xml() {
    for name in [
        "new_product_approval.scxml",
        "document_lifecycle.scxml",
        "settlement.scxml",
        "parallel_checks.scxml",
        "onboarding_approval.scxml",
    ] {
        let xml = load_example(name);
        let chart = parse_xml(&xml).unwrap();
        let exported = scxml::export::xml::to_xml(&chart);
        let chart2 = parse_xml(&exported).unwrap();
        validate(&chart2).unwrap();
        assert_eq!(
            chart.states.len(),
            chart2.states.len(),
            "state count mismatch after roundtrip for {name}"
        );
    }
}
