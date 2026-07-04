# Selkie Specifications

This file is generated from `@spec` annotations. Do not edit it manually.

## FLOW

### FLOW-1.1

When a flowchart declares a named subgraph, the application shall preserve the subgraph title text in the parsed model.

Source: `src/diagrams/flowchart/parser.rs:826`

### FLOW-1.2

When a subgraph declares its own direction, the application shall preserve that direction without changing the parent flowchart direction.

Source: `src/render/flowchart.rs:572`

### FLOW-1.3

When a flowchart contains subgraph member nodes, the application shall preserve parent-child relationships in the layout graph.

Source: `src/render/flowchart.rs:334`

### FLOW-1.4

When a rendered flowchart reference contains subgraph titles that Selkie omits, the eval report shall group the failures under the visible subgraph titles requirement.

Source: `src/eval/failure_families.rs:294`

### FLOW-1.5

When a flowchart edge uses a subgraph ID as an endpoint, the application shall route the edge to the subgraph container without rendering a duplicate node for that subgraph ID.

Source: `tests/render_integration.rs:471`

### FLOW-2.1

When a flowchart edge has a Mermaid label, the application shall preserve that label in the layout graph edge model.

Source: `src/render/flowchart.rs:406`

### FLOW-2.2

When an ASCII flowchart edge label is placed near a diamond node, the application shall render the full edge label text without truncation.

Source: `tests/flowchart_edge_label_truncation.rs:3`

### FLOW-2.3

When Mermaid renders a flowchart edge label as layout text, the eval report shall group missing edge-label text under the edge label visibility requirement.

Source: `src/eval/failure_families.rs:344`

### FLOW-2.4

When Selkie renders flowchart label text with raw HTML tags, double-escaped entities, or unnormalized Mermaid escapes, the eval report shall flag the label as a visible markup artifact.

Source: `src/eval/checks.rs:3262`

### FLOW-3.1

When a flowchart edge is rendered to SVG, the application shall emit an SVG path for the edge route.

Source: `src/render/flowchart.rs:532`

### FLOW-3.2

When a flowchart's major rendered orientation differs from Mermaid, the eval report shall group the failure under the orientation preservation requirement.

Source: `src/eval/failure_families.rs:388`

### FLOW-3.3

When flowchart edge routes differ from Mermaid, the eval report shall group the failure under the edge routing preservation requirement.

Source: `src/eval/failure_families.rs:389`

### FLOW-3.4

When Mermaid lays out a flowchart with direction and node/rank spacing, the eval report shall group dimension drift under the layout sizing requirement.

Source: `src/eval/failure_families.rs:414`

### FLOW-4.1

When a Mermaid flowchart applies a class to nodes, the application shall preserve the class assignment in the parsed node model.

Source: `src/diagrams/flowchart/parser.rs:2040`

### FLOW-4.2

When flowchart styling differs from Mermaid stroke or color behavior, the eval report shall group the failure under the visual styling requirement.

Source: `src/eval/failure_families.rs:435`
