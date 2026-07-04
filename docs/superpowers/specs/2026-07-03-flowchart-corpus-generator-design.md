# Flowchart Corpus Generator — Design

**Date:** 2026-07-03
**Status:** Approved (design), pending implementation
**Branch:** leverage-on-quality

## Problem

The flowchart eval set (`docs/sources/*flowchart*.mmd`, 29 files) is drawn almost
entirely from real-world "channel" diagrams. It is stylistically monolithic and
leaves large parts of the flowchart grammar untested:

- **Direction:** all 29 are top-down (TB/TD). Zero LR, RL, or BT — despite
  direction handling being a dominant source of layout bugs this cycle.
- **Styling:** only 4 use `classDef`; zero use `linkStyle`.
- **Labels:** zero markdown-string labels; entity/escape and wrap edge cases
  are incidental, not deliberate.
- **Shapes/edges:** exercised only where the real diagrams happened to use them,
  not systematically.

Because coverage is incidental, a regression in an untested feature (e.g. LR
layout) is invisible to the eval, and a failure in a busy real-world diagram is
hard to attribute to one feature.

We will **generate an original corpus** (no mermaid content copied — licensing)
that exercises flowchart functionality **one axis at a time**, so failures are
attributable to a single feature, and structure the generator so other diagram
types plug in later.

## Goals

1. Systematic coverage of every flowchart grammar feature **selkie declares**
   (the 15 legacy shapes, all edge/arrow variants, 4 directions, subgraphs,
   `classDef`/`class`/`style`/`linkStyle`, interactions, markdown/entity labels,
   accessibility, init directives).
2. **Orthogonal, not combinatorial:** vary one axis at full resolution per file,
   plus a small set of realistic multi-feature integration diagrams. Target
   ~70–85 files, each attributable to one feature.
3. **Original content**, authored by the generator's own templates.
4. **Idempotent + CI-reproducible**, and picked up by the existing eval with no
   extra wiring.
5. Generator structured as a **reusable template** for other diagram types.

## Non-Goals (YAGNI)

- The mermaid v11 `@{ shape: … }` catalog (~70 shapes selkie's grammar does not
  parse), FontAwesome/icon/image shapes, and ELK-layout variants. These are
  "features selkie has not built," a separate concern from covering what selkie
  claims to support. A single tagged stretch-file will note them as a future
  axis; we will not generate cases that cannot pass.
- Changing the eval scoring, the renderer, or any parser behavior. This work
  only adds inputs.

## Architecture

A single Python script, `scripts/generate-corpus.py`, run via `uv`, mirroring
the existing `scripts/generate-specs.py` (argparse, `--check`, `int` return from
`main`).

Two layers:

### Framework (type-agnostic)

- CLI: `--type <name>` (default `flowchart`), `--all`, `--check`, `--validate`,
  `--out-dir docs/sources`.
- `Case` = `(family: str, name: str, source: str)`.
- `write_cases(diagram_type, cases, out_dir)`: writes each case to
  `<out_dir>/gen_<diagram_type>_<family>_<name>.mmd`.
- **Idempotent regeneration:** before writing, delete existing
  `gen_<diagram_type>_*.mmd` in `out_dir`, then emit the fresh set. Curated
  `channel_*`/`example_*`/bare samples are never touched (different namespace).
- **Registry:** `EMITTERS: dict[str, Callable[[], list[Case]]]` maps a diagram
  type to its emitter. Flowchart is the first entry; a new type is a new
  function registered here. `--all` iterates the registry.

### Flowchart emitter (feature families)

`emit_flowchart() -> list[Case]` composed of one function per family, each
returning `list[Case]`. Families and cases:

