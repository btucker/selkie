# FLOW Eval Failure Families Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic flowchart eval failure-family summaries mapped to `FLOW-*` requirements.

**Architecture:** Add a small eval classifier that groups `DiagramResult` issues into named failure families with spec IDs, affected diagrams, and issue counts. Compute summaries from the current `EvalResult` diagrams when rendering reports, include them in summary JSON, and show the highest-signal families in text reports.

**Tech Stack:** Rust eval module, serde JSON reports, existing generated `SPECS.md`.

**Reference implementation notes:**
- `reference-implementations/mermaid/packages/mermaid/src/diagrams/flowchart/flowDb.ts` stores subgraph titles as group node labels and normalizes chart direction before layout data is built.
- `reference-implementations/mermaid/packages/mermaid/src/rendering-util/createGraph.ts` inserts edge labels as layout nodes, so missing edge-label text is a compatibility failure.
- `reference-implementations/mermaid/packages/mermaid/src/diagrams/flowchart/flowRenderer-v3-unified.ts` passes direction, node spacing, rank spacing, and marker choices into the registered layout algorithm.
- `reference-implementations/mermaid/packages/mermaid/src/diagrams/flowchart/styles.ts` defines flowchart edge, node, cluster, and cluster-label styling behavior.

---

### Task 1: Failure Family Classifier

**Files:**
- Create: `src/eval/failure_families.rs`
- Modify: `src/eval/mod.rs`

- [x] **Step 1: Write failing tests**

Add tests for grouping missing flowchart subgraph labels, missing edge labels, dimension drift, aspect-ratio orientation drift, edge-position drift, and style mismatch issues into `FLOW-*` failure families.

- [x] **Step 2: Run tests to verify RED**

Run: `cargo test --features all-formats,eval groups_missing_subgraph_titles_under_flow_requirement -- --nocapture`

Expected: FAIL because the classifier is not implemented/exported.

- [x] **Step 3: Implement classifier and data model**

Add serializable failure-family summary types and compute them from diagram issues.

- [x] **Step 4: Run focused tests to verify GREEN**

Run: `cargo test --features all-formats,eval failure_families -- --nocapture`

Expected: PASS.

### Task 2: Reports and Specs

**Files:**
- Modify: `src/eval/report.rs`
- Modify: `src/eval/failure_families.rs`
- Modify: `SPECS.md`

- [x] **Step 1: Write failing report tests**

Add assertions that agent-friendly/text summary output includes a "Failure Families" section with spec IDs and diagram counts.

- [x] **Step 2: Implement report output**

Print family summaries near the top of text reports and include them in summary JSON.

- [x] **Step 3: Add `FLOW-*` annotations**

Annotate tests for subgraph title visibility and major orientation preservation, then regenerate `SPECS.md`.

### Task 3: Verification and Landing

**Files:**
- All changed files.

- [x] **Step 1: Run focused checks**

Run: `cargo test --features all-formats,eval failure_families -- --nocapture`

Run: `cargo test --features all-formats,eval eval::report -- --nocapture`

Run: `python3 scripts/generate-specs.py --check`

- [x] **Step 2: Run required quality gates**

Run: `cargo fmt`

Run: `cargo clippy --features all-formats -- -D warnings`

Run: `cargo test --features all-formats`

Run: `cargo run --features eval --bin selkie -- eval --type flowchart --use-repo-svgs -o /tmp/selkie-flow-families`

- [x] **Step 3: Close issue, sync, commit, push**

Close `se-8d07a4d6`, run `mb sync`, commit, rebase/merge as needed, push, and verify `git status` is up to date with origin.
