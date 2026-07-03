# Layout Crate Refactor Design

## Context

GitHub issue 201 asks whether Selkie's layout functionality can be separated into its own crate, or at least partitioned behind tighter feature flags. The motivating use case is a non-Selkie consumer who wants the graph layout components, especially the coordinate-producing layout algorithm, without pulling in parsing, rendering, CLI, diagram-specific code, or other Mermaid-focused modules.

The current `src/layout` module is close to a library boundary, but it still depends on Selkie's crate-level error type and mixes pure layout concerns with Selkie adapter and sizing conveniences. Internal Selkie consumers import `crate::layout` broadly across SVG rendering, ASCII rendering, diagram adapters, eval checks, and tests, so the refactor must preserve compatibility while creating a clean public API.

Reference implementation directories exist as git submodules under `reference-implementations/`, but they are not initialized in this worktree. Implementation planning must include hydrating and reviewing the relevant Dagre and Mermaid reference code before algorithm-sensitive changes.

## Goals

- Create a public crate optimized for non-Selkie users.
- Let users construct a graph with explicit node dimensions and edges, run layout, then inspect node positions and routed edge points.
- Expose a stable high-level layout API.
- Expose a stable curated `dagre` expert API without freezing every current internal phase module or temporary algorithm field.
- Keep Selkie's existing `selkie::layout` import path working through a compatibility facade.
- Avoid changing layout behavior during the extraction.

## Non-Goals

- Do not design a multi-engine layout ecosystem yet.
- Do not move Mermaid-specific diagram adapters into the public layout crate.
- Do not move text measurement or font lookup into the public layout crate initially.
- Do not expose every current file under `src/layout/dagre` as public stable API.
- Do not combine this refactor with unrelated layout parity fixes.

## Architecture

Add a Cargo workspace crate at `crates/selkie-layout`.

The layout crate owns algorithm and graph-layout concepts:

- `LayoutGraph`
- `LayoutNode`
- `LayoutEdge`
- `LayoutOptions`
- `LayoutDirection`
- `LayoutRanker`
- `Point`
- `Padding`
- `NodeShape`
- `layout(graph) -> LayoutResult<LayoutGraph>`
- curated `selkie_layout::dagre` expert module

The main `selkie-rs` crate depends on `selkie-layout` and keeps a `selkie::layout` compatibility facade. That facade re-exports layout crate types and hosts Selkie-specific adapter and sizing code:

- `ToLayoutGraph`
- `NodeSizeConfig`
- `SizeEstimator`
- `CharacterSizeEstimator`
- `FontdueSizeEstimator`
- `create_size_estimator`

`NodeShape` stays in `selkie-layout` because shape metadata affects edge intersection and routing. It should be documented as layout geometry metadata rather than Mermaid rendering metadata.

## Public High-Level API

The high-level API should support the issue-201 use case:

```rust
use selkie_layout::{
    layout, LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode, LayoutOptions,
};

let mut graph = LayoutGraph::new("example").with_options(LayoutOptions {
    direction: LayoutDirection::TopToBottom,
    node_spacing: 50.0,
    layer_spacing: 50.0,
    ..Default::default()
});

graph.add_node(LayoutNode::new("A", 80.0, 40.0));
graph.add_node(LayoutNode::new("B", 80.0, 40.0));
graph.add_edge(LayoutEdge::new("A_to_B", "A", "B"));

let result = layout(graph)?;

for node in result.nodes() {
    println!("{}: {:?}, {:?}", node.id(), node.position(), node.size());
}

for edge in result.edges() {
    println!("{} routed through {:?}", edge.id(), edge.points());
}
```

The public crate should favor stable constructors, builders, accessors, and iterators over broad direct field access. The compatibility facade can reduce churn in Selkie by continuing to support existing internal usage patterns where practical, but the new public API should not casually freeze internal representation details.

## Public Dagre Expert API

