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
