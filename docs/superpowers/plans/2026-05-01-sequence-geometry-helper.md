# Sequence Geometry Helper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace duplicated sequence SVG geometry helpers in eval and rendering tests with one shared rendered-SVG analysis module.

**Architecture:** Add `src/render/svg/sequence_geometry.rs` as a normal-build SVG inspection helper under `render::svg`, documented as unstable support for eval/tests. Refactor `src/eval/checks.rs` to keep overlap policy while using `SequenceGeometry` for extraction/math, then refactor `tests/sequence_rendering.rs` to use the same helper.

**Tech Stack:** Rust, `roxmltree`, existing Selkie SVG rendering and eval modules, `cargo test --features all-formats`, `cargo clippy --features all-formats -- -D warnings`, and `cargo run --features eval --bin selkie -- eval --type sequence`.

---

## File Structure

- Create `src/render/svg/sequence_geometry.rs`
  - Owns sequence SVG parsing and approximate geometry math.
  - Exposes `SequenceBox`, `SequenceGeometry`, and `SEQUENCE_OVERLAP_TOLERANCE`.
  - Keeps raw `SequenceLine` private.
- Modify `src/render/svg/mod.rs`
  - Add `pub mod sequence_geometry;`.
- Modify `src/eval/checks.rs`
  - Remove private sequence geometry structs/parsers.
  - Keep `check_sequence_overlaps` and allowed-overlap policy.
  - Import and use `SequenceGeometry`.
- Modify `tests/sequence_rendering.rs`
  - Remove duplicated `TestBox`, text measurement, path parsing, lifeline lookup, and fragment frame helpers.
  - Import `selkie::render::svg::sequence_geometry::SequenceGeometry`.
- No generated `docs/images` changes are expected. If eval regenerates attribute-order-only SVG churn, restore it before committing.

---

### Task 0: Reference And Baseline Review

**Files:**
- Read: `reference-implementations/mermaid/packages/mermaid/src/diagrams/sequence/sequenceRenderer.ts`
- Read: `reference-implementations/mermaid/packages/mermaid/src/diagrams/sequence/svgDraw.js`
- Read: `docs/images/reference/*.svg`
- Read: `src/eval/checks.rs`
- Read: `tests/sequence_rendering.rs`

- [ ] **Step 1: Identify and claim the mb issue**

Run:

```bash
mb ready
mb show <mb-id>
mb update <mb-id> --status in_progress
```

Use the existing sequence-rendering issue if it covers this work, likely `se-d9bc527b` from `mb ready`. If no existing issue covers this refactor, create one and use the new ID for Task 6:

```bash
mb create "sequence: share SVG geometry helpers" --description "Remove duplicated sequence SVG geometry parsing between eval overlap checks and sequence rendering tests."
mb update <new-mb-id> --status in_progress
```

Expected: there is one concrete mb ID for this work. Record it and use that same `<mb-id>` in Task 6.

- [ ] **Step 2: Ensure Mermaid reference implementation is available**

Run:

```bash
git submodule update --init reference-implementations/mermaid
```

Expected: `reference-implementations/mermaid/packages/mermaid/src/diagrams/sequence/` exists.

- [ ] **Step 3: Review the relevant Mermaid/reference SVG behavior**

Run:

```bash
rg -n "loopLine|messageText|noteText|actor-line|labelBox|loopText|drawNote|drawLoop|drawMessage" reference-implementations/mermaid/packages/mermaid/src/diagrams/sequence docs/images/reference
```

Inspect the matched Mermaid renderer/SVG sections. Record that this plan is a refactor only: preserve current Selkie geometry semantics and SVG class assumptions; do not change layout behavior.

- [ ] **Step 4: Run required baseline sequence eval**

Run:

```bash
cargo run --features eval --bin selkie -- eval --type sequence
```

Expected: eval may exit nonzero due known broader sequence parity issues. Record the report path and confirm the baseline `sequence_overlap` state with the explicit check from Task 5 Step 4.

- [ ] **Step 5: Confirm current duplicated helper locations**

Run:

