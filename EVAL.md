# Selkie Evaluation System

The evaluation system is a core component of Selkie's development process. It compares Selkie's output against the reference Mermaid.js implementation, providing automated feedback on parsing and rendering parity.

## Overview

The eval system serves as the primary guidance mechanism for Claude Code during development. By quantifying differences between Selkie and Mermaid.js outputs, it enables targeted improvements and prevents regressions.

```
┌─────────────────────────────────────────────────────────────┐
│                    Evaluation Pipeline                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   Diagram Source                                             │
│        │                                                     │
│        ├──────────────────┬──────────────────┐              │
│        ▼                  ▼                  │              │
│   ┌─────────┐       ┌──────────┐             │              │
│   │ Selkie  │       │ Mermaid  │             │              │
│   │ Parser  │       │   .js    │             │              │
│   └────┬────┘       └────┬─────┘             │              │
│        │                 │                   │              │
│        ▼                 ▼                   │              │
│   ┌─────────┐       ┌──────────┐             │              │
│   │ Selkie  │       │ Reference│             │              │
│   │ Render  │       │   SVG    │◄── Cache    │              │
│   └────┬────┘       └────┬─────┘             │              │
│        │                 │                   │              │
│        └────────┬────────┘                   │              │
│                 ▼                            │              │
│        ┌───────────────┐                     │              │
│        │   Structural  │                     │              │
│        │   Comparison  │                     │              │
│        └───────┬───────┘                     │              │
│                │                             │              │
│                ▼                             │              │
│        ┌───────────────┐                     │              │
│        │    Visual     │                     │              │
│        │     SSIM      │                     │              │
│        └───────┬───────┘                     │              │
│                │                             │              │
│                ▼                             │              │
│        ┌───────────────┐                     │              │
│        │    Report     │                     │              │
│        │  Generation   │                     │              │
│        └───────────────┘                     │              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

```bash
# Run evaluation with built-in samples (outputs to /tmp/selkie-eval-XXXX/)
selkie eval

# Evaluate specific diagram types
selkie eval --type flowchart

# Evaluate custom diagram files
selkie eval ./diagrams/

# Custom output directory (creates selkie-eval-XXXX/ subdirectory)
selkie eval -o ./reports

# Also generate JSON report
selkie eval --json results.json
```

## Three-Level Issue Classification

The eval system classifies issues into three severity levels:

### Error (Structural Breaks)

Issues that indicate the diagram is functionally incorrect:

- **Node count mismatch** - Different number of nodes between Selkie and reference
- **Edge count mismatch** - Different number of connections
- **Missing labels** - Labels present in reference but absent in Selkie

Errors indicate bugs that must be fixed. A diagram with errors is not considered "matching."

### Warning (Significant Differences)

Issues that may cause noticeable visual differences:

- **Dimension mismatch >20%** - Width or height differs significantly
- **Shape count differences** - Different numbers of rects, circles, paths, etc.

Warnings indicate areas for improvement but don't necessarily mean the diagram is wrong.

### Info (Acceptable Variations)

Minor differences that are often intentional:

- **Extra labels** - Additional labels in Selkie (may be intentional annotations)
- **Dimension mismatch 5-20%** - Minor sizing differences
- **Marker differences** - Different arrow/endpoint styling

Info issues are logged but don't affect the matching status.

## Structural Comparison

The eval system extracts structural information from SVG output:

| Metric | Description |
|--------|-------------|
| Node count | Number of diagram nodes (shapes with content) |
| Edge count | Number of connections/arrows between nodes |
| Labels | Text content within the diagram |
| Dimensions | Overall SVG width and height |
| Shape counts | Count of each SVG element type (rect, circle, path, etc.) |
| Markers | Arrow heads and other endpoint decorations |

### SVG Structure Extraction

```rust
pub struct SvgStructure {
    pub width: f64,
    pub height: f64,
    pub node_count: usize,
    pub edge_count: usize,
    pub labels: Vec<String>,
    pub shapes: ShapeCounts,
    pub marker_count: usize,
}
```

## Visual Similarity (SSIM)

For more nuanced comparison, the eval system calculates SSIM (Structural Similarity Index) when PNG output is available:

- **SSIM = 1.0** - Identical images
- **SSIM > 0.95** - Very similar (minor pixel differences)
- **SSIM > 0.90** - Similar (acceptable visual variation)
- **SSIM < 0.90** - Noticeable differences

The SSIM implementation follows the algorithm from:

> Wang, Z., Bovik, A. C., Sheikh, H. R., & Simoncelli, E. P. (2004).
> "Image quality assessment: from error visibility to structural similarity"

## Reference Caching

To avoid repeatedly calling Mermaid.js, the eval system caches reference SVGs:

- Cache location: `~/.cache/selkie/references/`
- Cache key: Content hash of the diagram source (whitespace-normalized)
- Format: `{hash}.svg`

```bash
# Force re-render all references (ignore cache)
selkie eval --force-refresh

