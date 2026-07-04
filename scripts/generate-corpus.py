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


FLOWCHART_SHAPES = [
    ("square", "A[Square]"),
    ("round", "A(Round)"),
    ("stadium", "A([Stadium])"),
    ("subroutine", "A[[Subroutine]]"),
    ("cylinder", "A[(Database)]"),
    ("circle", "A((Circle))"),
    ("double_circle", "A(((Double)))"),
    # NOTE: ellipse `A(-t-)` is intentionally omitted. Selkie's grammar parses
    # it, but mermaid v11 rejects it at render time ("No such shape: ellipse"),
    # so there is no reference to compare against. Tracked as a selkie/mermaid
    # grammar-superset divergence; keep out of the parity corpus.
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
        '  A["Legacy shape stands in for future @{shape} coverage"] --> B[B]\n'
    )
    return [Case("stretch", "future_shapes", src)]


def emit_flowchart() -> list[Case]:
    return (
        _shapes() + _directions() + _edges() + _labels()
        + _subgraphs() + _styling() + _interactions()
        + _a11y_directives() + _integration() + _stretch()
    )


EMITTERS: dict[str, Callable[[], list[Case]]] = {
    "flowchart": emit_flowchart,
}


def validate(paths: list[Path]) -> int:
    """Render every file through the selkie binary; return count of failures."""
    failures = 0
    for path in paths:
        result = subprocess.run(
            ["cargo", "run", "--quiet", "--bin", "selkie", "--", "render", str(path), "-e", "svg", "-o", "-"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            failures += 1
            print(f"INVALID (selkie): {path.name}\n{result.stderr.strip()}", file=sys.stderr)
    return failures


def validate_reference(paths: list[Path]) -> int:
    """Render every file through mmdc (mermaid). A parity corpus case is only
    useful if mermaid can produce a reference for it; mermaid's grammar is a
    subset of selkie's in places (e.g. the ellipse `A(-t-)` shape), so a case
    that selkie renders may still have no mermaid reference. Returns the count
    of files mermaid cannot render. Skips gracefully if mmdc is absent."""
    from shutil import which

    if which("mmdc") is None:
        print("mmdc not found; skipping mermaid reference validation", file=sys.stderr)
        return 0
    failures = 0
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "ref.svg"
        for path in paths:
            result = subprocess.run(
                ["mmdc", "-i", str(path), "-o", str(out), "-q"],
                capture_output=True,
                text=True,
            )
            err = result.stderr or ""
            if result.returncode != 0 or "No such shape" in err or "Error" in err:
                failures += 1
                first = next((ln for ln in err.splitlines() if "Error" in ln or "No such" in ln), err.strip()[:120])
                print(f"NO MERMAID REFERENCE: {path.name} :: {first}", file=sys.stderr)
    return failures


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--type", default="flowchart", help="diagram type to generate")
    parser.add_argument("--all", action="store_true", help="generate every registered type")
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--validate", action="store_true", help="render each file via selkie; fail on errors")
    parser.add_argument("--validate-reference", action="store_true", help="render each file via mmdc (mermaid); fail if mermaid cannot produce a reference")
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
        print(f"validated {len(all_written)} files, all render OK under selkie")

    if args.validate_reference:
        failures = validate_reference(all_written)
        if failures:
            print(f"{failures} file(s) have no mermaid reference", file=sys.stderr)
            return 1
        print(f"validated {len(all_written)} files, all render OK under mermaid")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
