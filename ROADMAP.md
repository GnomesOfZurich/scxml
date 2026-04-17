# scxml — Roadmap

> For what has been built, see [README.md](README.md).

---

## Considered and deferred

- **Make `serde` an optional dependency.** Invasive for narrow benefit. Revisit if
  someone requests it.

- **SmallVec for transitions/children.** Evaluated and rejected. `Transition` is ~200
  bytes and `Action` is ~100+ bytes, so inlining even one element in a SmallVec bloats
  the parent struct more than a Vec pointer (24 bytes). CompactString SSO already
  handles identifier allocation.

- **String interning.** Evaluated and rejected. All identifiers in real statecharts
  (state IDs, event names, guards, targets) are under 24 bytes, fully covered by
  CompactString SSO (zero heap allocations). Even the most complex instrument
  lifecycle (structured notes, convertibles, OTC derivatives) tops out at ~150 states.
  Interning would save ~300 bytes total on a 50-state chart.

- **Parallel validation (rayon).** The parallelisation boundary is across charts, not
  within a single chart. No single instrument lifecycle exceeds a few hundred states.
  The real scale is thousands of independent charts processed concurrently (batch
  onboarding, portfolio validation), which parallelises trivially at the caller level
  with `par_iter()`; no crate changes needed.

---

## Non-goals

- **Runtime interpreter for production** &larr; compiled native types do that.
- **ECMAScript evaluation** &larr; no runtime interpreter. `<script>` content is stored verbatim but never evaluated. Guards are named predicate references.
- **`<invoke>` / actor spawning** &larr; actor lifecycle belongs in ECS, not the statechart.
- **Full W3C compliance** &larr; we implement the subset that matters, not the full spec.
- **PlantUML export** &larr; DOT and Mermaid cover visualization.
- **Python bindings** &larr; the differentiators don't translate well to Python.