```bash
rg -n "struct SequenceBox|struct TestBox|find_lifeline_x_for_actor|find_first_fragment_frame|fragment_boxes_from_lines|path_points" src/eval/checks.rs tests/sequence_rendering.rs
```

Expected: matches in both eval checks and sequence rendering tests.

- [ ] **Step 6: Commit no code**

This task is context gathering only.

---

### Task 1: Add Shared Helper Skeleton And Red Geometry Test

**Files:**
- Create: `src/render/svg/sequence_geometry.rs`
- Modify: `src/render/svg/mod.rs`

- [ ] **Step 1: Write the failing helper test**

Create `src/render/svg/sequence_geometry.rs` with the module docs, public API skeleton, and this test first. Keep methods stubbed with `todo!()` or minimal placeholder return values so the test compiles but fails.

```rust
//! Sequence-diagram SVG geometry helpers.
//!
//! This module is unstable support for eval and integration tests. It inspects
//! rendered SVG output; it is not part of the sequence renderer layout model.

pub const SEQUENCE_OVERLAP_TOLERANCE: f64 = 4.0;

#[derive(Debug, Clone)]
pub struct SequenceBox {
    pub kind: &'static str,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug)]
pub struct SequenceGeometry {
    // Fill in during implementation.
}

impl SequenceBox {
    fn new(
        kind: &'static str,
        label: impl Into<String>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Self {
        Self {
            kind,
            label: label.into(),
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub fn intersects_with_tolerance(&self, other: &Self, tolerance: f64) -> bool {
        todo!("red test first")
    }

    pub fn contains_with_tolerance(&self, inner: &Self, tolerance: f64) -> bool {
        todo!("red test first")
    }
}

impl SequenceGeometry {
    pub fn parse(_svg: &str) -> Option<Self> {
        todo!("red test first")
    }

    pub fn notes(&self) -> &[SequenceBox] {
        todo!("red test first")
    }

    pub fn message_texts(&self) -> &[SequenceBox] {
        todo!("red test first")
    }

    pub fn note_texts(&self) -> &[SequenceBox] {
        todo!("red test first")
    }

    pub fn loop_texts(&self) -> &[SequenceBox] {
        todo!("red test first")
    }

    pub fn actor_boxes(&self) -> &[SequenceBox] {
        todo!("red test first")
    }

    pub fn self_message_path_boxes(&self) -> &[SequenceBox] {
        todo!("red test first")
    }

    pub fn fragments(&self) -> &[SequenceBox] {
        todo!("red test first")
    }

    pub fn fragment_frame_groups(&self) -> &[SequenceBox] {
        todo!("red test first")
    }

    pub fn fragment_headers(&self) -> &[SequenceBox] {
        todo!("red test first")
    }

    pub fn fragment_borders(&self) -> &[SequenceBox] {
        todo!("red test first")
    }

    pub fn text_box_containing(&self, _label: &str) -> Option<SequenceBox> {
        todo!("red test first")
    }

    pub fn note_box_containing(&self, _label: &str) -> Option<SequenceBox> {
        todo!("red test first")
    }

    pub fn actor_box_containing(&self, _label: &str) -> Option<SequenceBox> {
        todo!("red test first")
    }

    pub fn lifeline_x_for_actor(&self, _label: &str) -> Option<f64> {
        todo!("red test first")
    }

    pub fn self_message_path_box(&self) -> Option<SequenceBox> {
        todo!("red test first")
    }

    pub fn first_fragment_frame(&self) -> Option<SequenceBox> {
        todo!("red test first")
    }

    pub fn first_fragment_frame_group(&self) -> Option<SequenceBox> {
        todo!("red test first")
    }

    pub fn svg_width(&self) -> Option<f64> {
        todo!("red test first")
    }

    pub fn svg_visible_right(&self) -> Option<f64> {
        todo!("red test first")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sequence_svg_geometry_once() {
        let svg = r##"<svg width="300" height="220" viewBox="-10 0 300 220" xmlns="http://www.w3.org/2000/svg">
          <g class="actor">
            <rect class="actor actor-box" x="0" y="0" width="150" height="65"/>
            <text class="actor actor-box" x="75" y="32.5" text-anchor="middle">Alice</text>
          </g>
          <line class="actor-line" x1="75" y1="65" x2="75" y2="180"/>
          <g>
            <line class="loopLine" x1="40" y1="80" x2="220" y2="80"/>
            <line class="loopLine" x1="220" y1="80" x2="220" y2="160"/>
            <line class="loopLine" x1="40" y1="160" x2="220" y2="160"/>
            <line class="loopLine" x1="40" y1="80" x2="40" y2="160"/>
            <text class="loopText" x="130" y="98" text-anchor="middle">[Healthcheck]</text>
          </g>
          <path class="message-line" marker-end="url(#arrow-filled)" d="M 75 110 L 115 110 L 115 140 L 75 140"/>
          <text class="message-label" x="120" y="125" text-anchor="start">Fight</text>
          <g class="note">
            <rect class="note" x="95" y="165" width="150" height="39"/>
            <text class="noteText" x="170" y="170" text-anchor="middle" dominant-baseline="middle" dy="1em">Rational</text>
          </g>
        </svg>"##;

        let geometry = SequenceGeometry::parse(svg).expect("geometry");

        assert_eq!(geometry.svg_width(), Some(300.0));
        assert_eq!(geometry.svg_visible_right(), Some(290.0));
        assert_eq!(geometry.actor_box_containing("Alice").expect("actor").x, 0.0);
        assert_eq!(geometry.lifeline_x_for_actor("Alice"), Some(75.0));
        assert_eq!(geometry.text_box_containing("Fight").expect("message").kind, "message-label");
        assert_eq!(geometry.note_box_containing("Rational").expect("note").width, 150.0);
        assert!(geometry.self_message_path_box().expect("self path").width >= 40.0);
        assert_eq!(geometry.first_fragment_frame_group().expect("frame").x, 40.0);
        assert!(geometry.fragment_headers().len() == 1);
        assert!(geometry.fragment_borders().len() == 4);
    }
}
```

