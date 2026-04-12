# scxml — Live Statechart Editor Demo

A single-page HTML demo that runs entirely client-side via WebAssembly.
Edit SCXML in the left pane, see the statechart rendered as a graph in real-time.
Send events in the right pane to simulate transitions through the state machine.

![Settlement workflow with event simulation](SCXML_Demo_01.png)

## Prerequisites

```bash
# Install wasm-pack (one time)
cargo install wasm-pack

# Ensure the wasm32 target is installed (one time)
rustup target add wasm32-unknown-unknown
```

## Build & Run

From the repo root (`scxml/`):

```bash
# Build the WASM package and sync the demo copy
./scripts/build-wasm-package.sh --scope gnomes --sync-demo

# Serve locally (any static file server works)
cd demo
python3 -m http.server 8080
```

Then open [http://localhost:8080](http://localhost:8080).

## Rebuild after changes

After editing Rust source, re-run from the repo root:

```bash
./scripts/build-wasm-package.sh --scope gnomes --sync-demo
```

Reload the browser page. No server restart needed.

## Features

- Live SCXML XML editing with 300ms debounced re-rendering
- Three output views: Graph (DOT to SVG via viz.js), DOT source, Mermaid source
- Four built-in examples in the demo: Simple, NPA, Settlement, Parallel
- Status bar shows state/transition count and validation errors
- **Event simulation**: send events, see the current state highlighted in the graph
- **Guard toggle**: switch between "pass all" and "block all" to test guard behavior
- **Transition history**: full log of every transition fired
- All processing runs in-browser, no server needed after initial file serving

The full set of example files lives in [`../examples/`](../examples/) and includes
`new_product_approval.scxml`, `document_lifecycle.scxml`, `settlement.scxml`,
`onboarding_approval.scxml`, and `parallel_checks.scxml` (plus matching XState JSON
for two of them). The demo embeds shortened snippets — load any full example by
pasting its contents into the editor pane.

## How it works

```
SCXML text --> parseXml() [WASM] --> JSON model
                                       |
                    +------------------+------------------+
                    |                  |                  |
              validate()         toDot()           flatten()
              [WASM]             [WASM]            [WASM]
                    |                  |                  |
              status bar        viz.js --> SVG     Mermaid text
                                       |
                                 graph pane
                                 + state highlighting
```

Event simulation reads the flat transitions from `flatten()` and walks the
graph client-side in JavaScript. The yellow highlight on the active state
is applied by modifying the SVG stroke after viz.js renders it.
