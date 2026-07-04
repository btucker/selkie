#!/usr/bin/env python3
"""Generate SPECS.md from @spec annotations."""

from __future__ import annotations

import argparse
import dataclasses
import re
import sys
from collections import defaultdict
from pathlib import Path


SPEC_RE = re.compile(r"@spec\s+([A-Z][A-Z0-9]*-\d+(?:\.\d+)*):\s*(.+)")
SCAN_SUFFIXES = {".rs", ".py", ".md"}
COMMENT_PREFIXES = ("///", "//!", "//", "#", "<!--")
SKIP_DIRS = {
    ".git",
    "target",
    "eval-report",
    "reference-implementations",
    ".venv",
    "node_modules",
}


class SpecError(Exception):
    """Raised when spec annotations are invalid."""


@dataclasses.dataclass(frozen=True)
class Spec:
    spec_id: str
    text: str
    path: str
    line: int

    @property
    def prefix(self) -> str:
        return self.spec_id.split("-", 1)[0]


def _sort_key(spec: Spec) -> tuple[str, list[int], str, int]:
    prefix, numeric = spec.spec_id.split("-", 1)
    parts = [int(part) for part in numeric.split(".")]
    return (prefix, parts, spec.path, spec.line)


def iter_source_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        rel_parts = path.relative_to(root).parts
        if any(part in SKIP_DIRS for part in rel_parts):
            continue
        if path.name == "SPECS.md":
            continue
        if path.suffix in SCAN_SUFFIXES:
            files.append(path)
    return sorted(files)


def collect_specs(root: Path) -> list[Spec]:
    specs: list[Spec] = []
    for path in iter_source_files(root):
        rel_path = path.relative_to(root).as_posix()
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except UnicodeDecodeError:
            continue

        for line_number, line in enumerate(lines, start=1):
            stripped = line.lstrip()
            if not stripped.startswith(COMMENT_PREFIXES):
                continue
            match = SPEC_RE.search(stripped)
            if not match:
                continue
            specs.append(
                Spec(
                    spec_id=match.group(1),
                    text=match.group(2).strip(),
                    path=rel_path,
                    line=line_number,
                )
            )

    validate_specs(specs)
    return sorted(specs, key=_sort_key)


def validate_specs(specs: list[Spec]) -> None:
    by_id: dict[str, list[Spec]] = defaultdict(list)
    for spec in specs:
        by_id[spec.spec_id].append(spec)

    duplicates = {spec_id: matches for spec_id, matches in by_id.items() if len(matches) > 1}
    if not duplicates:
        return

    messages = []
    for spec_id, matches in sorted(duplicates.items()):
        locations = ", ".join(f"{spec.path}:{spec.line}" for spec in matches)
        messages.append(f"Duplicate @spec ID {spec_id}: {locations}")
    raise SpecError("\n".join(messages))


def render_specs(specs: list[Spec]) -> str:
    grouped: dict[str, list[Spec]] = defaultdict(list)
    for spec in sorted(specs, key=_sort_key):
        grouped[spec.prefix].append(spec)

    lines = [
        "# Selkie Specifications",
        "",
        "This file is generated from `@spec` annotations. Do not edit it manually.",
        "",
    ]

    for prefix in sorted(grouped):
        lines.append(f"## {prefix}")
        lines.append("")
        for spec in grouped[prefix]:
            lines.append(f"### {spec.spec_id}")
            lines.append("")
            lines.append(spec.text)
            lines.append("")
            lines.append(f"Source: `{spec.path}:{spec.line}`")
            lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd(), help="repository root")
    parser.add_argument("--check", action="store_true", help="fail if SPECS.md is stale")
    args = parser.parse_args(argv)

    root = args.root.resolve()
    specs_path = root / "SPECS.md"

    try:
        rendered = render_specs(collect_specs(root))
    except SpecError as exc:
        print(str(exc), file=sys.stderr)
        return 1

    if args.check:
        existing = specs_path.read_text(encoding="utf-8") if specs_path.exists() else ""
        if existing != rendered:
            print("SPECS.md is stale; run scripts/generate-specs.py", file=sys.stderr)
            return 1
        return 0

    specs_path.write_text(rendered, encoding="utf-8")
    print(f"Wrote {specs_path.relative_to(root)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
