# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-17

First public release. A W3C SCXML statechart library for Rust that parses,
validates, visualises, and simulates Harel statecharts. Not a runtime executor;
compiled native types handle that.

### Repository

- Generic example workflows scrubbed of project-internal naming.
- README sections reorganised: added Installation, framework-agnostic note, FAQ.
- `types.d.ts` moved from repo root into `demo/` (it documents the WASM consumer surface).
- Cargo.toml metadata: `homepage`, `documentation` URLs added; keywords/categories tuned for discoverability.

### Parse

- W3C SCXML XML via quick-xml (`parse_xml`).
- JSON via serde (`parse_json`).
- Supports `<scxml>`, `<state>`, `<parallel>`, `<final>`, `<history>`, `<transition>`,
  `<onentry>`, `<onexit>`, `<datamodel>`, `<data>`.
- All W3C executable content elements stored as action descriptors (never evaluated):
  `<cancel>`, `<if>`/`<elseif>`/`<else>`, `<foreach>`, `<script>`, `<invoke>`.

### Model

- `Statechart`, `State` (Atomic, Compound, Parallel, Final, History), `Transition`,
  `Action` (Raise, Send, Assign, Log, Cancel, If, Foreach, Script, Invoke, Custom
  descriptors), `IfBranch`, `DataModel`, `DataItem`.
- Named guards (string references, not executable expressions).
- Extensions: `delay` (ISO 8601 duration) and `gnomes:quorum` attributes on transitions.
- `Display` impl for `Statechart`.
- rkyv `Archive`/`Serialize`/`Deserialize` derives on all model types (feature-gated).

### Validate

- **Structural**: duplicate IDs, unknown transition targets, initial state existence,
  final state constraints, compound/parallel region rules.
- **Liveness**: BFS reachability from initial state, deadlock detection with inherited
  transition awareness (children of compound states with parent-level transitions are
  not flagged).

### Export

- SCXML XML (roundtrip-capable).
- DOT (Graphviz) with delay, quorum, and entry/exit action labels; deadline transitions
  rendered as dashed red lines.
- Mermaid stateDiagram-v2 with parallel region separators and compound state blocks.
- JSON via serde.
- Flat state/transition lists for frontend rendering.

### Simulate

- Lightweight test executor (`Simulator`) with event sending, guard evaluation
  (pass-all or custom closure), transition history, and reset.
- Not a production runtime; designed for testing workflow definitions before deployment.

### Build

- `StatechartBuilder` for ergonomic programmatic construction with compound, parallel,
  entry/exit actions, delay, quorum, and guard support.
- Consuming (`with_*`) and mutating (`set_*`) builder methods on `Transition`.

### Utilities

- **Diff**: structural comparison of two statecharts with path-qualified differences
  (changed, added, removed).
- **Stats**: `StatechartStats` with state counts by kind, transition counts
  (total/guarded/deadline), max nesting depth, data item count.
- **Sanitize**: `parse_untrusted()` with configurable input limits (`InputLimits`),
  DOCTYPE/ENTITY rejection, and identifier validation (`[a-zA-Z0-9_\-\.:]` only).

### WebAssembly

- `wasm-bindgen` bindings: `parseXml`, `parseJson`, `validate`, `toDot`, `toXml`,
  `flatten`, `xmlToDot` (feature-gated).
- TypeScript type declarations (`demo/types.d.ts`) for all JSON payloads.
- 306 KB optimised WASM binary.

### Demo

- Single-page HTML live editor with Graphviz rendering (viz.js), Mermaid export,
  and event simulation with state highlighting, guard toggle, and transition history.
- Four built-in examples in the demo: Simple, NPA, Settlement, Parallel.

### Testing

- 166 tests + 12 doc-tests across 9 test suites: unit, roundtrip, W3C conformance
  subset (15 tests), proptest invariants, rkyv roundtrip, edge cases, XState
  round-trip, and example file validation.

### Infrastructure

- **CI**: GitHub Actions with format, clippy, tests on Linux/macOS/Windows, benchmarks
  on 3 platforms, MSRV (1.85), security audit, docs build, coverage with auto-updated
  badge.
- **Benchmarks**: criterion benchmarks for parse, validate, export (DOT/XML/JSON),
  flatten, stats, diff, and rkyv (serialize/access/deserialize).
- **Examples**: NPA approval, document lifecycle, settlement, parallel compliance checks.
