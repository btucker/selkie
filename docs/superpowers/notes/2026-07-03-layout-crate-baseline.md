# Layout Crate Baseline Notes

## Pre-Migration Eval

Command:

```bash
cargo run --features eval --bin selkie -- eval --type flowchart
```

Baseline result:

- Total diagrams: 33
- Passing: 0
- Errors: 29
- Warnings: 66
- Report: `./eval-report/selkie-eval-0c8964f9/index.html`
- Exit status: 1 due existing flowchart eval failures.

Relevant eval guidance:

- Initialize reference implementations if needed before investigating parity issues.
- Compare Selkie SVGs, reference SVGs, and comparison PNGs from the per-diagram report files.
- For parsing issues, inspect `src/parser/flowchart.rs` against Mermaid's flowchart parser/database.
- For rendering or layout issues, inspect `src/render/flowchart.rs`, `src/types/flowchart.rs`, Mermaid's flow renderer, and Dagre layout behavior.
- The eval report identifies Dagre as the layout reference used by Mermaid flowcharts.

## Reference Implementations

Hydrated submodules:

- `reference-implementations/dagre`: `ba986662394f8f3ed608717194e5958f3386ce01`
- `reference-implementations/mermaid`: `9745f325cb9e1967640f0e85da193a2f820634f1`

Entrypoint notes:

- Dagre `index.js` exposes a narrow public surface: `graphlib`, `layout`, `debug`, `util`, and `version`.
- Dagre `lib/layout.js` builds an internal layout graph by whitelisting graph, node, and edge attributes, runs the layered layout phases, then copies computed positions, ranks, edge points, labels, and graph dimensions back to the input graph.
- Dagre defaults include `ranksep: 50`, `edgesep: 20`, `nodesep: 50`, and `rankdir: tb`.
- Dagre coordinate handling swaps width/height for `LR`/`RL`, reverses y for `BT`/`RL`, and swaps x/y back after positioning.
- Mermaid `packages/mermaid/src/dagre-wrapper/index.js` measures and prepares nodes and edge labels first, adjusts clusters and edges, calls `dagreLayout(graph)`, then positions rendered nodes, clusters, edge paths, and labels from Dagre coordinates.
- Mermaid `packages/mermaid/src/dagre-wrapper/mermaid-graphlib.js` handles cluster descendants, external cluster edges, cluster extraction, and recursive subgraph layout setup before Dagre layout runs.

## Public API RED Check

Command:

```bash
cargo test -p selkie-layout --test public_api
```

Expected failure:

```text
error: package ID specification `selkie-layout` did not match any packages
```

This is the intended RED state for Task 1 because the `selkie-layout` package and workspace crate are created by the next task.
