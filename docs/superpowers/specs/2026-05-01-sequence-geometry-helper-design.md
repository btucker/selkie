# Sequence Geometry Helper Design

## Context

The issue 202 work added sequence-specific SVG geometry checks in `src/eval/checks.rs` and sequence rendering assertions in `tests/sequence_rendering.rs`. Those two files now carry parallel implementations of the same ideas:

- approximate text boxes from SVG `<text>` nodes
- note and actor rectangle lookup
- path point parsing for self-message shapes
- fragment frame extraction from `.loopLine` elements
- tolerance-based box intersection

That duplication is a maintenance risk. A future fix to SVG text positioning, baseline handling, or fragment detection could update eval but leave tests using stale geometry, or vice versa.

## Goals

- Make eval and sequence rendering tests use one sequence SVG geometry implementation.
- Keep the helper focused on inspection of rendered SVG, not production layout.
- Preserve current overlap detection behavior and current sequence rendering test coverage.
- Avoid adding stable renderer API surface to the crate.

## Non-Goals

- Rewriting the renderer layout model.
- Making a generic SVG geometry engine for all diagram types.
- Improving eval performance beyond removing duplicate helper logic.
- Changing generated `docs/images` SVGs.

## Architecture

Add a new SVG inspection module compiled in normal library builds:

```text
src/render/svg/sequence_geometry.rs
```

This is intentionally not part of the renderer layout model. It is a rendered-SVG analysis helper used by eval and integration tests. Because `tests/sequence_rendering.rs` is an integration test, crate-private visibility is not enough; the module must be reachable through the crate boundary. Expose it under the existing `render::svg` namespace and document it as unstable internal support, not as a compatibility promise.

The module should expose minimal public types and functions for sequence SVG analysis:

- `SequenceBox`: x, y, width, height, kind, and label.
- `SequenceGeometry`: parsed collections for notes, message texts, note texts, loop texts, actor boxes, actor lifelines, self-message path boxes, fragments, grouped fragment frames, fragment headers, fragment borders, and parsed SVG root metrics.
- Lookup helpers used by tests, such as text boxes by label, note boxes by contained label, actor boxes by label, self-message path boxes, fragment frame boxes, SVG width, and visible right edge.

`SequenceLine` should stay a private implementation detail used while deriving fragment frames. Callers should consume derived `SequenceBox` values instead of raw line coordinates.

`src/eval/checks.rs` should call `SequenceGeometry::parse(&selkie.raw_svg)` and perform overlap policy decisions from the parsed geometry. The helper should own SVG extraction and geometry math; `checks.rs` should own which overlaps become eval issues.

`tests/sequence_rendering.rs` should import the same helper through the crate when running with `--features all-formats`. The tests should stop defining their own `TestBox`, text measurement, baseline handling, path parser, lifeline lookup, and fragment frame parser.

## API Shape

Keep the API intentionally small. These items need `pub` visibility because integration tests use the crate as an external dependency, but the module docs should describe them as unstable SVG analysis support:

```rust
pub const SEQUENCE_OVERLAP_TOLERANCE: f64 = 4.0;

pub struct SequenceBox {
    pub kind: &'static str,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub struct SequenceGeometry { ... }

impl SequenceBox {
    pub fn right(&self) -> f64;
    pub fn bottom(&self) -> f64;
    pub fn intersects_with_tolerance(&self, other: &Self, tolerance: f64) -> bool;
    pub fn contains_with_tolerance(&self, inner: &Self, tolerance: f64) -> bool;
}

impl SequenceGeometry {
    pub fn parse(svg: &str) -> Option<Self>;
    pub fn notes(&self) -> &[SequenceBox];
    pub fn message_texts(&self) -> &[SequenceBox];
    pub fn note_texts(&self) -> &[SequenceBox];
    pub fn loop_texts(&self) -> &[SequenceBox];
    pub fn actor_boxes(&self) -> &[SequenceBox];
    pub fn self_message_path_boxes(&self) -> &[SequenceBox];
    pub fn fragments(&self) -> &[SequenceBox];
    pub fn fragment_frame_groups(&self) -> &[SequenceBox];
    pub fn fragment_headers(&self) -> &[SequenceBox];
    pub fn fragment_borders(&self) -> &[SequenceBox];
    pub fn text_box_containing(&self, label: &str) -> Option<SequenceBox>;
    pub fn note_box_containing(&self, label: &str) -> Option<SequenceBox>;
    pub fn actor_box_containing(&self, label: &str) -> Option<SequenceBox>;
    pub fn lifeline_x_for_actor(&self, label: &str) -> Option<f64>;
    pub fn self_message_path_box(&self) -> Option<SequenceBox>;
    pub fn first_fragment_frame(&self) -> Option<SequenceBox>;
    pub fn first_fragment_frame_group(&self) -> Option<SequenceBox>;
    pub fn svg_width(&self) -> Option<f64>;
    pub fn svg_visible_right(&self) -> Option<f64>;
}
```

Do not expose `SequenceLine` unless a caller needs it. It can stay private if `SequenceGeometry` exposes derived fragment boxes, headers, and borders.

`first_fragment_frame()` should preserve the current test helper behavior: aggregate all `.loopLine` coordinates into one enclosing box. `first_fragment_frame_group()` should preserve the stricter first four-line group behavior. Both are needed because current tests use the aggregate helper for broad frame coverage and the grouped helper when they need one concrete fragment frame.

## Testing Strategy

Follow TDD:

1. Add a focused unit test for the shared helper that parses a small sequence SVG and finds the same text, note, actor, self-message, and fragment boxes currently parsed by duplicated helpers.
2. Watch that test fail before moving helper code.
3. Move the extraction logic into `sequence_geometry.rs` until the helper test passes.
4. Refactor `check_sequence_overlaps` to use `SequenceGeometry`; run existing eval overlap tests.
5. Refactor `tests/sequence_rendering.rs` to use the shared helper; run `cargo test --features all-formats --test sequence_rendering`.
6. Run full quality gates and sequence eval.

Keep policy tests in `src/eval/checks.rs`. `sequence_geometry.rs` tests should cover parsing and geometry math only; eval tests remain responsible for warning policy, allowed-overlap exceptions, and issue messages.

## Risks

- The helper is public enough for integration tests. Keep it in a clearly named SVG analysis namespace and avoid re-exporting it from the crate root.
- Moving geometry code can accidentally change overlap semantics. The first implementation should preserve current math and only centralize ownership.
- The helper may tempt broader reuse. Keep it sequence-only until another diagram type has a concrete need.