- [ ] **Step 2: Expose the module**

Modify `src/render/svg/mod.rs`:

```rust
pub mod sequence_geometry;
```

- [ ] **Step 3: Run the red test**

Run:

```bash
cargo test --features all-formats render::svg::sequence_geometry::tests::parses_sequence_svg_geometry_once -- --nocapture
```

Expected: the test fails because parsing/intersection methods are not implemented yet. If it fails to compile for a typo, fix the typo and rerun until the failure is from missing behavior.

- [ ] **Step 4: Commit nothing yet**

Do not commit until Task 2 makes the helper test pass.

---

### Task 2: Implement Shared Geometry Extraction

**Files:**
- Modify: `src/render/svg/sequence_geometry.rs`

- [ ] **Step 1: Implement data storage**

Use this shape. Keep `SequenceLine` private.

```rust
const SEQUENCE_CHAR_WIDTH: f64 = 8.0;
const SEQUENCE_LINE_HEIGHT: f64 = 18.0;

#[derive(Debug, Clone, Copy)]
struct SequenceLine {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

#[derive(Debug)]
pub struct SequenceGeometry {
    svg_width: Option<f64>,
    view_box_min_x: f64,
    notes: Vec<SequenceBox>,
    message_texts: Vec<SequenceBox>,
    note_texts: Vec<SequenceBox>,
    loop_texts: Vec<SequenceBox>,
    actor_boxes: Vec<SequenceBox>,
    actor_lifelines: Vec<SequenceLine>,
    self_message_path_boxes: Vec<SequenceBox>,
    aggregate_fragment_frame: Option<SequenceBox>,
    fragments: Vec<SequenceBox>,
    fragment_frame_groups: Vec<SequenceBox>,
    fragment_headers: Vec<SequenceBox>,
    fragment_borders: Vec<SequenceBox>,
}
```

- [ ] **Step 2: Move geometry math from existing helpers**

Move the current logic, preserving behavior:

- From `src/eval/checks.rs`: `SequenceBox`, `SequenceLine`, `rect_sequence_box`, `text_sequence_box`, `text_box_y`, `has_middle_baseline`, `parse_text_dy`, `line_coords`, `fragment_boxes_from_lines`, `aggregate_fragment_box`, `fragment_border_boxes`, `has_class`, `parse_attr`, `node_text`.
- From `tests/sequence_rendering.rs`: actor lookup behavior, lifeline lookup behavior, `svg_width`, `svg_visible_right`, self-message `path_points`, `box_from_points`, aggregate frame behavior, first 4-line group behavior.

