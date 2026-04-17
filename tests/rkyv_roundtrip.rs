#![cfg(feature = "rkyv")]

use rkyv::util::AlignedVec;
use scxml::model::{State, Statechart, Transition};

fn npa_chart() -> Statechart {
    Statechart::new(
        "draft",
        vec![
            {
                let mut s = State::atomic("draft");
                s.transitions
                    .push(Transition::new("submit", "review").with_guard("has_documents"));
                s
            },
            {
                let mut s = State::atomic("review");
                s.transitions
                    .push(Transition::new("approve", "done").with_guard("approval.committee"));
                s.transitions.push(Transition::new("reject", "draft"));
                s
            },
            State::final_state("done"),
        ],
    )
    .with_name("npa")
}

#[test]
fn rkyv_serialize_deserialize_roundtrip() {
    let chart = npa_chart();

    // Serialize to bytes.
    let bytes =
        rkyv::api::high::to_bytes_in::<_, rkyv::rancor::Error>(&chart, AlignedVec::<16>::new())
            .unwrap();

    // Access zero-copy archived form.
    let archived = rkyv::api::high::access::<
        scxml::model::statechart::ArchivedStatechart,
        rkyv::rancor::Error,
    >(&bytes)
    .unwrap();

    // Verify archived fields match.
    assert_eq!(archived.initial.as_str(), "draft");
    assert_eq!(archived.name.as_ref().unwrap().as_str(), "npa");
    assert_eq!(archived.states.len(), 3);
    assert_eq!(archived.states[0].id.as_str(), "draft");
    assert_eq!(archived.states[0].transitions.len(), 1);
    assert_eq!(archived.states[1].transitions.len(), 2);
}

#[test]
fn rkyv_deserialized_chart_is_usable() {
    let chart = npa_chart();
    let bytes =
        rkyv::api::high::to_bytes_in::<_, rkyv::rancor::Error>(&chart, AlignedVec::<16>::new())
            .unwrap();

    // Deserialize back to owned Statechart.
    let deserialized: Statechart =
        rkyv::from_bytes::<Statechart, rkyv::rancor::Error>(&bytes).unwrap();

    // Validate the deserialized chart.
    scxml::validate(&deserialized).unwrap();

    assert_eq!(deserialized.initial.as_str(), "draft");
    assert_eq!(deserialized.states.len(), 3);
    assert_eq!(
        deserialized.states[0].transitions[0].guard.as_deref(),
        Some("has_documents")
    );
}

#[test]
fn rkyv_with_nested_states() {
    let chart = Statechart::new(
        "main",
        vec![State::compound(
            "main",
            "child_a",
            vec![
                {
                    let mut s = State::atomic("child_a");
                    s.transitions.push(Transition::new("next", "child_b"));
                    s
                },
                {
                    let mut s = State::atomic("child_b");
                    s.transitions.push(Transition::new("done", "end"));
                    s
                },
                State::final_state("end"),
            ],
        )],
    );

    let bytes =
        rkyv::api::high::to_bytes_in::<_, rkyv::rancor::Error>(&chart, AlignedVec::<16>::new())
            .unwrap();
    let archived = rkyv::api::high::access::<
        scxml::model::statechart::ArchivedStatechart,
        rkyv::rancor::Error,
    >(&bytes)
    .unwrap();

    assert_eq!(archived.states[0].children.len(), 3);
    assert_eq!(archived.states[0].children[0].id.as_str(), "child_a");
}
