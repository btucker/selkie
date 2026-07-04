# FLOW Spec Pilot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a narrow `FLOW-*` spec annotation pilot with generated `SPECS.md`.

**Architecture:** A Python generator scans source and test files for `@spec` annotations, validates duplicate IDs, and writes a stable Markdown index. Existing flowchart tests receive a few initial `FLOW-*` doc comments so compatibility requirements become grep-able and reportable.

**Tech Stack:** Rust tests for product behavior, Python `unittest` for the generator, Markdown for generated specs.

---

### Task 1: Spec Generator

**Files:**
- Create: `tests/test_generate_specs.py`
- Create: `scripts/generate-specs.py`
- Create: `SPECS.md`

- [x] **Step 1: Write failing tests**

Add Python `unittest` coverage for extracting `@spec FLOW-*` annotations, rendering grouped Markdown, duplicate ID rejection, and stale `SPECS.md` check behavior.

- [x] **Step 2: Run test to verify it fails**

Run: `python3 -m unittest tests/test_generate_specs.py`

Expected: FAIL because `scripts/generate-specs.py` does not exist yet.

- [x] **Step 3: Implement generator**

Implement deterministic scanning, Markdown rendering, duplicate validation, `--check`, and normal write mode.

- [x] **Step 4: Run test to verify it passes**

Run: `python3 -m unittest tests/test_generate_specs.py`

Expected: PASS.

### Task 2: Initial FLOW Requirements

**Files:**
- Modify: `src/diagrams/flowchart/parser.rs`
- Modify: `src/render/flowchart.rs`
- Modify: `tests/flowchart_edge_label_truncation.rs`
- Modify: `SPECS.md`

- [x] **Step 1: Add `FLOW-*` doc comments**

Annotate existing tests covering subgraph title parsing, class styling, subgraph direction, rendered edge paths, and ASCII edge-label preservation.

- [x] **Step 2: Regenerate specs**

Run: `python3 scripts/generate-specs.py`

Expected: `SPECS.md` lists the initial `FLOW-*` requirements grouped under `FLOW`.

- [x] **Step 3: Verify focused tests**

Run: `python3 -m unittest tests/test_generate_specs.py`

Expected: PASS.

Run: `cargo test --features all-formats flowchart -- --nocapture`

Expected: PASS.

### Task 3: Quality Gates and Commit

**Files:**
- All changed files from Tasks 1-2.

- [x] **Step 1: Run formatters and checks**

Run: `cargo fmt`

Run: `cargo clippy --features all-formats -- -D warnings`

Run: `cargo test --features all-formats`

- [x] **Step 2: Update issue and sync**

Close `se-2811bc05` if the pilot is working, then run `mb sync`.

- [ ] **Step 3: Commit and push**

Stage only intentional files, commit, pull/rebase, push, then confirm `git status` is up to date with origin aside from any explicitly excluded local artifacts.