Keep these semantics:

- `first_fragment_frame()` aggregates all `.loopLine` coordinates.
- `first_fragment_frame_group()` returns the first group with exactly four direct child `.loopLine` elements.
- Text boxes use `8.0` px per character and `18.0` px line height.
- `noteText` with middle/central baseline and `dy` adjusts y as before.
- `self_message_path_box()` selects the first message path with a marker-end and at least three parsed path points.

- [ ] **Step 3: Add accessors and lookup methods**

Implement the API from the spec:

```rust
pub fn notes(&self) -> &[SequenceBox] { &self.notes }
pub fn message_texts(&self) -> &[SequenceBox] { &self.message_texts }
pub fn note_texts(&self) -> &[SequenceBox] { &self.note_texts }
pub fn loop_texts(&self) -> &[SequenceBox] { &self.loop_texts }
pub fn actor_boxes(&self) -> &[SequenceBox] { &self.actor_boxes }
pub fn self_message_path_boxes(&self) -> &[SequenceBox] { &self.self_message_path_boxes }
pub fn fragments(&self) -> &[SequenceBox] { &self.fragments }
pub fn fragment_frame_groups(&self) -> &[SequenceBox] { &self.fragment_frame_groups }
pub fn fragment_headers(&self) -> &[SequenceBox] { &self.fragment_headers }
pub fn fragment_borders(&self) -> &[SequenceBox] { &self.fragment_borders }
```

Lookup methods should return cloned `SequenceBox` values to keep callers from mutating stored collections.

`SequenceBox::new` should stay private to the module. External callers should use the public fields or values returned by `SequenceGeometry`; do not add a public constructor unless a later test proves it is required.

- [ ] **Step 4: Run helper test green**

Run:

```bash
cargo test --features all-formats render::svg::sequence_geometry::tests::parses_sequence_svg_geometry_once -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run formatting**

Run:

```bash
cargo fmt
cargo clippy --features all-formats -- -D warnings
```

Expected: formatting completes and clippy passes with warnings denied.

- [ ] **Step 6: Commit shared helper**

Run:

```bash
git add src/render/svg/mod.rs src/render/svg/sequence_geometry.rs
git commit -m "feat(sequence): add shared svg geometry helper"
```

---

### Task 3: Refactor Eval Overlap Checks Onto Shared Helper

**Files:**
- Modify: `src/eval/checks.rs`
- Test: existing `src/eval/checks.rs` sequence overlap tests

- [ ] **Step 1: Confirm current eval tests pass before refactor**

Run:

```bash
cargo test --features eval sequence_overlap -- --nocapture
```

Expected: existing sequence overlap tests pass before code movement. If this fails before editing, stop and diagnose the baseline.

- [ ] **Step 2: Replace eval-private extraction with `SequenceGeometry`**

In `src/eval/checks.rs`, import:

```rust
use crate::render::svg::sequence_geometry::{
    SequenceBox, SequenceGeometry, SEQUENCE_OVERLAP_TOLERANCE,
};
```

Change `check_sequence_overlaps` to:

```rust
pub fn check_sequence_overlaps(selkie: &SvgStructure, issues: &mut Vec<Issue>) {
    let Some(geometry) = SequenceGeometry::parse(&selkie.raw_svg) else {
        return;
    };

    for message in geometry.message_texts() {
        for note in geometry.notes() {
            report_overlap_if_intersects(message, note, issues);
        }

        for (fragment, header) in geometry.fragments().iter().zip(geometry.fragment_headers()) {
            if message.intersects_with_tolerance(header, SEQUENCE_OVERLAP_TOLERANCE)
                && !is_message_text_inside_fragment_body(message, fragment, header)
            {
                push_sequence_overlap(message, header, issues);
            }
        }
    }

    // Keep the current noteText/header, loopText/note, note/header, and note/border policy.
}
```

Keep these policy helpers in `checks.rs`:

- `report_overlap_if_intersects`
- `push_sequence_overlap`
- `is_note_text_inside_own_note`
- `is_loop_text_inside_own_header`
- `is_message_text_inside_fragment_body`
- `contains_with_tolerance` can become a direct call to `SequenceBox::contains_with_tolerance`.

If existing eval tests construct boxes with `SequenceBox::new`, change those tests to use struct literals:

```rust
let a = SequenceBox {
    kind: "message",
    label: "A".to_string(),
    x: 10.0,
    y: 10.0,
    width: 40.0,
    height: 20.0,
};
```

Delete eval-private geometry helpers after the replacement:

- private `SequenceBox`
- private `SequenceLine`
- `rect_sequence_box`
- `text_sequence_box`
- `text_box_y`
- `has_middle_baseline`
- `parse_text_dy`
- `line_coords`
- `fragment_boxes_from_lines`
- `aggregate_fragment_box`
- `fragment_border_boxes`
- local `has_class`
- local `parse_attr`
- local `node_text`

Do not move overlap policy into `sequence_geometry.rs`.

- [ ] **Step 3: Run eval overlap tests**

Run:

```bash
cargo test --features eval sequence_overlap -- --nocapture
```

Expected: PASS. If issue messages change, keep the existing wording unless a test reveals it was depending on duplicated helper bugs.

- [ ] **Step 4: Run full eval checks unit target**

Run:

```bash
cargo test --features eval eval::checks -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit eval refactor**

