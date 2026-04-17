# scxml Agent Guide

This file is the canonical, repo-shared instruction source for AI coding agents and automation helpers working in this repository.

Tool-specific files should stay thin and point back here:

- `.claude/CLAUDE.md`
- `.github/.copilot-instructions.md`

If project guidance changes, update this file first and only keep tool-specific deltas in the adapters.

## Overview

`scxml` is a W3C SCXML statechart library for Rust. It parses, validates, visualizes, diffs, exports, and simulates Harel statecharts.

It is **not** a runtime executor. This crate is the cold-path document model that sits between human-editable workflow definitions and downstream native runtime types.

Project goals:

- Strong SCXML import/export and round-trip fidelity
- Validation and review tooling for workflow documents
- Clean browser/WASM story for client-side inspection
- Publishable, platform-agnostic Rust crate with zero runtime-framework coupling

## Core Boundaries

Keep these boundaries intact:

- This crate owns parsing, validation, export, diffing, flattening, statistics, sanitization, and lightweight simulation.
- This crate does not execute production workflows at hot-path transition time.
- Guards are named predicates, not embedded logic.
- Actions are inert descriptors, not executable code.
- Platform-specific integration belongs downstream, not in this crate.

Two-speed architecture:

- Cold path: parse, validate, simulate, export, diff, review
- Hot path: downstream code compiles validated statecharts into native runtime types

## Repository Map

Primary areas:

- `src/`: public library code
- `src/model/`: statechart data model
- `src/parse/`: SCXML XML and JSON parsing
- `src/validate/`: structural and liveness validation
- `src/export/`: XML, DOT, Mermaid, JSON exporters
- `src/flatten.rs`: flat graph projection helpers
- `src/diff.rs`: structural diff support
- `src/simulate.rs`: lightweight test simulator
- `src/wasm.rs`: wasm-bindgen surface for browser/demo consumers
- `tests/`: roundtrip, invariants, examples, XState, W3C subset
- `examples/`: checked-in SCXML examples and representative XState JSON counterparts
- `benches/`: criterion performance benchmarks
- `demo/`: browser demo using the generated WASM package and synced example files
- `scripts/build-wasm-package.sh`: canonical WASM build and demo sync entry point

## Coding Rules

- Rust edition: `2024`
- MSRV: `1.87`
- Use `thiserror`, never `anyhow`, in library code.
- Use `compact_str::CompactString` for model identifiers.
- Error payload fields may use `String` where required for compatibility.
- No embedded scripting engine. Never evaluate `<script>`, `<invoke>`, `<if>`, `<foreach>`, or `<cancel>`.
- Keep public APIs semver-safe and additive where practical.
- No lint suppressions. Do not add `_unused` bindings or `#[allow(...)]` shortcuts.
- Keep changes minimal and targeted; do not reformat unrelated code.

## SCXML Semantics To Preserve

- Structural validation must recurse from top-level states; do not validate nested states as if they were root nodes.
- Liveness analysis must respect inherited transitions.
- `Transition` supports both fluent `with_*` methods and mutable `set_*` methods.
- `delay` values use ISO 8601 duration syntax.
- `gnomes:quorum` is a Gnomes extension and must remain additive to standard SCXML.
- `diff::diff()` is semantic by state ID, not by array position.
- `xstate::types` stays `pub(crate)`; user-facing XState integration goes through parse/export functions.

## Security Rules

When input is untrusted, prefer `parse_untrusted()` over raw XML parsing.

Security expectations:

- Reject `<!DOCTYPE>` and `<!ENTITY>` in untrusted parsing flows.
- Enforce identifier validation and configured input limits.
- Treat the `Statechart` as inert data only.
- Preserve escaping guarantees in XML, DOT, and Mermaid exporters.

## Performance And Benchmark Story

The benchmark story should stay coherent, honest, and based on checked-in fixtures.

Current benchmark framing:

- small realistic example: `examples/document_lifecycle.scxml` with 5 states
- synthetic scaling chain sizes: `10`, `50`, `500`
- `rkyv` benches should use the same size matrix when comparisons are shown side-by-side

If benchmarks change materially, keep these aligned together:

- `benches/statechart.rs`
- `benches/rkyv_bench.rs`
- `README.md`
- any marketing copy or demo copy that references measured numbers

## Examples And Interop

- Not every checked-in SCXML example needs an XState counterpart.
- Keep XState JSON files for representative interop coverage, not for box-ticking parity.
- Prefer realistic examples over toy workflows when they clarify hierarchy, parallelism, approval flow, or review semantics.

## Demo Workflow

The browser demo is part of the repo story, not a throwaway extra.

Important behavior:

- The demo loads synced copies from `demo/examples/`.
- The canonical build path is `./scripts/build-wasm-package.sh --scope gnomes --sync-demo`.
- The demo showcases graph, JSON, DOT, Mermaid, semantic diff, and simulation.
- The loaded example becomes the review baseline.
- `Restore Baseline` resets editor contents to that loaded example.
- `Reset Simulation` resets only runtime simulation state and history.

If example files or WASM exports change, keep the demo and demo README consistent.

## Testing Expectations

Run the narrowest relevant checks first, then broader ones when warranted.

Common commands:

```bash
cargo test
cargo test --all-features
cargo test --test examples
cargo test --test w3c_subset
cargo clippy --all-features
cargo bench --bench statechart
cargo bench --bench rkyv_bench --features rkyv
./scripts/build-wasm-package.sh --scope gnomes --sync-demo
```

When editing examples, demo behavior, README performance claims, or WASM bindings, validate the affected surface directly.

## Documentation Rules

- Keep README claims accurate to the measured benchmark story.
- Keep demo documentation synchronized with actual demo behavior.
- Do not leave stale counts, stale example inventories, or obsolete build steps in instruction files.
- Prefer one canonical explanation over duplicated, drifting copies.

## Instruction File Policy

This repository intentionally keeps agent instruction files tracked.

Policy:

- `AGENTS.md` is the canonical project guidance.
- `.claude/CLAUDE.md` is a thin adapter for Claude-compatible tooling.
- `.github/.copilot-instructions.md` is a thin adapter for Copilot-compatible tooling.
- Do not add these files to `.gitignore` unless the project explicitly decides to stop shipping agent guidance.
- Do not maintain multiple full copies of the same project instructions.

## Default Agent Behavior

When making changes in this repo:

- preserve the crate's role as a document model rather than a runtime engine
- prefer precise, minimal edits
- validate behavior after changes
- keep benchmarks, examples, demo, and docs narratively aligned
- avoid speculative features that blur the crate boundary

License context: MIT OR Apache-2.0.