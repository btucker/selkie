<p align="center">
    <img src="docs/images/selkie-logo.png" width="250">
</p>

# Selkie

**⚠️ Still actively under development ⚠️**

A 100% Rust implementation of the [Mermaid](https://mermaid.js.org/) diagram parser and renderer.

Try it in your browser: **[btucker.github.io/selkie](https://btucker.github.io/selkie/)**

## About

Selkie aims to provide a fast, native alternative to Mermaid.js for parsing and rendering diagrams. The entire implementation is written in Rust, with no JavaScript dependencies at runtime.

This project has been built entirely with coding agents, mostly [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Development is guided by an evaluation system that compares Selkie's output against the reference Mermaid.js implementation, toward visual and structural parity.

## Performance

Selkie provides significant performance improvements over mermaid-js in both CLI and browser environments.

### CLI Benchmark

Compared to [mermaid-cli](https://github.com/mermaid-js/mermaid-cli) (`mmdc`):

| Diagram | mmdc | Selkie |
|---------|------|--------|
| Simple flowchart (5 nodes) | 1.53s | 6ms |
| Medium flowchart (15 nodes) | 1.54s | 7ms |
| Sequence diagram (4 actors) | 1.52s | 5ms |
| Class diagram (5 classes) | 1.55s | 5ms |
| Large flowchart (100 nodes) | 1.82s | 27ms |

_CLI-to-CLI comparison. Median of 5 runs after 2 warmup runs._

The dramatic speedup comes from avoiding the browser entirely—mermaid-cli spawns Puppeteer + Chromium for each render.

### Browser Benchmark

For client-side rendering, Selkie compiles to WebAssembly. Both run in the same Chromium browser for a fair comparison. [Run it yourself →](https://btucker.github.io/selkie/benchmark.html)

| Diagram | Mermaid.js | Selkie WASM |
|---------|------------|-------------|
| Simple flowchart (5 nodes) | 10ms | 1ms |
| Medium flowchart (15 nodes) | 21ms | 1.6ms |
| Sequence diagram (4 actors) | 4.5ms | 0.3ms |
| Class diagram (5 classes) | 16ms | 0.5ms |
| State diagram (8 states) | 22ms | 0.75ms |
| Pie chart (5 slices) | 1.6ms | 0.1ms |

_Median of 10 runs after 2 warmup runs. Chromium via Playwright._

**Bundle Size:** ~350 KB gzipped (WASM + JS glue) vs ~2.5 MB for mermaid.min.js

## Credits

Selkie could not exist without all the human effort that has gone into these excellent projects:

- **[Mermaid](https://github.com/mermaid-js/mermaid)** - The original JavaScript diagramming library that defines the syntax and rendering we aim to match
- **[Dagre](https://github.com/dagrejs/dagre)** - Graph layout algorithms that inspire our layout engine
- **[ELK](https://github.com/kieler/elkjs)** - Eclipse Layout Kernel, providing additional layout strategies

## Supported Diagram Types

Selkie supports parsing and rendering for all major Mermaid diagram types.

| Type | Example |
|------|---------|
| **Flowchart** | <img src="docs/images/flowchart_complex.svg" alt="Flowchart" width="500"> |
| **Sequence** | <img src="docs/images/sequence.svg" alt="Sequence Diagram" width="500"> |
| **Class** | <img src="docs/images/class.svg" alt="Class Diagram" width="500"> |
| **State** | <img src="docs/images/state_complex.svg" alt="State Diagram" width="400"> |
| **ER** | <img src="docs/images/er.svg" alt="ER Diagram" width="500"> |
| **Gantt** | <img src="docs/images/gantt.svg" alt="Gantt Chart" width="500"> |
| **Pie** | <img src="docs/images/pie.svg" alt="Pie Chart" width="300"> |
| **Architecture** | <img src="docs/images/architecture_complex.svg" alt="Architecture Diagram" width="500"> |
| **Git Graph** | <img src="docs/images/git_complex.svg" alt="Git Graph" width="500"> |

<details>
<summary><strong>Parser Only</strong> (rendering in progress)</summary>

| Type | Description |
|------|-------------|
| Mindmap | Hierarchical mindmaps |
| Timeline | Timeline visualizations |
| Quadrant | Quadrant charts |
| XY Chart | Line and bar charts |
| Sankey | Flow diagrams with proportional widths |
| Requirement | Requirements diagrams |
| C4 | C4 architecture diagrams |
| Block | Block diagrams |
| Packet | Network packet diagrams |
| Kanban | Kanban boards |
| Journey | User journey maps |
| Radar | Radar/spider charts |
| Treemap | Treemap visualizations |

</details>

## Installation

```bash
cargo install selkie
```

Or build from source:

```bash
git clone https://github.com/btucker/selkie
cd selkie
cargo build --release
```

## Usage

### Command Line

```bash
# Render a diagram to SVG
selkie render -i diagram.mmd -o output.svg

# Shorthand (implicit render command)
selkie -i diagram.mmd -o output.svg

# Read from stdin, write to stdout
cat diagram.mmd | selkie -i - -o -

# Use a specific theme
selkie -i diagram.mmd -o output.svg --theme dark

# Output to PNG (requires 'png' feature)
selkie -i diagram.mmd -o output.png
```

### Evaluation System

Selkie includes a built-in evaluation system that compares output against Mermaid.js. See [EVAL.md](EVAL.md) for detailed documentation.

```bash
# Run evaluation with built-in samples
selkie eval

# Evaluate specific diagram types
selkie eval --type flowchart

# Output to custom directory
selkie eval -o ./reports

# Show detailed per-diagram diffs
selkie eval --verbose
```

The eval system generates an HTML report with:
- **Structural comparison** - Node/edge counts, labels, connections
- **Visual similarity** - SSIM-based image comparison
- **Side-by-side PNGs** - Selkie output next to Mermaid.js reference

Requires [Mermaid CLI](https://github.com/mermaid-js/mermaid-cli) for reference rendering (`npm install -g @mermaid-js/mermaid-cli`).

### As a Library

```rust
use mermaid::{parse, render};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let diagram_source = r#"
        flowchart LR
            A[Start] --> B{Decision}
            B -->|Yes| C[OK]
            B -->|No| D[Cancel]
    "#;

    let diagram = parse(diagram_source)?;
    let svg = render(&diagram)?;

    println!("{}", svg);
    Ok(())
}
```

### WebAssembly (Browser)

Selkie can be compiled to WebAssembly for client-side rendering in the browser. The WASM entrypoint mirrors the mermaid-js API (`initialize`, `parse`, and `render`) and also exposes `render_text` for a minimal wrapper.

```bash
# Build the WASM package (requires wasm-bindgen / wasm-pack)
wasm-pack build --target web --features wasm
```

```js
import init, { initialize, parse, render } from "./pkg/selkie.js";

await init();
initialize({ startOnLoad: false });
parse(`flowchart TD; A-->B;`);
const { svg } = render("diagram1", `flowchart TD; A-->B;`);
document.body.innerHTML = svg;
```

## Feature Flags

Selkie uses Cargo feature flags to enable optional functionality. This keeps the core library lightweight while allowing additional capabilities when needed.

### Default Features

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `cli` | Command line interface | [clap](https://crates.io/crates/clap) |

The CLI is enabled by default. To build only the library without CLI:

```bash
cargo build --release --no-default-features
```

### Output Formats

SVG output is always available with no additional dependencies:

```bash
selkie -i diagram.mmd -o output.svg
```

Additional output formats require feature flags:

| Feature | Format | Dependencies |
|---------|--------|--------------|
| _(none)_ | SVG | _(built-in)_ |
| `png` | PNG | [resvg](https://crates.io/crates/resvg) |
| `pdf` | PDF | [svg2pdf](https://crates.io/crates/svg2pdf), resvg |
| `kitty` | Terminal inline | resvg, [image](https://crates.io/crates/image), [base64](https://crates.io/crates/base64), libc, atty |
| `wasm` | WebAssembly bindings | [wasm-bindgen](https://crates.io/crates/wasm-bindgen) |
| `all-formats` | All of the above | All of the above |

### Usage Examples

```bash
# Build with PNG support
cargo build --release --features png

# Build with all output formats
cargo build --release --features all-formats

# Install with PDF support
cargo install selkie --features pdf

# Library only (no CLI, minimal dependencies)
cargo build --release --no-default-features
```

### Feature Details

#### `cli`

Provides the `selkie` command-line binary with subcommands for rendering and evaluation. Without this feature, only the library is built.

#### `png`

Enables PNG output via the `resvg` crate, a high-quality SVG rendering library. Use with:

```bash
selkie -i diagram.mmd -o output.png
```

#### `pdf`

Enables PDF output via `svg2pdf`. Useful for generating print-ready documents:

```bash
selkie -i diagram.mmd -o output.pdf
```

#### `kitty`

Enables inline image display in terminals that support the Kitty graphics protocol (Kitty, Ghostty, WezTerm). When enabled, diagrams can be rendered directly in the terminal:

```bash
selkie -i diagram.mmd  # Displays inline if terminal supports it
```

#### `wasm`

Enables WebAssembly bindings for browser usage. Build with:

```bash
wasm-pack build --target web --features wasm
```

#### `all-formats`

Convenience feature that enables `png`, `pdf`, and `kitty` together. Best for development or when you need maximum flexibility:

```bash
cargo install selkie --features all-formats
```

## Issue Tracking

This project uses [Beads](https://github.com/steveyegge/beads) for issue tracking - an AI-native issue tracker that lives directly in the repository. Issues are stored in `.beads/` and sync with git, making them accessible to both humans and AI coding agents.

```bash
# View available work
bd ready

# View issue details
bd show <issue-id>

# Update issue status
bd update <issue-id> --status in_progress
bd close <issue-id>

# Sync with remote
bd sync
```

## Development

This project follows test-driven development. Run the test suite:

```bash
cargo test
```

Run the evaluation to check parity with Mermaid.js:

```bash
cargo run -- eval
```

## License

MIT License - see [LICENSE](LICENSE) for details.
