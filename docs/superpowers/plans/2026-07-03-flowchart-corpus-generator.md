# Flowchart Corpus Generator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a deterministic Python generator that emits an original corpus of flowchart `.mmd` files systematically covering selkie's flowchart grammar, one feature-axis at a time, wired into the existing eval.

**Architecture:** A single script `scripts/generate-corpus.py` with a type-agnostic framework (CLI, `Case` records, idempotent write, emitter registry, `--validate`, `--check`) and a flowchart emitter composed of one function per feature family. Output files land in `docs/sources/gen_flowchart_*.mmd` and are auto-discovered by the eval (`src/eval/samples.rs` globs `docs/sources/*.mmd`). A pytest self-test guards framework invariants; `--validate` (render each file through the `selkie` binary) guards content correctness.

**Tech Stack:** Python 3 (run via `uv`), argparse, pathlib, pytest; the existing `selkie` Rust CLI for validation; mmdc (already used by the eval) generates references.

## Global Constraints

- Python is run via `uv` (project rule). Invoke the script as `uv run scripts/generate-corpus.py ...` and tests as `uv run pytest ...`.
- All generated diagram content is **original** — no text copied from mermaid's repo or docs (licensing).
- Generated files use the `gen_<type>_<family>_<case>.mmd` naming namespace only. Never create, modify, or delete `channel_*`, `example_*`, or bare per-type samples.
- Generator output must be **deterministic**: same code → byte-identical files (required for `--check`).
- Scope is limited to flowchart features selkie's grammar **declares** (15 legacy shapes; no v11 `@{shape}`/icon/image/ELK). One tagged `stretch` file documents the out-of-scope axis but is not expected to pass.
- Commit message trailer on every commit: `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- Mirror `scripts/generate-specs.py`: argparse with `--check`, `main(argv) -> int`.

---

## File Structure

- Create: `scripts/generate-corpus.py` — the generator (framework + flowchart emitter).
- Create: `scripts/test_generate_corpus.py` — pytest self-tests for framework invariants.
- Create (generated, committed): `docs/sources/gen_flowchart_*.mmd` — ~70–85 corpus files.
- Modify: `.github/workflows/*.yml` (the Lint job) — add `generate-corpus.py --check` step.

---

## Task 1: Framework skeleton (CLI, Case, registry, idempotent write) with self-tests

**Files:**
- Create: `scripts/generate-corpus.py`
- Test: `scripts/test_generate_corpus.py`

**Interfaces:**
- Produces:
  - `Case = namedtuple("Case", ["family", "name", "source"])`
  - `EMITTERS: dict[str, Callable[[], list[Case]]]`
  - `filename(diagram_type: str, case: Case) -> str` → `f"gen_{diagram_type}_{case.family}_{case.name}.mmd"`
  - `write_cases(diagram_type: str, cases: list[Case], out_dir: Path) -> list[Path]` (deletes existing `gen_{diagram_type}_*.mmd` first, then writes; returns written paths)
  - `main(argv: list[str] | None = None) -> int`

- [ ] **Step 1: Write the failing test**

Create `scripts/test_generate_corpus.py`:

```python
import importlib.util
from pathlib import Path

_spec = importlib.util.spec_from_file_location(
    "generate_corpus", Path(__file__).parent / "generate-corpus.py"
)
gc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gc)


def test_registry_has_flowchart():
    assert "flowchart" in gc.EMITTERS


def test_filename_format():
    c = gc.Case(family="shapes", name="diamond", source="flowchart TD\n  A{q}\n")
    assert gc.filename("flowchart", c) == "gen_flowchart_shapes_diamond.mmd"


def test_write_is_idempotent(tmp_path):
    cases = [gc.Case("shapes", "sq", "flowchart TD\n  A[x]\n")]
    first = gc.write_cases("flowchart", cases, tmp_path)
    contents1 = {p.name: p.read_text() for p in tmp_path.glob("*.mmd")}
    second = gc.write_cases("flowchart", cases, tmp_path)
    contents2 = {p.name: p.read_text() for p in tmp_path.glob("*.mmd")}
    assert contents1 == contents2
    assert [p.name for p in first] == [p.name for p in second]


def test_write_clobbers_only_own_namespace(tmp_path):
    (tmp_path / "channel_flowchart_keep.mmd").write_text("flowchart TD\n  K[keep]\n")
    (tmp_path / "gen_flowchart_shapes_stale.mmd").write_text("stale")
    gc.write_cases("flowchart", [gc.Case("shapes", "sq", "flowchart TD\n  A[x]\n")], tmp_path)
    assert (tmp_path / "channel_flowchart_keep.mmd").exists()
    assert not (tmp_path / "gen_flowchart_shapes_stale.mmd").exists()
    assert (tmp_path / "gen_flowchart_shapes_sq.mmd").exists()


def test_no_duplicate_filenames():
    cases = gc.EMITTERS["flowchart"]()
    names = [gc.filename("flowchart", c) for c in cases]
    assert len(names) == len(set(names)), "duplicate generated filenames"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest scripts/test_generate_corpus.py -q`
Expected: FAIL — `generate-corpus.py` does not exist / `EMITTERS` undefined.

- [ ] **Step 3: Write minimal framework**

Create `scripts/generate-corpus.py` (stub flowchart emitter returns one case so tests pass; real cases land in Tasks 2–3):

```python
#!/usr/bin/env python3
"""Generate an original corpus of mermaid .mmd files exercising diagram-type
grammar one feature-axis at a time. See
docs/superpowers/specs/2026-07-03-flowchart-corpus-generator-design.md.

Files are written as gen_<type>_<family>_<case>.mmd into docs/sources/ and are
auto-discovered by the eval. Deterministic: same code -> identical output.
"""
from __future__ import annotations

import argparse
import subprocess
import sys
import tempfile
from collections import namedtuple
from pathlib import Path
from typing import Callable

Case = namedtuple("Case", ["family", "name", "source"])

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUT_DIR = REPO_ROOT / "docs" / "sources"


def filename(diagram_type: str, case: Case) -> str:
    return f"gen_{diagram_type}_{case.family}_{case.name}.mmd"


def write_cases(diagram_type: str, cases: list[Case], out_dir: Path) -> list[Path]:
    out_dir.mkdir(parents=True, exist_ok=True)
    for stale in sorted(out_dir.glob(f"gen_{diagram_type}_*.mmd")):
        stale.unlink()
    written: list[Path] = []
    for case in cases:
        path = out_dir / filename(diagram_type, case)
        text = case.source if case.source.endswith("\n") else case.source + "\n"
        path.write_text(text)
        written.append(path)
    return written


def emit_flowchart() -> list[Case]:
    # Replaced with full feature families in Tasks 2-3.
    return [Case("shapes", "square", "flowchart TD\n  A[Square]\n")]


EMITTERS: dict[str, Callable[[], list[Case]]] = {
    "flowchart": emit_flowchart,
}


def validate(paths: list[Path]) -> int:
    """Render every file through the selkie binary; return count of failures."""
    failures = 0
    for path in paths:
        result = subprocess.run(
            ["cargo", "run", "--quiet", "--bin", "selkie", "--", "render", str(path), "-f", "svg"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            failures += 1
            print(f"INVALID: {path.name}\n{result.stderr.strip()}", file=sys.stderr)
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--type", default="flowchart", help="diagram type to generate")
    parser.add_argument("--all", action="store_true", help="generate every registered type")
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--validate", action="store_true", help="render each file via selkie; fail on errors")
    parser.add_argument("--check", action="store_true", help="fail if committed corpus is stale")
    args = parser.parse_args(argv)

    types = list(EMITTERS) if args.all else [args.type]
    for t in types:
        if t not in EMITTERS:
            print(f"unknown diagram type: {t}", file=sys.stderr)
            return 2

    if args.check:
        stale = False
        for t in types:
            cases = EMITTERS[t]()
            with tempfile.TemporaryDirectory() as tmp:
                tmp_dir = Path(tmp)
                write_cases(t, cases, tmp_dir)
                for case in cases:
                    name = filename(t, case)
                    want = (tmp_dir / name).read_text()
                    committed = args.out_dir / name
                    if not committed.exists() or committed.read_text() != want:
                        print(f"stale/missing: {name}", file=sys.stderr)
                        stale = True
                committed_names = {p.name for p in args.out_dir.glob(f"gen_{t}_*.mmd")}
                expected_names = {filename(t, c) for c in cases}
                for extra in committed_names - expected_names:
                    print(f"unexpected committed file: {extra}", file=sys.stderr)
                    stale = True
        if stale:
            print("corpus is stale; run: uv run scripts/generate-corpus.py --all", file=sys.stderr)
            return 1
        return 0

    all_written: list[Path] = []
    for t in types:
        written = write_cases(t, EMITTERS[t](), args.out_dir)
        print(f"{t}: wrote {len(written)} files")
        all_written.extend(written)

    if args.validate:
        failures = validate(all_written)
        if failures:
            print(f"{failures} invalid file(s)", file=sys.stderr)
            return 1
        print(f"validated {len(all_written)} files, all render OK")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run pytest scripts/test_generate_corpus.py -q`
Expected: PASS (5 passed).

- [ ] **Step 5: Verify the CLI runs end-to-end into a temp dir**

Run: `uv run scripts/generate-corpus.py --type flowchart --out-dir /tmp/corpus_smoke && ls /tmp/corpus_smoke`
Expected: prints `flowchart: wrote 1 files`; `gen_flowchart_shapes_square.mmd` present.

- [ ] **Step 6: Commit**

```bash
git add scripts/generate-corpus.py scripts/test_generate_corpus.py
git commit -m "feat(corpus): generator framework with registry, idempotent write, check/validate

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 2: Flowchart emitters — shapes, directions, edges, labels

**Files:**
- Modify: `scripts/generate-corpus.py` (replace `emit_flowchart`, add family functions)
- Test: `scripts/test_generate_corpus.py` (add coverage assertions)

**Interfaces:**
- Consumes: `Case`, `filename`, `EMITTERS` from Task 1.
- Produces: `emit_flowchart()` returns cases including families `shapes`, `directions`, `edges`, `labels`. Later families are appended in Task 3.

Reference — exact grammar constructs to emit (verified against `src/diagrams/flowchart/`):
- **15 shapes** with their syntax: square `A[t]`, round `A(t)`, stadium `A([t])`, subroutine `A[[t]]`, cylinder `A[(t)]`, circle `A((t))`, double-circle `A(((t)))`, ellipse `A(-t-)`, diamond `A{t}`, hexagon `A{{t}}`, lean-right `A[/t/]`, lean-left `A[\t\]`, trapezoid `A[/t\]`, inv-trapezoid `A[\t/]`, odd `A>t]`.
- **5 directions:** `TB`, `TD`, `LR`, `RL`, `BT` (keyword `flowchart <DIR>`).
- **edge variants:** `-->`, `--x`, `--o`, `---`, `<-->`, `x--x`, `o--o`, `-.->`, `-.-`, `==>`, `===`, text-pipe `A -->|t| B`, text-inline `A -- t --> B`, chain `A --> B --> C`, long `A ----> B`.
- **labels:** plain `A[Plain]`, quoted `A["Quote, comma"]`, break `A["Line1<br>Line2"]`, entity `A["Vec&lt;T&gt; &amp; more"]`, escape `A["path\\to"]`, markdown `A["\`**bold**\`"]`, wrap (a >200px single line), unicode/emoji `A["café ✅ 日本"]`.

- [ ] **Step 1: Write the failing test**

Add to `scripts/test_generate_corpus.py`:

```python
def test_flowchart_families_present():
    cases = gc.EMITTERS["flowchart"]()
    families = {c.family for c in cases}
    for required in ("shapes", "directions", "edges", "labels"):
        assert required in families, f"missing family {required}"


def test_all_15_shapes_covered():
    cases = gc.EMITTERS["flowchart"]()
    shape_cases = [c for c in cases if c.family == "shapes"]
    # 15 individual shapes + 1 grid
    assert len(shape_cases) >= 16


def test_all_5_directions_covered():
    cases = gc.EMITTERS["flowchart"]()
    dir_names = {c.name for c in cases if c.family == "directions"}
    assert dir_names == {"tb", "td", "lr", "rl", "bt"}


def test_every_case_starts_with_flowchart_keyword():
    for c in gc.EMITTERS["flowchart"]():
        assert c.source.lstrip().startswith(("flowchart", "graph", "%%{")), c.name
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest scripts/test_generate_corpus.py -q`
Expected: FAIL — `test_all_15_shapes_covered` (stub emits 1 shape), `test_all_5_directions_covered`.

- [ ] **Step 3: Implement the four family functions**

In `scripts/generate-corpus.py`, replace the stub `emit_flowchart` with family functions and an aggregator. Example implementation (complete):

```python
FLOWCHART_SHAPES = [
    ("square", "A[Square]"),
    ("round", "A(Round)"),
    ("stadium", "A([Stadium])"),
    ("subroutine", "A[[Subroutine]]"),
    ("cylinder", "A[(Database)]"),
    ("circle", "A((Circle))"),
    ("double_circle", "A(((Double)))"),
    ("ellipse", "A(-Ellipse-)"),
    ("diamond", "A{Diamond}"),
    ("hexagon", "A{{Hexagon}}"),
    ("lean_right", "A[/Lean right/]"),
    ("lean_left", "A[\\Lean left\\]"),
    ("trapezoid", "A[/Trapezoid\\]"),
    ("inv_trapezoid", "A[\\Inv trap/]"),
    ("odd", "A>Odd shape]"),
]


def _shapes() -> list[Case]:
    cases = []
    for name, decl in FLOWCHART_SHAPES:
        src = f"flowchart TD\n  {decl} --> B[Next]\n"
        cases.append(Case("shapes", name, src))
    grid_nodes = "\n".join(
        f"  N{i}{decl[1:]}" for i, (_, decl) in enumerate(FLOWCHART_SHAPES)
    )
    cases.append(Case("shapes", "all_grid", f"flowchart TD\n{grid_nodes}\n"))
    return cases


def _directions() -> list[Case]:
    cases = []
    for d in ("TB", "TD", "LR", "RL", "BT"):
        src = f"flowchart {d}\n  A[Start] --> B[Middle]\n  B --> C[End]\n  B --> D[Side]\n"
        cases.append(Case("directions", d.lower(), src))
    return cases


FLOWCHART_EDGES = [
    ("arrow", "A --> B"),
    ("cross", "A --x B"),
    ("circle_end", "A --o B"),
    ("open", "A --- B"),
    ("bi_arrow", "A <--> B"),
    ("bi_cross", "A x--x B"),
    ("bi_circle", "A o--o B"),
    ("dotted_arrow", "A -.-> B"),
    ("dotted_open", "A -.- B"),
    ("thick_arrow", "A ==> B"),
    ("thick_open", "A === B"),
    ("text_pipe", "A -->|label| B"),
    ("text_inline", "A -- label --> B"),
    ("chain", "A --> B --> C"),
    ("length", "A ----> B"),
]


def _edges() -> list[Case]:
    cases = []
    for name, edge in FLOWCHART_EDGES:
        src = f"flowchart LR\n  {edge}\n"
        cases.append(Case("edges", name, src))
    return cases


FLOWCHART_LABELS = [
    ("plain", "A[Plain text]"),
    ("quoted", 'A["Quoted, with comma"]'),
    ("br", 'A["Line one<br>Line two"]'),
    ("entity", 'A["Vec&lt;T&gt; &amp; Co"]'),
    ("escape", 'A["path\\to\\file"]'),
    ("markdown", 'A["`**bold** and _em_`"]'),
    ("wrap", 'A["This is a deliberately long label that should exceed the two hundred pixel wrapping width and wrap onto multiple lines"]'),
    ("unicode", 'A["café ✅ 日本語"]'),
]


def _labels() -> list[Case]:
    cases = []
    for name, decl in FLOWCHART_LABELS:
        src = f"flowchart TD\n  {decl} --> B[Next]\n"
        cases.append(Case("labels", name, src))
    return cases


def emit_flowchart() -> list[Case]:
    return _shapes() + _directions() + _edges() + _labels()
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run pytest scripts/test_generate_corpus.py -q`
Expected: PASS.

- [ ] **Step 5: Generate into a temp dir and eyeball a few files**

Run:
```bash
uv run scripts/generate-corpus.py --type flowchart --out-dir /tmp/corpus_t2
cat /tmp/corpus_t2/gen_flowchart_directions_lr.mmd
cat /tmp/corpus_t2/gen_flowchart_edges_dotted_arrow.mmd
cat /tmp/corpus_t2/gen_flowchart_labels_markdown.mmd
```
Expected: LR file starts `flowchart LR`; dotted file contains `-.->`; markdown file contains a backtick-wrapped `**bold**`.

- [ ] **Step 6: Commit**

```bash
git add scripts/generate-corpus.py scripts/test_generate_corpus.py
git commit -m "feat(corpus): flowchart shapes, directions, edges, labels families

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 3: Flowchart emitters — subgraphs, styling, interactions, a11y/directives, integration, stretch

**Files:**
- Modify: `scripts/generate-corpus.py` (add family functions; extend `emit_flowchart`)
- Test: `scripts/test_generate_corpus.py`

**Interfaces:**
- Consumes: `Case`, family helpers from Task 2.
- Produces: `emit_flowchart()` now also returns families `subgraphs`, `styling`, `interactions`, `a11y_directives`, `integration`, `stretch`.

- [ ] **Step 1: Write the failing test**

Add to `scripts/test_generate_corpus.py`:

```python
def test_all_families_present():
    cases = gc.EMITTERS["flowchart"]()
    families = {c.family for c in cases}
    expected = {
        "shapes", "directions", "edges", "labels",
        "subgraphs", "styling", "interactions", "a11y_directives",
        "integration", "stretch",
    }
    assert expected <= families, expected - families


def test_corpus_size_in_target_range():
    cases = gc.EMITTERS["flowchart"]()
    assert 60 <= len(cases) <= 100, len(cases)


def test_styling_family_uses_classdef_and_linkstyle():
    cases = gc.EMITTERS["flowchart"]()
    styling = "\n".join(c.source for c in cases if c.family == "styling")
    assert "classDef" in styling
    assert "linkStyle" in styling
    assert "\n  style " in "\n" + styling


def test_isolated_and_connected_subgraph_cases_exist():
    names = {c.name for c in gc.EMITTERS["flowchart"]() if c.family == "subgraphs"}
    assert "isolated" in names
    assert "connected" in names
    assert "own_direction" in names
```

- [ ] **Step 2: Run test to verify it fails**

Run: `uv run pytest scripts/test_generate_corpus.py -q`
Expected: FAIL — new families absent.

- [ ] **Step 3: Implement the remaining family functions**

Add to `scripts/generate-corpus.py` (complete):

```python
def _subgraphs() -> list[Case]:
    cases = []
    cases.append(Case("subgraphs", "titled",
        "flowchart TD\n  subgraph S1 [Titled Group]\n    A[A] --> B[B]\n  end\n"))
    cases.append(Case("subgraphs", "untitled",
        "flowchart TD\n  subgraph S1\n    A[A] --> B[B]\n  end\n"))
    cases.append(Case("subgraphs", "nested",
        "flowchart TD\n  subgraph Outer\n    subgraph Inner\n      A[A]\n    end\n    B[B]\n  end\n  A --> B\n"))
    cases.append(Case("subgraphs", "cross_edge",
        "flowchart TD\n  subgraph S1\n    A[A]\n  end\n  subgraph S2\n    B[B]\n  end\n  A --> B\n"))
    cases.append(Case("subgraphs", "own_direction",
        "flowchart TD\n  subgraph S1 [Inner LR]\n    direction LR\n    A[A] --> B[B]\n  end\n  C[C] --> S1\n"))
    # isolated: cluster with NO external edges (extracted + flipped by layout)
    cases.append(Case("subgraphs", "isolated",
        "flowchart TB\n  subgraph S1 [Isolated]\n    A[A] --> B[B] --> C[C]\n  end\n  X[Outside] --> Y[Alone]\n"))
    # connected: cluster WITH an external edge (not extracted)
    cases.append(Case("subgraphs", "connected",
        "flowchart TB\n  subgraph S1 [Connected]\n    A[A] --> B[B]\n  end\n  Z[Ext] --> A\n"))
    return cases


def _styling() -> list[Case]:
    cases = []
    cases.append(Case("styling", "classdef",
        "flowchart TD\n  A[A] --> B[B]\n  classDef hot fill:#f96,stroke:#333,stroke-width:2px\n  class A hot\n"))
    cases.append(Case("styling", "inline_style",
        "flowchart TD\n  A[A] --> B[B]\n  style A fill:#9f6,stroke:#333\n"))
    cases.append(Case("styling", "linkstyle",
        "flowchart TD\n  A[A] --> B[B]\n  linkStyle 0 stroke:#f00,stroke-width:3px\n"))
    cases.append(Case("styling", "combined",
        "flowchart LR\n  A[A] --> B[B] --> C[C]\n"
        "  classDef warn fill:#fe9,stroke:#a80\n  class B warn\n"
        "  style C fill:#9cf\n  linkStyle 1 stroke:#08f,stroke-width:2px\n"))
    return cases


def _interactions() -> list[Case]:
    cases = []
    cases.append(Case("interactions", "click_callback",
        'flowchart TD\n  A[Click me] --> B[B]\n  click A callback "A tooltip"\n'))
    cases.append(Case("interactions", "href",
        'flowchart TD\n  A[Link] --> B[B]\n  click A href "https://example.com" "Open"\n'))
    cases.append(Case("interactions", "tooltip",
        'flowchart TD\n  A[Hover] --> B[B]\n  click A callback "Just a tooltip"\n'))
    cases.append(Case("interactions", "callback_args",
        'flowchart TD\n  A[Args] --> B[B]\n  click A call handler("x", 42) "With args"\n'))
    return cases


def _a11y_directives() -> list[Case]:
    cases = []
    cases.append(Case("a11y_directives", "acc_title_descr",
        "flowchart TD\n  accTitle: Accessible Title\n  accDescr: A short description\n  A[A] --> B[B]\n"))
    cases.append(Case("a11y_directives", "init_theme",
        "%%{init: {'theme': 'forest'}}%%\nflowchart TD\n  A[A] --> B[B]\n"))
    return cases


def _integration() -> list[Case]:
    cases = []
    cases.append(Case("integration", "pipeline",
        "flowchart LR\n"
        "  subgraph Ingest\n    S[(Source)] --> P{Parse?}\n  end\n"
        "  P -->|ok| T[[Transform]]\n  P -->|fail| E>Error]\n"
        "  T --> D[(Sink)]\n  classDef bad fill:#f99\n  class E bad\n"))
    cases.append(Case("integration", "decision_tree",
        "flowchart TD\n"
        '  A{Start} -->|yes| B([Do it])\n  A -->|no| C([Skip])\n'
        "  B --> D((Done))\n  C --> D\n  style D fill:#9f6\n"))
    cases.append(Case("integration", "state_machine",
        "flowchart LR\n"
        "  I((Init)) --> R[Running]\n  R -.->|pause| P[Paused]\n"
        "  P ==>|resume| R\n  R --> F(((Final)))\n"))
    cases.append(Case("integration", "nested_styled",
        "flowchart TB\n"
        "  subgraph Web [Frontend]\n    direction LR\n    UI[UI] --> API[API]\n  end\n"
        "  subgraph Data\n    DB[(DB)]\n  end\n"
        "  API --> DB\n  classDef svc fill:#cef\n  class UI,API svc\n"))
    cases.append(Case("integration", "wide_labels",
        "flowchart TD\n"
        '  A["Long label that wraps across the wrapping width boundary here"] --> B{Choose}\n'
        '  B -->|"first option with text"| C[C]\n  B -->|"second option"| D[D]\n'))
    return cases


def _stretch() -> list[Case]:
    # Out-of-scope axis (v11 @{shape}/icons) documented for the future; NOT
    # expected to pass under current selkie grammar. Uses only a comment + a
    # legacy shape so it still parses.
    src = (
        "flowchart TD\n"
        "  %% STRETCH: mermaid v11 @{shape:...}, fa: icons, and img shapes are\n"
        "  %% out of scope until selkie's grammar supports them. Placeholder:\n"
        "  A[Legacy shape stands in for future @{shape} coverage] --> B[B]\n"
    )
    return [Case("stretch", "future_shapes", src)]


def emit_flowchart() -> list[Case]:
    return (
        _shapes() + _directions() + _edges() + _labels()
        + _subgraphs() + _styling() + _interactions()
        + _a11y_directives() + _integration() + _stretch()
    )
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `uv run pytest scripts/test_generate_corpus.py -q`
Expected: PASS. Confirm `test_corpus_size_in_target_range` reports a count in 60–100 (expect ~70).

- [ ] **Step 5: Commit**

```bash
git add scripts/generate-corpus.py scripts/test_generate_corpus.py
git commit -m "feat(corpus): subgraph, styling, interaction, a11y, integration families

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Task 4: Validation pass — every generated file must render under selkie

**Files:**
- Modify: `scripts/generate-corpus.py` (only if a fix to `validate` or an emitted source is needed)
- Modify: any `gen_*` source that fails to render (via its family function)

**Interfaces:**
- Consumes: `validate(paths)`, `write_cases`, `emit_flowchart` from prior tasks.

- [ ] **Step 1: Build the selkie binary once (fast subsequent runs)**

Run: `cargo build --bin selkie`
Expected: builds clean.

- [ ] **Step 2: Generate into a temp dir and validate every file through selkie**

Run:
```bash
uv run scripts/generate-corpus.py --type flowchart --out-dir /tmp/corpus_validate --validate
```
Expected: either `validated N files, all render OK`, or a list of `INVALID: <file>` with stderr.

- [ ] **Step 3: Fix any invalid sources**

For each `INVALID:` file, inspect the source and the selkie error. The fix belongs in the emitting family function in `scripts/generate-corpus.py` (adjust the diagram to valid grammar selkie accepts) — NOT by hand-editing the generated file. Re-run Step 2 until zero failures.

Note on likely offenders to verify against `src/diagrams/flowchart/flowchart.pest`:
- ellipse `A(-t-)` and double-circle `A(((t)))` — confirm the exact accepted delimiters; if selkie rejects a form, use the delimiter the grammar defines.
- `click ... call handler(...)` / `callback` — confirm the accepted `click` action keywords (`call`, `callback`, `href`); drop any form selkie's parser rejects.
- `%%{init:...}%%` directive — confirm selkie parses it; if not, move that case to `stretch` or remove it.

- [ ] **Step 4: Re-run the self-tests (family fixes must not break coverage)**

Run: `uv run pytest scripts/test_generate_corpus.py -q`
Expected: PASS (coverage assertions still hold after any source fixes).

- [ ] **Step 5: Commit any fixes**

```bash
git add scripts/generate-corpus.py
git commit -m "fix(corpus): correct generated sources to valid selkie grammar

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

(If no fixes were needed, skip the commit and note validation passed clean.)

---

## Task 5: Generate the committed corpus, wire `--check` into CI, verify eval discovery

**Files:**
- Create (generated): `docs/sources/gen_flowchart_*.mmd`
- Modify: the CI Lint workflow (add `generate-corpus.py --check` step)

**Interfaces:**
- Consumes: the full generator from Tasks 1–4.

- [ ] **Step 1: Generate the real corpus into docs/sources**

Run: `uv run scripts/generate-corpus.py --all`
Expected: `flowchart: wrote ~70 files`. Confirm only `gen_flowchart_*` appear as new/changed:
```bash
git status --short docs/sources | grep -v '^.. docs/sources/gen_flowchart_' || echo "only gen_ files touched"
```
Expected: prints `only gen_ files touched`.

- [ ] **Step 2: Confirm `--check` passes against the committed set**

Run: `uv run scripts/generate-corpus.py --all --check; echo "exit=$?"`
Expected: no `stale/missing` output; `exit=0`.

- [ ] **Step 3: Confirm the eval discovers the new files**

Run: `cargo run --features eval --bin selkie -- eval --type flowchart --brief 2>&1 | grep -E 'flowchart|Similarity' | head`
Expected: the flowchart diagram count reflects the added `gen_` files (was 33; now ~100+). (Do not chase scores here — this task only confirms discovery. Revert any `docs/images` churn: `git checkout -- docs/images`.)

- [ ] **Step 4: Add the `--check` step to CI**

Find the Lint job's existing specs step:
```bash
grep -rn 'generate-specs.py --check' .github/workflows/
```
In that workflow file, immediately after the `generate-specs.py --check` step, add a sibling step (match the surrounding YAML indentation and `run:` style):

```yaml
      - name: Check corpus is up to date
        run: uv run scripts/generate-corpus.py --all --check
```

- [ ] **Step 5: Commit the corpus and CI change**

```bash
git add docs/sources/gen_flowchart_*.mmd .github/workflows/
git commit -m "feat(corpus): generate committed flowchart corpus and gate staleness in CI

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

- [ ] **Step 6: Final verification**

Run: `uv run pytest scripts/test_generate_corpus.py -q && uv run scripts/generate-corpus.py --all --check && echo OK`
Expected: tests pass, check passes, prints `OK`.

---

## Self-Review

**Spec coverage:**
- Original generator, Python under `scripts/`, mirrors generate-specs.py → Task 1. ✓
- Framework/registry/idempotency/naming → Task 1. ✓
- All feature families (shapes, directions, edges, labels, subgraphs, styling, interactions, a11y_directives, integration, stretch) → Tasks 2–3. ✓
- Orthogonal coverage, ~70–85 files → Task 3 size assertion (60–100 guard). ✓
- Original content constraint → Global Constraints + all sources hand-written in emitters. ✓
- `--validate` renders via selkie → Task 1 (impl) + Task 4 (run/fix). ✓
- `--check` staleness gate + CI → Task 1 (impl) + Task 5 (wire). ✓
- Eval auto-discovery → Task 5 Step 3. ✓
- Non-goals (v11 shapes) → `stretch` file, Task 3 `_stretch`. ✓
- Template extensibility (registry) → Task 1 `EMITTERS`. ✓

**Placeholder scan:** No TBD/TODO; every code step shows complete code; Task 4 fixes reference the exact grammar file to check. ✓

**Type consistency:** `Case(family, name, source)`, `filename()`, `write_cases()`, `validate()`, `EMITTERS`, `emit_flowchart()` used consistently across all tasks. Family helper names (`_shapes`,`_directions`,`_edges`,`_labels`,`_subgraphs`,`_styling`,`_interactions`,`_a11y_directives`,`_integration`,`_stretch`) are consistent between Tasks 2, 3, and their tests. ✓
