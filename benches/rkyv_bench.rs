#![cfg(feature = "rkyv")]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use rkyv::util::AlignedVec;
use scxml::model::{State, Statechart, Transition};
use scxml::parse_xml;

/// Checked-in document lifecycle example (5 states).
const DOCUMENT_XML: &str = include_str!("../examples/document_lifecycle.scxml");

fn document_chart() -> Statechart {
    parse_xml(DOCUMENT_XML).unwrap()
}

fn chain_chart(n: usize) -> Statechart {
    let mut states = Vec::with_capacity(n);
    for i in 0..n - 1 {
        let mut s = State::atomic(format!("s{i}"));
        s.transitions
            .push(Transition::new("next", format!("s{}", i + 1)));
        states.push(s);
    }
    states.push(State::final_state(format!("s{}", n - 1)));
    Statechart::new("s0", states)
}

fn bench_rkyv(c: &mut Criterion) {
    let mut group = c.benchmark_group("rkyv");

    let document = document_chart();
    let document_bytes =
        rkyv::api::high::to_bytes_in::<_, rkyv::rancor::Error>(&document, AlignedVec::<16>::new())
            .unwrap();

    group.bench_function("serialize_document_5", |b| {
        b.iter(|| {
            rkyv::api::high::to_bytes_in::<_, rkyv::rancor::Error>(
                black_box(&document),
                AlignedVec::<16>::new(),
            )
            .unwrap()
        });
    });

    group.bench_function("access_document_5", |b| {
        b.iter(|| {
            rkyv::api::high::access::<
                scxml::model::statechart::ArchivedStatechart,
                rkyv::rancor::Error,
            >(black_box(&document_bytes))
            .unwrap()
        });
    });

    group.bench_function("deserialize_document_5", |b| {
        b.iter(|| {
            rkyv::from_bytes::<Statechart, rkyv::rancor::Error>(black_box(&document_bytes)).unwrap()
        });
    });

    // 10-state chain (matches the other benchmarks).
    let small = chain_chart(10);
    let small_bytes =
        rkyv::api::high::to_bytes_in::<_, rkyv::rancor::Error>(&small, AlignedVec::<16>::new())
            .unwrap();

    group.bench_function("serialize_chain_10", |b| {
        b.iter(|| {
            rkyv::api::high::to_bytes_in::<_, rkyv::rancor::Error>(
                black_box(&small),
                AlignedVec::<16>::new(),
            )
            .unwrap()
        });
    });

    group.bench_function("access_chain_10", |b| {
        b.iter(|| {
            rkyv::api::high::access::<
                scxml::model::statechart::ArchivedStatechart,
                rkyv::rancor::Error,
            >(black_box(&small_bytes))
            .unwrap()
        });
    });

    group.bench_function("deserialize_chain_10", |b| {
        b.iter(|| {
            rkyv::from_bytes::<Statechart, rkyv::rancor::Error>(black_box(&small_bytes)).unwrap()
        });
    });

    // 50-state chain (matches the other benchmarks).
    let mid = chain_chart(50);
    let mid_bytes =
        rkyv::api::high::to_bytes_in::<_, rkyv::rancor::Error>(&mid, AlignedVec::<16>::new())
            .unwrap();

    group.bench_function("serialize_chain_50", |b| {
        b.iter(|| {
            rkyv::api::high::to_bytes_in::<_, rkyv::rancor::Error>(
                black_box(&mid),
                AlignedVec::<16>::new(),
            )
            .unwrap()
        });
    });

    group.bench_function("access_chain_50", |b| {
        b.iter(|| {
            rkyv::api::high::access::<
                scxml::model::statechart::ArchivedStatechart,
                rkyv::rancor::Error,
            >(black_box(&mid_bytes))
            .unwrap()
        });
    });

    group.bench_function("deserialize_chain_50", |b| {
        b.iter(|| {
            rkyv::from_bytes::<Statechart, rkyv::rancor::Error>(black_box(&mid_bytes)).unwrap()
        });
    });

    // 500-state chain (matches the other benchmarks).
    let big = chain_chart(500);
    let big_bytes =
        rkyv::api::high::to_bytes_in::<_, rkyv::rancor::Error>(&big, AlignedVec::<16>::new())
            .unwrap();

    group.bench_function("serialize_chain_500", |b| {
        b.iter(|| {
            rkyv::api::high::to_bytes_in::<_, rkyv::rancor::Error>(
                black_box(&big),
                AlignedVec::<16>::new(),
            )
            .unwrap()
        });
    });

    group.bench_function("access_chain_500", |b| {
        b.iter(|| {
            rkyv::api::high::access::<
                scxml::model::statechart::ArchivedStatechart,
                rkyv::rancor::Error,
            >(black_box(&big_bytes))
            .unwrap()
        });
    });

    group.bench_function("deserialize_chain_500", |b| {
        b.iter(|| {
            rkyv::from_bytes::<Statechart, rkyv::rancor::Error>(black_box(&big_bytes)).unwrap()
        });
    });

    // Compare: serialize size.
    let document_json = serde_json::to_string(&document).unwrap();
    let small_json = serde_json::to_string(&small).unwrap();
    let mid_json = serde_json::to_string(&mid).unwrap();
    println!(
        "Document-5: rkyv={} bytes, JSON={} bytes, ratio={:.1}x",
        document_bytes.len(),
        document_json.len(),
        document_json.len() as f64 / document_bytes.len() as f64
    );

    println!(
        "Chain-10: rkyv={} bytes, JSON={} bytes, ratio={:.1}x",
        small_bytes.len(),
        small_json.len(),
        small_json.len() as f64 / small_bytes.len() as f64
    );

    println!(
        "Chain-50: rkyv={} bytes, JSON={} bytes, ratio={:.1}x",
        mid_bytes.len(),
        mid_json.len(),
        mid_json.len() as f64 / mid_bytes.len() as f64
    );

    let big_json = serde_json::to_string(&big).unwrap();
    println!(
        "Chain-500: rkyv={} bytes, JSON={} bytes, ratio={:.1}x",
        big_bytes.len(),
        big_json.len(),
        big_json.len() as f64 / big_bytes.len() as f64
    );

    group.finish();
}

criterion_group!(benches, bench_rkyv);
criterion_main!(benches);