Run required pre-commit gates:

```bash
cargo fmt
cargo clippy --features all-formats -- -D warnings
```

Expected: PASS.

Run:

```bash
git add src/eval/checks.rs src/render/svg/sequence_geometry.rs
git commit -m "refactor(eval): use shared sequence geometry"
```

---

### Task 4: Refactor Sequence Rendering Tests Onto Shared Helper

**Files:**
- Modify: `tests/sequence_rendering.rs`
- Test: `tests/sequence_rendering.rs`

- [ ] **Step 1: Confirm sequence rendering tests pass before refactor**

Run:

```bash
cargo test --features all-formats --test sequence_rendering
```

Expected: 13 tests pass before editing.

- [ ] **Step 2: Import the shared helper**

At the top of `tests/sequence_rendering.rs`, add:

```rust
use selkie::render::svg::sequence_geometry::{SequenceBox, SequenceGeometry};
```

- [ ] **Step 3: Replace local helper functions incrementally**

Remove these local definitions after replacing all call sites:

- `TEST_CHAR_WIDTH`
- `TEST_LINE_HEIGHT`
- `TestBox`
- `parse_num`
- `find_text_box`
- `node_text`
- `text_box_y`
- `has_middle_baseline`
- `parse_text_dy`
- `find_note_box_containing`
- `find_actor_box_containing`
- `find_lifeline_x_for_actor`
- `svg_visible_right`
- `svg_width`
- `find_self_message_path_box`
- `path_points`
- `box_from_points`
- `find_first_fragment_frame`
- `find_first_fragment_frame_group`

Use small wrappers only if they keep tests readable and do not duplicate geometry logic. Preferred wrappers:

```rust
fn geometry(svg: &str) -> SequenceGeometry {
    SequenceGeometry::parse(svg).expect("valid sequence svg geometry")
}

fn text_box(svg: &str, label: &str) -> SequenceBox {
    geometry(svg)
        .text_box_containing(label)
        .unwrap_or_else(|| panic!("missing text {label}"))
}
```

If repeated parsing makes a test awkward, parse once inside that test:

```rust
let geometry = SequenceGeometry::parse(&svg).expect("geometry");
let note = geometry.note_box_containing("Rational").expect("note");
```

- [ ] **Step 4: Preserve test assertion semantics**

Map old helpers to new calls exactly:

