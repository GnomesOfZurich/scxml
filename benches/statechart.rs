use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use scxml::model::{State, Statechart, Transition};
use scxml::{export, flatten, parse_xml, validate};

// ── Fixtures ────────────────────────────────────────────────────────────────

/// Checked-in document lifecycle example (5 states).
const DOCUMENT_XML: &str = include_str!("../examples/document_lifecycle.scxml");

/// Generate a linear chain SCXML with `n` states.
fn generate_chain_xml(n: usize) -> String {
    let mut xml = String::from(
        "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" initial=\"s0\">\n",
    );
    for i in 0..n - 1 {
        xml.push_str(&format!(
            "  <state id=\"s{i}\">\n    <transition event=\"next\" target=\"s{}\"/>\n  </state>\n",
            i + 1
        ));
    }
    xml.push_str(&format!("  <final id=\"s{}\"/>\n", n - 1));
    xml.push_str("</scxml>\n");
    xml
}

/// Generate a Statechart programmatically with `n` states.
fn generate_chain_chart(n: usize) -> Statechart {
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

/// Generate a parallel workflow with `regions` each containing `depth` states.
fn generate_parallel_chart(regions: usize, depth: usize) -> Statechart {
    let mut region_states = Vec::with_capacity(regions);
    for r in 0..regions {
        let mut children = Vec::with_capacity(depth);
        for d in 0..depth - 1 {
            let mut s = State::atomic(format!("r{r}_s{d}"));
            s.transitions
                .push(Transition::new("next", format!("r{r}_s{}", d + 1)));
            children.push(s);
        }
        children.push(State::final_state(format!("r{r}_s{}", depth - 1)));
        region_states.push(State::compound(
            format!("region{r}"),
            format!("r{r}_s0"),
            children,
        ));
    }
    Statechart::new("par", vec![State::parallel("par", region_states)])
}

// ── Benchmarks ──────────────────────────────────────────────────────────────

fn bench_parse_xml(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_xml");

    // Document lifecycle workflow (5 states).
    group.bench_function("document_5states", |b| {
        b.iter(|| parse_xml(black_box(DOCUMENT_XML)).unwrap());
    });

    // Linear chains of increasing size.
    for n in [10, 50, 100, 500] {
        let xml = generate_chain_xml(n);
        group.bench_with_input(BenchmarkId::new("chain", n), &xml, |b, xml| {
            b.iter(|| parse_xml(black_box(xml)).unwrap());
        });
    }

    group.finish();
}

fn bench_validate(c: &mut Criterion) {
    let mut group = c.benchmark_group("validate");

    let document = parse_xml(DOCUMENT_XML).unwrap();
    group.bench_function("document_5states", |b| {
        b.iter(|| validate(black_box(&document)).unwrap());
    });

    for n in [10, 50, 100, 500] {
        let chart = generate_chain_chart(n);
        group.bench_with_input(BenchmarkId::new("chain", n), &chart, |b, chart| {
            b.iter(|| validate(black_box(chart)).unwrap());
        });
    }

    // Parallel: 4 regions x 10 states each = 40 leaf states.
    let par = generate_parallel_chart(4, 10);
    group.bench_function("parallel_4x10", |b| {
        b.iter(|| validate(black_box(&par)).unwrap());
    });

    group.finish();
}

fn bench_export_dot(c: &mut Criterion) {
    let mut group = c.benchmark_group("export_dot");

    let document = parse_xml(DOCUMENT_XML).unwrap();
    group.bench_function("document_5states", |b| {
        b.iter(|| export::dot::to_dot(black_box(&document)));
    });

    for n in [10, 50, 100, 500] {
        let chart = generate_chain_chart(n);
        group.bench_with_input(BenchmarkId::new("chain", n), &chart, |b, chart| {
            b.iter(|| export::dot::to_dot(black_box(chart)));
        });
    }

    group.finish();
}

fn bench_export_xml(c: &mut Criterion) {
    let mut group = c.benchmark_group("export_xml");

    let document = parse_xml(DOCUMENT_XML).unwrap();
    group.bench_function("document_5states", |b| {
        b.iter(|| export::xml::to_xml(black_box(&document)));
    });

    for n in [10, 50, 100, 500] {
        let chart = generate_chain_chart(n);
        group.bench_with_input(BenchmarkId::new("chain", n), &chart, |b, chart| {
            b.iter(|| export::xml::to_xml(black_box(chart)));
        });
    }

    group.finish();
}

fn bench_export_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("export_json");

    let document = parse_xml(DOCUMENT_XML).unwrap();
    group.bench_function("document_5states", |b| {
        b.iter(|| export::json::to_json_string(black_box(&document)).unwrap());
    });

    for n in [10, 50, 500] {
        let chart = generate_chain_chart(n);
        group.bench_with_input(BenchmarkId::new("chain", n), &chart, |b, chart| {
            b.iter(|| export::json::to_json_string(black_box(chart)).unwrap());
        });
    }

    group.finish();
}

fn bench_flatten(c: &mut Criterion) {
    let mut group = c.benchmark_group("flatten");

    let document = parse_xml(DOCUMENT_XML).unwrap();
    group.bench_function("document_5states", |b| {
        b.iter(|| flatten::flatten(black_box(&document)));
    });

    for n in [10, 50, 500] {
        let chart = generate_chain_chart(n);
        group.bench_with_input(BenchmarkId::new("chain", n), &chart, |b, chart| {
            b.iter(|| flatten::flatten(black_box(chart)));
        });
    }

    let par = generate_parallel_chart(4, 10);
    group.bench_function("parallel_4x10", |b| {
        b.iter(|| flatten::flatten(black_box(&par)));
    });

    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    // Full pipeline: parse → validate → export XML.
    group.bench_function("parse_validate_export_document", |b| {
        b.iter(|| {
            let chart = parse_xml(black_box(DOCUMENT_XML)).unwrap();
            validate(&chart).unwrap();
            export::xml::to_xml(&chart)
        });
    });

    // JSON roundtrip: parse XML → export JSON → parse JSON.
    let document = parse_xml(DOCUMENT_XML).unwrap();
    let document_json = export::json::to_json_string(&document).unwrap();
    group.bench_function("json_roundtrip_document", |b| {
        b.iter(|| {
            let chart = scxml::parse_json(black_box(&document_json)).unwrap();
            export::json::to_json_string(&chart).unwrap()
        });
    });

    group.finish();
}

fn bench_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("stats");

    let document = parse_xml(DOCUMENT_XML).unwrap();
    group.bench_function("document_5states", |b| {
        b.iter(|| scxml::stats(black_box(&document)));
    });

    let big = generate_chain_chart(500);
    group.bench_function("chain_500", |b| {
        b.iter(|| scxml::stats(black_box(&big)));
    });

    group.finish();
}

fn bench_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("diff");

    let document = parse_xml(DOCUMENT_XML).unwrap();
    let document2 = parse_xml(DOCUMENT_XML).unwrap();
    group.bench_function("identical_document", |b| {
        b.iter(|| scxml::diff::diff(black_box(&document), black_box(&document2)));
    });

    let big = generate_chain_chart(500);
    let big2 = generate_chain_chart(500);
    group.bench_function("identical_chain_500", |b| {
        b.iter(|| scxml::diff::diff(black_box(&big), black_box(&big2)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse_xml,
    bench_validate,
    bench_export_dot,
    bench_export_xml,
    bench_export_json,
    bench_flatten,
    bench_roundtrip,
    bench_stats,
    bench_diff,
);
criterion_main!(benches);
