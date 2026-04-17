# scxml — Live Statechart Editor Demo

**[Live version →](https://gnomesofzurich.github.io/scxml/)** — deployed automatically on push to `main`.

![Settlement workflow with event simulation](SCXML_Demo_01.png)

## Local development

```bash
# One-time setup
cargo install wasm-pack
rustup target add wasm32-unknown-unknown

# Build and serve
./scripts/build-wasm-package.sh --scope gnomes --sync-demo
cd demo && python3 -m http.server 8080
```

## Rebuild after changes

After editing Rust source, re-run from the repo root:

```bash
./scripts/build-wasm-package.sh --scope gnomes --sync-demo
```

Reload the browser page. No server restart needed.

## Features

- Live SCXML XML editing with 300ms debounced re-rendering
- Five output views: Graph (DOT to SVG via viz.js), normalized JSON, DOT source, Mermaid source, and semantic diff against the loaded baseline
- Five built-in examples loaded from the checked-in example set: document lifecycle, NPA, settlement, parallel checks, onboarding approval
- Review-mode diff: edit an example and inspect the semantic change set against the originally loaded SCXML
- Restore Baseline button: restore the editor back to the loaded baseline SCXML in one click
- Reset Simulation button: reset only the current state and transition history without changing the SCXML source
- Status bar shows state/transition count and validation errors
- **Event simulation**: send events, see the current state highlighted in the graph
- **Guard toggle**: switch between "pass all" and "block all" to test guard behavior
- **Transition history**: full log of every transition fired
- All processing runs in-browser, no server needed after initial file serving

The source examples live in [`../examples/`](../examples/). The demo uses synced copies under `demo/examples/`, populated by `./scripts/build-wasm-package.sh --sync-demo`, so the browser UI stays aligned with the checked-in fixtures instead of carrying its own hand-maintained snippets.

## How it works

```
baseline SCXML + edited SCXML --> xmlDiff() [WASM] --> semantic change list

SCXML text --> parseXml() [WASM] --> JSON model
                                       |
            +--------------------------+---------------------------+
            |                          |                           |
       validate()                 toDot() / toMermaid()      flatten()
       [WASM]                     [WASM]                     [WASM]
            |                          |                           |
       status bar      graph + DOT + Mermaid + JSON + Diff   simulation data
```

Event simulation reads the flat transitions from `flatten()` and walks the graph client-side in JavaScript. The yellow highlight on the active state is applied by modifying the SVG stroke after viz.js renders it.