- `find_text_box(&svg, "...")` -> `geometry.text_box_containing("...").expect(...)`
- `find_note_box_containing(&svg, "...")` -> `geometry.note_box_containing("...").expect(...)`
- `find_actor_box_containing(&svg, "...")` -> `geometry.actor_box_containing("...").expect(...)`
- `find_lifeline_x_for_actor(&svg, "...")` -> `geometry.lifeline_x_for_actor("...").expect(...)`
- `find_self_message_path_box(&svg)` -> `geometry.self_message_path_box().expect(...)`
- `find_first_fragment_frame(&svg)` -> `geometry.first_fragment_frame().expect(...)`
- `find_first_fragment_frame_group(&svg)` -> `geometry.first_fragment_frame_group().expect(...)`
- `svg_width(&svg)` -> `geometry.svg_width().expect(...)`
- `svg_visible_right(&svg)` -> `geometry.svg_visible_right().expect(...)`

- [ ] **Step 5: Run sequence rendering tests**

Run:

```bash
cargo test --features all-formats --test sequence_rendering
```

Expected: 13 tests pass.

- [ ] **Step 6: Commit test refactor**

Run required pre-commit gates:

```bash
cargo fmt
cargo clippy --features all-formats -- -D warnings
```

Expected: PASS.

Run:

```bash
git add tests/sequence_rendering.rs src/render/svg/sequence_geometry.rs
git commit -m "refactor(sequence): share rendering test geometry"
```

---

### Task 5: Verify Full Behavior And Eval

**Files:**
- Read: `eval-report/selkie-eval-*/sequence/*_comparison.json`
- Potentially restore generated SVG churn under `docs/images/`

- [ ] **Step 1: Run formatting and clippy**

Run:

```bash
cargo fmt
cargo clippy --features all-formats -- -D warnings
```

Expected: PASS.

- [ ] **Step 2: Run full tests**

Run:

```bash
cargo test --features all-formats
```

Expected: PASS, with the existing ignored flowchart integration test still ignored.

- [ ] **Step 3: Run sequence eval**

Run:

```bash
cargo run --features eval --bin selkie -- eval --type sequence
```

Expected: eval may still exit nonzero due known broader sequence parity issues. It must not introduce new `sequence_overlap` findings.

- [ ] **Step 4: Check for sequence overlap warnings in latest report**

Replace `<report>` with the report directory printed by eval:

```bash
test -d <report>/sequence
ls <report>/sequence/*_comparison.json >/dev/null
rg -n "sequence_overlap" <report>/sequence/*_comparison.json
test $? -eq 1
```

Expected: `test` and `ls` pass, `rg` prints no matches, and `test $? -eq 1` passes because `rg` returns 1 when it finds no matches.

- [ ] **Step 5: Restore generated SVG churn if needed**

If eval changed `docs/images/*.svg` only due attribute ordering or reference regeneration, restore those files:

```bash
git restore docs/images
```

Expected: `git status --short` shows only intentional source/doc changes.

- [ ] **Step 6: Commit any final cleanup**

If Task 5 required source edits after prior commits:

```bash
cargo fmt
cargo clippy --features all-formats -- -D warnings
git add <changed-source-files>
git commit -m "chore(sequence): finalize shared geometry helper"
```

If no source edits were needed, do not make an empty commit.

---

### Task 6: Issue Lifecycle, Sync, Push, And Handoff

**Files:**
- No code changes expected.

- [ ] **Step 1: Review issue state and file follow-ups**

Run:

```bash
mb show <mb-id>
```

If any remaining work is discovered during implementation, file follow-up issues before closing this work:

```bash
mb create "<short follow-up title>" --description "<specific remaining work and context>"
```

Expected: all known remaining work is either complete or tracked.

- [ ] **Step 2: Close completed issue**

Run:

```bash
mb close <mb-id>
```

Expected: the mb issue for this work is closed. If the issue is already closed, note that in the handoff.

- [ ] **Step 3: Sync issue tracker**

Run:

```bash
mb sync
```

Expected: `Changes synced.`

- [ ] **Step 4: Push branch**

Run:

```bash
git pull --rebase
mb sync
git push
git status
```

Expected: branch is up to date with `origin/fix-issue-202`, working tree clean.

- [ ] **Step 5: Handoff summary**

Report:

- commits created
- tests and eval commands run
- latest sequence eval report path
- whether `sequence_overlap` appears in the latest report
- any remaining follow-up issues that should be tracked in `mb`