| Family | Cases |
|---|---|
| `shapes` | one minimal graph per shape (15): square, round, stadium, subroutine, cylinder, circle, double-circle, ellipse, diamond, hexagon, lean-left, lean-right, trapezoid, inv-trapezoid, odd; plus one `all_shapes` grid |
| `directions` | same 4-node graph in TB, TD, LR, RL, BT |
| `edges` | end-heads (`-->`, `--x`, `--o`, `---`); start+end (`<-->`, `x--x`, `o--o`); dotted (`-.->`, `-.-`); thick (`==>`, `===`); both edge-text syntaxes (`A -->|t| B`, `A -- t --> B`); multi-hop chain; edge length (`A ----> B`) |
| `labels` | plain, quoted, `<br>` breaks, HTML entities (`&lt;`,`&amp;`), backslash escapes, markdown-string (`` `**b**` ``), long label forcing 200px wrap, unicode + emoji |
| `subgraphs` | titled, untitled, nested, cross-subgraph edge, subgraph with own `direction LR`, isolated cluster, externally-connected cluster |
| `styling` | `classDef`+`class`, inline `style`, `linkStyle`, combined |
| `interactions` | `click` callback, `href` link, tooltip, `click` with callback+args |
| `a11y_directives` | `accTitle`/`accDescr`; `%%{init: {'theme':'forest'}}%%` directive |
| `integration` | ~5 realistic diagrams each combining ≥3 families (shapes+subgraphs+styling+edges+labels) |
| `stretch` (tagged) | 1 file documenting out-of-scope v11 `@{shape}`/icon axes as a comment; NOT expected to pass |

Estimated ~70–85 files.

## Naming & Integration

- Output path: `docs/sources/gen_flowchart_<family>_<case>.mmd`.
- The `gen_` prefix is a fresh namespace: no collision with `channel_*`,
  `example_*`, or bare `flowchart*.mmd`, and safe to clobber on regenerate.
- The eval globs `docs/sources/*.mmd` (`src/eval/samples.rs:40`), so each file's
  mermaid reference is rendered by `mmdc` automatically and enters the parity
  run with no additional wiring.

## Modes

- **generate** (default): regenerate the `gen_flowchart_*` set.
- `--validate`: after generating, render every emitted file with the `selkie`
  binary (`cargo run --bin selkie -- render <file>`), failing on any parse or
  render error. Catches generator bugs (malformed sources) before they reach the
  eval. Runs in CI-friendly batch; report per-file pass/fail.
- `--check`: regenerate into a temp dir and diff against the committed
  `gen_flowchart_*` files; exit non-zero if they differ (staleness gate,
  mirroring `generate-specs.py --check`). Intended for CI.

## Testing

- The generator is deterministic: same code → byte-identical output (enables
  `--check`).
- `--validate` is the correctness gate for the generated corpus: every file must
  render without error under selkie.
- A lightweight `scripts` self-test (pytest, run via `uv`) asserting: the
  registry contains `flowchart`; regeneration is idempotent (running twice
  yields the same files); every family emits ≥1 case; no two cases collide on
  filename.
- No Rust test changes required — corpus files are data consumed by the existing
  eval.

## CI

Add a `generate-corpus.py --check` step alongside the existing
`generate-specs.py --check` in the Lint job, so a stale committed corpus fails
CI. (Runs without cargo; fast.) `--validate` is heavier (renders via selkie) and
is run locally / on demand rather than in the lint gate initially.

## Template Extensibility

Adding a diagram type later:
1. Write `emit_<type>() -> list[Case]` with its own family functions.
2. Register it in `EMITTERS`.
3. `generate-corpus.py --type <type>` emits `gen_<type>_*.mmd`; the eval picks
   them up automatically.

The framework (CLI, naming, idempotency, validation, check) is unchanged.

## Risks & Mitigations

- **Generated diagram is invalid / selkie can't render it.** Mitigated by
  `--validate` gating every file through selkie before commit.
- **mmdc renders a feature selkie doesn't support, tanking that file's score.**
  Acceptable and desirable: it surfaces a real gap. The `stretch` file is the
  only deliberately-unsupported case and is tagged so it can be excluded from
  score aggregation if needed later. (Score-exclusion tagging is out of scope for
  this change.)
- **Corpus doubles eval runtime (more mmdc renders).** Acceptable; mmdc results
  are cached by the eval. If runtime becomes a problem, `--type flowchart`
  scoping and cache handle it.