# Clear the cache
rm -rf ~/.cache/selkie/references/
```

## Report Formats

### Text Report

Terminal-friendly output with progress bars and issue summaries:

```
Selkie Evaluation Report
========================

Overall Parity: 85.0% (17/20 diagrams match reference)

By Diagram Type:
  flowchart    ████████░░  80% (4/5)
  sequence     ██████████  100% (5/5)
  pie          ████████░░  80% (4/5)
  class        ██████████  100% (3/3)

Issues Summary:
    3 Error    - Structural breaks
    5 Warning  - Significant differences
    8 Info     - Acceptable variations
```

### JSON Report

Machine-readable format for CI integration:

```json
{
  "total": 20,
  "matching": 17,
  "parity_percent": 85.0,
  "by_type": {
    "flowchart": { "total": 5, "matching": 4, "parity_percent": 80.0 }
  },
  "diagrams": [
    {
      "name": "flowchart_basic",
      "status": "match",
      "structural_match": true,
      "issues": []
    }
  ]
}
```

### HTML Report

Visual comparison report with:
- Summary statistics
- Per-diagram status cards
- Issue details with severity highlighting
- Side-by-side SVG comparisons

The HTML report is always generated as `index.html` in the output directory:

```bash
selkie eval                    # Creates /tmp/selkie-eval-XXXX/index.html
selkie eval -o ./reports       # Creates ./reports/selkie-eval-XXXX/index.html
open /tmp/selkie-eval-*/index.html
```

## Output Directory Structure

Each eval run creates a unique directory with assets organized by diagram type:

```
selkie-eval-a1b2c3d4/
├── index.html                      # Main HTML report
├── manifest.json                   # PNG manifest (if png feature enabled)
├── flowchart/
│   ├── basic_selkie.svg            # Selkie-rendered SVG
│   ├── basic_reference.svg         # Mermaid.js reference SVG
│   ├── basic.png                   # Side-by-side comparison
│   ├── styled_selkie.svg
│   ├── styled_reference.svg
│   └── styled.png
├── sequence/
│   ├── simple_selkie.svg
│   ├── simple_reference.svg
│   └── simple.png
├── pie/
│   └── ...
└── state/
    └── ...
```

The output path is shown at the end of the evaluation:

```
Evaluation report written to: /tmp/selkie-eval-a1b2c3d4
```

## Built-in Samples

The eval system includes built-in sample diagrams covering:

- Basic flowcharts with various shapes
- Sequence diagrams with participants and messages
- Pie charts with multiple segments
- Class diagrams with inheritance
- State diagrams with transitions
- ER diagrams with relationships
- Gantt charts with tasks

```bash
# Evaluate only flowchart samples
selkie eval --type flowchart
```

## Integration with Development

The eval system is designed to work with Claude Code's development workflow:

1. **Before changes** - Run eval to establish baseline
2. **During development** - Run eval to check progress
3. **After changes** - Run eval to verify no regressions

```bash
# Quick check during development
selkie eval --type flowchart

# Full evaluation before commit
selkie eval --json baseline.json
```

## Configuration

The evaluation runner accepts configuration options:

```rust
pub struct EvalConfig {
    /// Filter by diagram type
    pub diagram_type_filter: Option<String>,

    /// Skip visual comparison (faster)
    pub skip_visual: bool,

    /// Structural check thresholds
    pub check_config: CheckConfig,
}

pub struct CheckConfig {
    /// Dimension warning threshold (default: 20%)
    pub dimension_warning_threshold: f64,

    /// Dimension info threshold (default: 5%)
    pub dimension_info_threshold: f64,
}
```

## Requirements

The eval system requires [Mermaid CLI](https://github.com/mermaid-js/mermaid-cli) (`mmdc`) to render reference SVGs:

```bash
# Install mermaid-cli globally
npm install -g @mermaid-js/mermaid-cli

# Verify setup
mmdc --version
```

## Troubleshooting

### "mmdc is not installed" or "mmdc not found"

Install the Mermaid CLI:

```bash
npm install -g @mermaid-js/mermaid-cli
```

### High cache disk usage

Clear old cache entries:

```bash
rm -rf ~/.cache/selkie/references/
```

### Inconsistent results

Force refresh the reference cache:

```bash
selkie eval --force-refresh
```
