//! Compile an SCXML statechart into a Rust-style event dispatcher.
//!
//! Demonstrates the intended consumption pattern for `ResolvedChart`:
//! parse → validate → resolve → generate code from the resolved form.
//!
//! Run: `cargo run --example compile_to_dispatcher`

use scxml::{parse_xml, resolve, validate};

const SCXML: &str = r#"
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="workflow">
    <state id="workflow" initial="idle">
        <transition event="cancel" target="cancelled"/>

        <state id="idle">
            <transition event="submit" target="review" cond="has_documents"/>
        </state>
        <state id="review">
            <transition event="approve" target="done" cond="manager_ok"/>
            <transition event="reject" target="idle"/>
        </state>
        <final id="done"/>
    </state>
    <final id="cancelled"/>
</scxml>
"#;

fn main() {
    let chart = parse_xml(SCXML).expect("valid SCXML");
    validate(&chart).expect("valid statechart");
    let resolved = resolve(&chart);

    println!("// Auto-generated dispatcher from SCXML");
    println!("// Events: {:?}", resolved.events);
    println!();
    println!("#[derive(Debug, Clone, Copy, PartialEq)]");
    println!("enum State {{");
    for s in &resolved.states {
        println!("    {},", to_pascal(&s.id));
    }
    println!("}}");
    println!();
    println!(
        "fn dispatch(state: State, event: &str, guards: &dyn Fn(&str) -> bool) -> Option<State> {{"
    );
    println!("    match (state, event) {{");

    for s in &resolved.states {
        for t in &s.transitions {
            let Some(ref event) = t.event else { continue };
            if t.targets.is_empty() {
                continue;
            }
            let target = &t.targets[0];
            let inherited = if t.defined_in != s.id {
                format!(" // inherited from {}", t.defined_in)
            } else {
                String::new()
            };

            if let Some(ref guard) = t.guard {
                println!(
                    "        (State::{}, \"{}\") if guards(\"{}\") => Some(State::{}),{}",
                    to_pascal(&s.id),
                    event,
                    guard,
                    to_pascal(target),
                    inherited,
                );
            } else {
                println!(
                    "        (State::{}, \"{}\") => Some(State::{}),{}",
                    to_pascal(&s.id),
                    event,
                    to_pascal(target),
                    inherited,
                );
            }
        }
    }

    println!("        _ => None,");
    println!("    }}");
    println!("}}");
}

fn to_pascal(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + chars.as_str()
                }
                None => String::new(),
            }
        })
        .collect()
}