Expose `selkie_layout::dagre` as a stable public expert API, but make it curated.

The API should provide stable graph construction, configuration, and layout entrypoints:

```rust
use selkie_layout::dagre::{DagreConfig, DagreGraph, RankDir, Ranker};

let mut graph = DagreGraph::new();
graph.set_node("A", 80.0, 40.0);
graph.set_node("B", 80.0, 40.0);
graph.set_edge("A", "B");

selkie_layout::dagre::layout(&mut graph, &DagreConfig::default());
```

Do not expose low-level phase modules such as normalization, ordering, parent dummy chains, self-edge handling, or Brandes-Kopf implementation files as semver-stable public modules. Internally, the curated expert API can wrap the current implementation structs and fields. This protects algorithm internals while still serving users who need direct Dagre-style graph layout.

## Error Handling

`selkie-layout` owns a layout-specific error type:

- `LayoutError`
- `LayoutResult<T> = Result<T, LayoutError>`

Initial error variants should cover:

- duplicate node IDs
- missing edge endpoints
- invalid parent relationships or parent cycles
- invalid dimensions or spacing values
- layout execution failure if positions or routes cannot be produced
- unsupported expert API operations that are intentionally outside the first stable surface

Selkie's existing `MermaidError::LayoutError` can convert from `selkie_layout::LayoutError` at the compatibility boundary.

## Migration Plan

Do the migration in one coordinated implementation pass rather than landing it as multiple staged refactor PRs.

The single-pass implementation should:

1. Initialize or otherwise hydrate reference implementation submodules before algorithm-sensitive work.
2. Create the workspace and `crates/selkie-layout`.
3. Move algorithm-owned layout code into the new crate.
4. Replace `crate::error::Result` in layout code with `LayoutResult`.
5. Introduce `selkie::layout` as a compatibility facade.
6. Keep Selkie-specific adapters and sizing utilities in the main crate.
7. Update all affected internal imports and call sites required by the extraction.
8. Add crate-level docs and examples for non-Selkie users.
9. Add tests proving the standalone crate works without Selkie parser or renderer modules.
10. Run quality gates and at least one eval smoke test to confirm output behavior did not change.

This is intentionally an all-at-once migration. The implementation plan should still identify checkpoints and validation commands, but those checkpoints are for execution control inside one migration, not for splitting the refactor into phased deliverables.

## Testing

Testing should include:

- moved layout unit tests inside `selkie-layout`
- a standalone integration test using only public `selkie-layout` APIs
- a compatibility test that `selkie::layout` re-exports still work
- representative structural regression tests for node positions, edge routes, compound graph support, and edge labels
- limited exact-coordinate assertions only for fixtures expected to remain stable through extraction

Required project gates after implementation:

```bash
cargo fmt
cargo clippy --features all-formats -- -D warnings
cargo test --features all-formats
cargo run --features eval --bin selkie -- eval --type flowchart
```

## Risks

- Exposing too much of the current Dagre internals would turn future parity fixes into breaking API changes.
- Private-field API hygiene may conflict with current Selkie internals that directly mutate layout structs.
- Doing the migration all at once increases merge-conflict and regression risk, so the implementation needs tight mechanical steps and frequent local verification.
- Leaving text sizing outside `selkie-layout` means the first public crate expects explicit node dimensions; this is a reasonable initial contract but should be clear in docs.
- Workspace and package naming need care because the existing package is `selkie-rs` while the library name is `selkie`.
- Reference submodules are not currently initialized, so planning must not assume local reference source is available until that is fixed.

## Open Decisions For Implementation Planning

- Exact package name: likely `selkie-layout`.
- Whether the public crate starts at the same version as `selkie-rs` or starts independently.
- How much direct field access to preserve in the compatibility facade versus wrapper methods in the new crate.
- Whether to add a root workspace-only `Cargo.toml` structure or keep the existing package as the workspace root package.
