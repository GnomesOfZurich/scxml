use proptest::prelude::*;
use scxml::export::{json, xml};
use scxml::model::{State, Statechart, Transition};
use scxml::{flatten, parse_json, parse_xml, validate};

/// Generate a random linear statechart with 2..=max_states states.
fn arb_linear_chart(max_states: usize) -> impl Strategy<Value = Statechart> {
    (2..=max_states).prop_flat_map(|n| {
        let states: Vec<State> = (0..n)
            .map(|i| {
                if i == n - 1 {
                    State::final_state(format!("s{i}"))
                } else {
                    let mut s = State::atomic(format!("s{i}"));
                    s.transitions
                        .push(Transition::new("next", format!("s{}", i + 1)));
                    s
                }
            })
            .collect();
        Just(Statechart::new("s0", states))
    })
}

/// Generate a statechart with optional guard on the first transition.
fn arb_guarded_chart() -> impl Strategy<Value = Statechart> {
    prop::option::of("[a-z_]{3,20}").prop_map(|guard| {
        let mut draft = State::atomic("draft");
        let mut t = Transition::new("submit", "done");
        if let Some(g) = guard {
            t = t.with_guard(g);
        }
        draft.transitions.push(t);
        Statechart::new("draft", vec![draft, State::final_state("done")])
    })
}

proptest! {
    #[test]
    fn valid_chart_stays_valid_after_xml_roundtrip(chart in arb_linear_chart(50)) {
        validate(&chart).unwrap();
        let xml_str = xml::to_xml(&chart);
        let chart2 = parse_xml(&xml_str).unwrap();
        validate(&chart2).unwrap();

        // Structural equivalence.
        prop_assert_eq!(chart.initial, chart2.initial);
        prop_assert_eq!(chart.states.len(), chart2.states.len());
        for (s1, s2) in chart.states.iter().zip(chart2.states.iter()) {
            prop_assert_eq!(&s1.id, &s2.id);
            prop_assert_eq!(s1.kind, s2.kind);
            prop_assert_eq!(s1.transitions.len(), s2.transitions.len());
        }
    }

    #[test]
    fn valid_chart_stays_valid_after_json_roundtrip(chart in arb_linear_chart(50)) {
        validate(&chart).unwrap();
        let json_str = json::to_json_string(&chart).unwrap();
        let chart2 = parse_json(&json_str).unwrap();
        validate(&chart2).unwrap();

        prop_assert_eq!(chart.initial, chart2.initial);
        prop_assert_eq!(chart.states.len(), chart2.states.len());
    }

    #[test]
    fn flatten_count_matches_state_count(chart in arb_linear_chart(100)) {
        let (states, transitions) = flatten::flatten(&chart);
        prop_assert_eq!(states.len(), chart.states.len());
        // Linear chain: n-1 transitions (final has none).
        let expected_transitions: usize = chart
            .states
            .iter()
            .map(|s| s.transitions.len())
            .sum();
        prop_assert_eq!(transitions.len(), expected_transitions);
    }

    #[test]
    fn guards_survive_roundtrip(chart in arb_guarded_chart()) {
        let xml_str = xml::to_xml(&chart);
        let chart2 = parse_xml(&xml_str).unwrap();

        let orig_guard = chart.states[0].transitions[0].guard.as_deref();
        let rt_guard = chart2.states[0].transitions[0].guard.as_deref();
        prop_assert_eq!(orig_guard, rt_guard);
    }

    #[test]
    fn initial_state_always_in_flat_states(chart in arb_linear_chart(50)) {
        let (states, _) = flatten::flatten(&chart);
        prop_assert!(
            states.iter().any(|s| s.id == chart.initial && s.initial),
            "initial state not found or not marked initial in flat output"
        );
    }
}
