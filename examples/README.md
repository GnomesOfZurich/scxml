# Examples

SCXML definition files and a Rust example showing how to consume the crate.

## SCXML definitions

These `.scxml` files can be parsed, validated, exported, and resolved by the library. They serve as both test fixtures and reference material for real-world statechart patterns.

| File | Pattern | Key features |
|------|---------|--------------|
| `document_lifecycle.scxml` | Document approval flow | Linear states, review cycle, terminal archival |
| `new_product_approval.scxml` | Multi-stage NPA workflow | Compound states, guards, sequential approval chain |
| `onboarding_approval.scxml` | Client onboarding | Nested hierarchy, rejection paths, compliance checks |
| `parallel_checks.scxml` | Payment release screening | Parallel regions (sanctions + fraud), explicit join |
| `settlement.scxml` | Trade settlement | Deadline-triggered transitions (`delay="PT48H"`), auto-fail |

### XState interop

The `.xstate.json` files are XState v5 equivalents of their SCXML counterparts, used for round-trip testing with the `xstate` feature:

- `document_lifecycle.xstate.json`
- `onboarding_approval.xstate.json`

## Rust examples

### `compile_to_dispatcher.rs`

Demonstrates the full pipeline from SCXML to executable code:

1. Parses an inline SCXML definition
2. Validates structural well-formedness
3. Resolves into a `ResolvedChart` (effective transitions, inherited from ancestors)
4. Generates a Rust `match (state, event)` dispatcher

This is the canonical example of how to consume `ResolvedChart` for code generation. Inherited transitions are annotated with `// inherited from <parent>` comments.

```bash
cargo run --example compile_to_dispatcher
```

Output:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Workflow,
    Idle,
    Review,
    Done,
    Cancelled,
}

fn dispatch(state: State, event: &str, guards: &dyn Fn(&str) -> bool) -> Option<State> {
    match (state, event) {
        (State::Workflow, "cancel") => Some(State::Cancelled),
        (State::Idle, "submit") if guards("has_documents") => Some(State::Review),
        (State::Idle, "cancel") => Some(State::Cancelled), // inherited from workflow
        (State::Review, "approve") if guards("manager_ok") => Some(State::Done),
        (State::Review, "reject") => Some(State::Idle),
        (State::Review, "cancel") => Some(State::Cancelled), // inherited from workflow
        _ => None,
    }
}
```

## Using the SCXML files from Rust

```rust
use scxml::{parse_xml, validate, resolve, export};

let xml = std::fs::read_to_string("examples/settlement.scxml").unwrap();
let chart = parse_xml(&xml).unwrap();
validate(&chart).unwrap();

// Export to Graphviz DOT
let dot = export::dot::to_dot(&chart);

// Resolve for code generation / tooling consumption
let resolved = resolve(&chart);
println!("Events: {:?}", resolved.events);
```
