//! TUI output evaluation checks.
//!
//! Parses TUI character-art output to extract structural and spatial
//! information, then compares it against the layout graph (ground truth).
//!
//! Unlike SVG eval (which compares selkie SVG vs mermaid.js reference SVG),
//! TUI eval compares the rendered TUI output against the positioned layout
//! graph, since there is no mermaid.js TUI reference.

use super::Issue;
use crate::layout::{LayoutGraph, NodeShape};

/// Structure extracted from TUI character-art output.
#[derive(Debug, Clone)]
pub struct TuiStructure {
    /// Node labels found (text inside box-drawing rectangles).
    pub labels: Vec<String>,
    /// Approximate row position of each label's center.
    pub label_positions: Vec<(usize, usize)>, // (row, col)
    /// Number of arrow tip characters found (▶▼◀▲).
    pub arrow_count: usize,
    /// Whether braille characters were found (indicating edge routing).
    pub has_braille: bool,
    /// Number of braille characters found.
    pub braille_count: usize,
    /// Edge labels found (text not inside box-drawing rectangles).
    pub edge_labels: Vec<String>,
    /// Canvas dimensions (rows, cols).
    pub dimensions: (usize, usize),
    /// Raw output string for substring matching (e.g., subgraph labels).
    pub raw_output: String,
}

/// Characters that indicate box-drawing borders.
const BOX_HORIZONTAL: &[char] = &['─'];
const BOX_VERTICAL: &[char] = &['│'];
const BOX_CORNERS: &[char] = &['┌', '┐', '└', '┘', '╭', '╮', '╰', '╯'];
const ARROW_TIPS: &[char] = &['▶', '▼', '◀', '▲'];

/// Parse TUI character-art output into a structural representation.
pub fn parse_tui(output: &str) -> TuiStructure {
    let raw_output = output.to_string();
    let lines: Vec<&str> = output.lines().collect();
    let rows = lines.len();
    let cols = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);

    // Convert to a 2D char grid for easier analysis
    let grid: Vec<Vec<char>> = lines
        .iter()
        .map(|line| {
            let mut chars: Vec<char> = line.chars().collect();
            chars.resize(cols, ' ');
            chars
        })
        .collect();

    let labels = extract_node_labels(&grid);
    let label_positions = find_label_positions(&grid, &labels);
    let arrow_count = count_arrows(&grid);
    let (has_braille, braille_count) = count_braille(&grid);
    let edge_labels = extract_edge_labels(&grid, &labels);

    TuiStructure {
        labels,
        label_positions,
        arrow_count,
        has_braille,
        braille_count,
        edge_labels,
        dimensions: (rows, cols),
        raw_output,
    }
}

/// Extract node labels by finding text between │ characters on rows
/// that are bounded above and below by box-drawing horizontal borders.
/// Also extracts diamond labels (text between / and \ on the widest row).
fn extract_node_labels(grid: &[Vec<char>]) -> Vec<String> {
    let mut labels = Vec::new();

    for (row_idx, row) in grid.iter().enumerate() {
        // Look for │...text...│ patterns (rectangles)
        let mut in_box = false;
        let mut text_start = 0;

        for (col_idx, &ch) in row.iter().enumerate() {
            if BOX_VERTICAL.contains(&ch) {
                if in_box {
                    // End of box content — extract text
                    let content: String = row[text_start..col_idx]
                        .iter()
                        .filter(|ch| {
                            !ARROW_TIPS.contains(ch) && !('\u{2800}'..='\u{28FF}').contains(ch)
                        })
                        .collect();
                    let trimmed = content.trim();
                    if !trimmed.is_empty() && is_inside_box(grid, row_idx, text_start, col_idx) {
                        labels.push(trimmed.to_string());
                    }
                    in_box = false;
                } else {
                    // Start of box content
                    in_box = true;
                    text_start = col_idx + 1;
                }
            }
        }

        // Look for /...text...\ patterns (diamonds — widest row)
        if let (Some(slash_pos), Some(backslash_pos)) = (
            row.iter().position(|&c| c == '/'),
            row.iter().rposition(|&c| c == '\\'),
        ) {
            if backslash_pos > slash_pos + 1 {
                let content: String = row[slash_pos + 1..backslash_pos]
                    .iter()
                    .filter(|ch| {
                        !ARROW_TIPS.contains(ch) && !('\u{2800}'..='\u{28FF}').contains(ch)
                    })
                    .collect();
                let trimmed = content.trim();
                if !trimmed.is_empty() && trimmed.len() > 1 {
                    // Verify it's a diamond: check for /\ above and \/ below
                    let has_top = row_idx > 0 && {
                        let above: String = grid[row_idx - 1].iter().collect();
                        above.contains('/') && above.contains('\\')
                    };
                    let has_bottom = row_idx + 1 < grid.len() && {
                        let below: String = grid[row_idx + 1].iter().collect();
                        below.contains('\\') && below.contains('/')
                    };
                    if has_top || has_bottom {
                        labels.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    // Deduplicate (a label might appear on multiple rows if box is tall)
    labels.sort();
    labels.dedup();
    labels
}

/// Check if a row segment is inside a box by verifying that the segment
/// is bounded by │ characters on the same row (already the case when called
/// from `extract_node_labels`), and that there are horizontal borders
/// directly above and below without gaps (i.e., every row between the text
/// and the border also has │ at the boundary columns).
fn is_inside_box(grid: &[Vec<char>], row: usize, col_start: usize, col_end: usize) -> bool {
    // col_start is the char after the opening │, col_end is the closing │
    // Check that the opening and closing │ exist on this row
    if col_start == 0 || col_end >= grid[0].len() {
        return false;
    }
    let left_border = col_start - 1;
    let right_border = col_end;

    if !BOX_VERTICAL.contains(&grid[row][left_border])
        || !BOX_VERTICAL.contains(&grid[row][right_border])
    {
        return false;
    }

    // Look for horizontal border directly above (within 3 rows)
    let has_top = if row > 0 {
        let search_start = row.saturating_sub(3);
        grid[search_start..row].iter().rev().any(|r| {
            left_border < r.len()
                && (BOX_CORNERS.contains(&r[left_border])
                    || BOX_HORIZONTAL.contains(&r[left_border]))
        })
    } else {
        false
    };

    // Look for horizontal border directly below (within 3 rows)
    let has_bottom = if row + 1 < grid.len() {
        let search_end = (row + 4).min(grid.len());
        grid[row + 1..search_end].iter().any(|r| {
            left_border < r.len()
                && (BOX_CORNERS.contains(&r[left_border])
                    || BOX_HORIZONTAL.contains(&r[left_border]))
        })
    } else {
        false
    };

    has_top && has_bottom
}

/// Find the (row, col) position of each label in the grid.
fn find_label_positions(grid: &[Vec<char>], labels: &[String]) -> Vec<(usize, usize)> {
    let mut positions = Vec::new();

    for label in labels {
        let mut found = false;
        for (row_idx, row) in grid.iter().enumerate() {
            let row_str: String = row.iter().collect();
            if let Some(col_idx) = row_str.find(label.as_str()) {
                positions.push((row_idx, col_idx + label.len() / 2));
                found = true;
                break;
            }
        }
        if !found {
            positions.push((0, 0)); // fallback
        }
    }

    positions
}

/// Count arrow tip characters in the grid.
fn count_arrows(grid: &[Vec<char>]) -> usize {
    grid.iter()
        .flat_map(|row| row.iter())
        .filter(|ch| ARROW_TIPS.contains(ch))
        .count()
}

/// Count braille characters in the grid.
fn count_braille(grid: &[Vec<char>]) -> (bool, usize) {
    let count = grid
        .iter()
        .flat_map(|row| row.iter())
        .filter(|ch| ('\u{2800}'..='\u{28FF}').contains(ch))
        .count();
    (count > 0, count)
}

/// Extract edge labels: text that appears in the output but isn't a node label
/// and isn't part of box-drawing characters. These are typically short strings
/// placed at edge midpoints.
fn extract_edge_labels(grid: &[Vec<char>], node_labels: &[String]) -> Vec<String> {
    let mut edge_labels = Vec::new();

    for (row_idx, row) in grid.iter().enumerate() {
        // Find contiguous runs of alphabetic/space chars not in boxes
        let mut run_start = None;
        for (col_idx, &ch) in row.iter().enumerate() {
            let is_text = ch.is_alphanumeric() || ch == ' ';
            let is_box = BOX_HORIZONTAL.contains(&ch)
                || BOX_VERTICAL.contains(&ch)
                || BOX_CORNERS.contains(&ch)
                || ARROW_TIPS.contains(&ch)
                || ('\u{2800}'..='\u{28FF}').contains(&ch);

            if is_text && !is_box {
                if run_start.is_none() {
                    run_start = Some(col_idx);
                }
            } else if let Some(start) = run_start {
                let text: String = row[start..col_idx].iter().collect();
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty()
                    && !node_labels.contains(&trimmed)
                    && !is_inside_box(grid, row_idx, start, col_idx)
                {
                    edge_labels.push(trimmed);
                }
                run_start = None;
            }
        }
        // Handle run extending to end of line
        if let Some(start) = run_start {
            let text: String = row[start..].iter().collect();
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty()
                && !node_labels.contains(&trimmed)
                && !is_inside_box(grid, row_idx, start, row.len())
            {
                edge_labels.push(trimmed);
            }
        }
    }

    edge_labels.sort();
    edge_labels.dedup();
    edge_labels
}

/// Run TUI-specific structural checks comparing TUI output against the layout graph.
pub fn check_tui_structure(tui: &TuiStructure, graph: &LayoutGraph) -> Vec<Issue> {
    let mut issues = Vec::new();

    check_tui_node_count(tui, graph, &mut issues);
    check_tui_labels(tui, graph, &mut issues);
    check_tui_edges(tui, graph, &mut issues);
    check_tui_spatial_order(tui, graph, &mut issues);

    issues
}

/// Run ER-specific TUI checks comparing output against the ER database.
///
/// Checks that entity attributes (type, name, keys) are present in the TUI
/// output and that table-structure characters (├ ┬ ┴) are used for entities
/// with attributes.
pub fn check_er_tui_structure(tui: &TuiStructure, db: &crate::diagrams::er::ErDb) -> Vec<Issue> {
    let mut issues = Vec::new();

    let entities = db.get_entities();

    for (name, entity) in entities {
        // Check entity name is present
        if !tui.raw_output.contains(name.as_str()) {
            issues.push(Issue::error(
                "er_tui_missing_entity",
                format!("ER TUI output missing entity: '{}'", name),
            ));
            continue;
        }

        // Check attributes are rendered
        for attr in &entity.attributes {
            if !tui.raw_output.contains(&attr.attr_type) {
                issues.push(Issue::warning(
                    "er_tui_missing_attr_type",
                    format!(
                        "ER TUI output missing attribute type '{}' for entity '{}'",
                        attr.attr_type, name
                    ),
                ));
            }
            if !tui.raw_output.contains(&attr.name) {
                issues.push(Issue::warning(
                    "er_tui_missing_attr_name",
                    format!(
                        "ER TUI output missing attribute name '{}' for entity '{}'",
                        attr.name, name
                    ),
                ));
            }
            for key in &attr.keys {
                if !tui.raw_output.contains(key.as_str()) {
                    issues.push(Issue::warning(
                        "er_tui_missing_attr_key",
                        format!(
                            "ER TUI output missing key '{}' for {}.{}",
                            key.as_str(),
                            name,
                            attr.name
                        ),
                    ));
                }
            }
        }

        // Check table structure for entities with attributes
        if !entity.attributes.is_empty() {
            if !tui.raw_output.contains('├') {
                issues.push(Issue::warning(
                    "er_tui_no_table_divider",
                    "ER TUI output missing table divider '├' for entities with attributes"
                        .to_string(),
                ));
            }
            if !tui.raw_output.contains('┬') {
                issues.push(Issue::warning(
                    "er_tui_no_column_separator",
                    "ER TUI output missing column separator '┬' for attribute tables".to_string(),
                ));
            }
        }
    }

    // Check relationship labels
    for rel in db.get_relationships() {
        if !rel.role_a.is_empty() && !tui.raw_output.contains(&rel.role_a) {
            issues.push(Issue::warning(
                "er_tui_missing_rel_label",
                format!("ER TUI output missing relationship label: '{}'", rel.role_a),
            ));
        }
    }

    issues
}

/// Check that TUI output has the correct number of nodes.
/// Counts nodes whose labels appear anywhere in the TUI output (boxes, free text, etc.).
fn check_tui_node_count(tui: &TuiStructure, graph: &LayoutGraph, issues: &mut Vec<Issue>) {
    let expected = graph.nodes.iter().filter(|n| !n.is_dummy).count();
    // Count nodes found: either in box labels or in the raw output
    let mut found = 0;
    for node in &graph.nodes {
        if node.is_dummy {
            continue;
        }
        if is_symbol_node(node) {
            // Circle/DoubleCircle nodes render as ●/◉ symbols, not text labels
            found += 1;
            continue;
        }
        let label = clean_label(node.label.as_deref().unwrap_or(&node.id));
        if tui.labels.contains(&label) || tui.raw_output.contains(&label) {
            found += 1;
        }
    }

    if found != expected {
        issues.push(
            Issue::error(
                "tui_node_count",
                format!(
                    "TUI node count mismatch: expected {}, found {}",
                    expected, found
                ),
            )
            .with_values(expected.to_string(), found.to_string()),
        );
    }
}

/// Check if a node renders as a symbol (●/◉) rather than a text label.
/// Circle and DoubleCircle nodes with empty or no label are start/end markers.
fn is_symbol_node(node: &crate::layout::LayoutNode) -> bool {
    matches!(node.shape, NodeShape::Circle | NodeShape::DoubleCircle)
        && node.label.as_deref().is_none_or(|l| l.is_empty())
}

/// Clean HTML line breaks from labels (matches TUI renderer behavior).
/// Also normalizes whitespace to single spaces.
fn clean_label(raw: &str) -> String {
    let cleaned = raw.replace("<br/>", " ").replace("<br>", " ");
    // Normalize multiple spaces to single space
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Check that all node labels from the layout graph appear in the TUI output.
/// Subgraph labels appear as free text (not inside boxes), so we check the
/// raw output string as well.
fn check_tui_labels(tui: &TuiStructure, graph: &LayoutGraph, issues: &mut Vec<Issue>) {
    let tui_labels: std::collections::HashSet<&str> =
        tui.labels.iter().map(|s| s.as_str()).collect();

    // Also collect all text from the raw grid for subgraph label matching
    let tui_all_labels: std::collections::HashSet<String> = tui
        .labels
        .iter()
        .cloned()
        .chain(tui.edge_labels.iter().cloned())
        .collect();

    for node in &graph.nodes {
        if node.is_dummy {
            continue;
        }
        // Circle/DoubleCircle nodes render as symbols (●/◉), not text labels.
        // We verify their symbols exist in the raw output instead of checking by ID.
        if is_symbol_node(node) {
            let symbol = if node.shape == NodeShape::DoubleCircle {
                '◉'
            } else {
                '●'
            };
            if !tui.raw_output.contains(symbol) {
                issues.push(Issue::error(
                    "tui_missing_label",
                    format!(
                        "TUI output missing {} symbol for node '{}'",
                        symbol, node.id
                    ),
                ));
            }
            continue;
        }
        let raw_label = node.label.as_deref().unwrap_or(&node.id);
        let label = clean_label(raw_label);
        // Check in box labels, edge labels, and raw output (for subgraph labels)
        if !tui_labels.contains(label.as_str())
            && !tui_all_labels.contains(&label)
            && !tui.raw_output.contains(&label)
        {
            issues.push(Issue::error(
                "tui_missing_label",
                format!("TUI output missing node label: '{}'", label),
            ));
        }
    }
}

/// Check that edges are rendered (arrows and/or braille dots present).
fn check_tui_edges(tui: &TuiStructure, graph: &LayoutGraph, issues: &mut Vec<Issue>) {
    let expected_edges = graph.edges.len();

    if expected_edges > 0 && tui.arrow_count == 0 && !tui.has_braille {
        issues.push(Issue::error(
            "tui_no_edges",
            format!(
                "TUI output has no edges rendered (expected {} edges, found 0 arrows and 0 braille chars)",
                expected_edges
            ),
        ));
    }

    // Arrow count should roughly match edge count (each edge has one arrow tip)
    if expected_edges > 0 && tui.arrow_count > 0 && tui.arrow_count != expected_edges {
        issues.push(
            Issue::warning(
                "tui_arrow_count",
                format!(
                    "TUI arrow count differs from edge count: expected {}, found {}",
                    expected_edges, tui.arrow_count
                ),
            )
            .with_values(expected_edges.to_string(), tui.arrow_count.to_string()),
        );
    }
}

/// Check spatial ordering: if node A is above node B in the layout,
/// it should be above in the TUI output too.
fn check_tui_spatial_order(tui: &TuiStructure, graph: &LayoutGraph, issues: &mut Vec<Issue>) {
    // Build a map of label → layout y position
    let mut layout_positions: Vec<(String, f64)> = Vec::new();
    for node in &graph.nodes {
        if node.is_dummy {
            continue;
        }
        if let Some(y) = node.y {
            let label = clean_label(node.label.as_deref().unwrap_or(&node.id));
            layout_positions.push((label, y));
        }
    }

    // Build a map of label → TUI row position
    let tui_positions: std::collections::HashMap<&str, usize> = tui
        .labels
        .iter()
        .zip(tui.label_positions.iter())
        .map(|(label, &(row, _col))| (label.as_str(), row))
        .collect();

    // Check pairwise ordering
    let mut ordering_violations = 0;
    for i in 0..layout_positions.len() {
        for j in (i + 1)..layout_positions.len() {
            let (ref label_a, y_a) = layout_positions[i];
            let (ref label_b, y_b) = layout_positions[j];

            // Skip if same y (horizontal arrangement doesn't need vertical ordering)
            if (y_a - y_b).abs() < 1.0 {
                continue;
            }

            let layout_a_above = y_a < y_b;

            if let (Some(&tui_row_a), Some(&tui_row_b)) = (
                tui_positions.get(label_a.as_str()),
                tui_positions.get(label_b.as_str()),
            ) {
                let tui_a_above = tui_row_a < tui_row_b;
                if layout_a_above != tui_a_above {
                    ordering_violations += 1;
                }
            }
        }
    }

    if ordering_violations > 0 {
        issues.push(Issue::warning(
            "tui_spatial_order",
            format!(
                "TUI spatial ordering has {} violations (nodes in wrong relative position)",
                ordering_violations
            ),
        ));
    }
}

/// Calculate a similarity score (0.0–1.0) between TUI output and layout graph.
///
/// Uses a multi-factor approach:
/// - Node presence: how many expected labels appear in the TUI output (via box
///   extraction or raw text substring matching)
/// - Edge presence: whether edges are rendered (arrows and/or braille)
pub fn calculate_tui_similarity(tui: &TuiStructure, graph: &LayoutGraph) -> f64 {
    let mut parts: Vec<f64> = Vec::new();

    // Node label presence: count how many graph labels appear in TUI output.
    // Uses both structured extraction (tui.labels) and raw substring matching
    // to handle diagram types where labels may not be inside standard │text│ boxes.
    let expected_nodes = graph.nodes.iter().filter(|n| !n.is_dummy).count();
    if expected_nodes > 0 {
        let mut found = 0;
        for node in &graph.nodes {
            if node.is_dummy {
                continue;
            }
            if is_symbol_node(node) {
                // Circle/DoubleCircle nodes render as symbols, always counted as found
                found += 1;
                continue;
            }
            let label = clean_label(node.label.as_deref().unwrap_or(&node.id));
            if tui.labels.contains(&label) || tui.raw_output.contains(&label) {
                found += 1;
            }
        }
        parts.push(found as f64 / expected_nodes as f64);
    }

    // Edge presence (binary: edges exist or not)
    let expected_edges = graph.edges.len();
    if expected_edges > 0 {
        let has_edges = tui.arrow_count > 0 || tui.has_braille;
        parts.push(if has_edges { 1.0 } else { 0.0 });
    }

    if parts.is_empty() {
        1.0
    } else {
        parts.iter().sum::<f64>() / parts.len() as f64
    }
}

// --- Pie chart-specific TUI evaluation ---

/// Check TUI pie chart output against the PieDb ground truth.
///
/// Since pie charts don't use LayoutGraph, we compare directly against PieDb:
/// - All section labels must appear in the output
/// - Percentage values should be present
/// - Title should appear if set
pub fn check_tui_pie_structure(output: &str, db: &crate::diagrams::pie::PieDb) -> Vec<Issue> {
    let mut issues = Vec::new();
    let sections = db.get_sections();
    let total: f64 = sections.iter().map(|(_, v)| *v).sum();

    // Check title
    if let Some(title) = db.get_diagram_title() {
        if !output.contains(title) {
            issues.push(Issue::error(
                "tui_pie_missing_title",
                format!("TUI pie output missing title: '{}'", title),
            ));
        }
    }

    if total <= 0.0 || sections.is_empty() {
        return issues;
    }

    // Check section labels
    for (label, _) in sections {
        if !output.contains(label.as_str()) {
            issues.push(Issue::error(
                "tui_pie_missing_label",
                format!("TUI pie output missing section label: '{}'", label),
            ));
        }
    }

    // Check percentages appear
    for (label, value) in sections {
        let pct = value / total * 100.0;
        let pct_str = format!("{:.1}%", pct);
        if !output.contains(&pct_str) {
            issues.push(Issue::warning(
                "tui_pie_missing_percentage",
                format!(
                    "TUI pie output missing percentage for '{}': expected {}",
                    label, pct_str
                ),
            ));
        }
    }

    // Check showData values appear
    if db.get_show_data() {
        for (label, value) in sections {
            let value_str = if value.fract() == 0.0 {
                format!("[{}]", *value as i64)
            } else {
                format!("[{}]", value)
            };
            if !output.contains(&value_str) {
                issues.push(Issue::warning(
                    "tui_pie_missing_data_value",
                    format!(
                        "TUI pie output missing data value for '{}': expected {}",
                        label, value_str
                    ),
                ));
            }
        }
    }

    // Check bar characters are present (at least one █ or ▌)
    let has_bars = output.contains('█') || output.contains('▌');
    if !has_bars {
        issues.push(Issue::error(
            "tui_pie_no_bars",
            "TUI pie output has no bar characters (█/▌)".to_string(),
        ));
    }

    issues
}

// ── Sequence diagram TUI checks ──────────────────────────────────────────────

/// Structure extracted from TUI sequence diagram output.
#[derive(Debug, Clone)]
pub struct TuiSequenceStructure {
    /// Participant labels found in box-drawing rectangles.
    pub participant_labels: Vec<String>,
    /// Message labels found (text not in participant boxes).
    pub message_labels: Vec<String>,
    /// Number of arrow characters found (> or <).
    pub arrow_count: usize,
    /// Number of lifeline characters found (│ not part of boxes).
    pub lifeline_count: usize,
    /// Fragment markers found (e.g., [loop], [alt], [end]).
    pub fragment_labels: Vec<String>,
    /// Canvas dimensions (rows, cols).
    pub dimensions: (usize, usize),
}

/// Parse TUI sequence diagram output into a structural representation.
pub fn parse_tui_sequence(output: &str) -> TuiSequenceStructure {
    let lines: Vec<&str> = output.lines().collect();
    let rows = lines.len();
    let cols = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);

    let grid: Vec<Vec<char>> = lines
        .iter()
        .map(|line| {
            let mut chars: Vec<char> = line.chars().collect();
            chars.resize(cols, ' ');
            chars
        })
        .collect();

    let participant_labels = extract_node_labels(&grid);
    let message_labels = extract_sequence_message_labels(&grid, &participant_labels);
    let arrow_count = count_sequence_arrows(&grid);
    let lifeline_count = count_lifelines(&grid);
    let fragment_labels = extract_fragment_labels(&grid);

    TuiSequenceStructure {
        participant_labels,
        message_labels,
        arrow_count,
        lifeline_count,
        fragment_labels,
        dimensions: (rows, cols),
    }
}

/// Count arrow characters (> and <) that indicate message direction.
fn count_sequence_arrows(grid: &[Vec<char>]) -> usize {
    grid.iter()
        .flat_map(|row| row.iter())
        .filter(|&&ch| ch == '>' || ch == '<')
        .count()
}

/// Count lifeline │ characters not adjacent to box corners.
fn count_lifelines(grid: &[Vec<char>]) -> usize {
    let mut count = 0;
    for (row_idx, row) in grid.iter().enumerate() {
        for (col_idx, &ch) in row.iter().enumerate() {
            if ch == '│' {
                // Check if this is a lifeline (not adjacent to box corners on same row)
                let left_is_border = col_idx > 0
                    && (BOX_HORIZONTAL.contains(&row[col_idx - 1])
                        || BOX_CORNERS.contains(&row[col_idx - 1]));
                let right_is_border = col_idx + 1 < row.len()
                    && (BOX_HORIZONTAL.contains(&row[col_idx + 1])
                        || BOX_CORNERS.contains(&row[col_idx + 1]));
                // Also check if it's a box side (has horizontal border above or below at same col)
                let is_box_side = is_inside_box(
                    grid,
                    row_idx,
                    col_idx.saturating_sub(1),
                    (col_idx + 2).min(row.len()),
                );

                if !left_is_border && !right_is_border && !is_box_side {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Extract message labels from sequence TUI output.
/// These are text runs that appear on rows with arrow characters or near them,
/// and are not participant box labels.
fn extract_sequence_message_labels(
    grid: &[Vec<char>],
    participant_labels: &[String],
) -> Vec<String> {
    let mut labels = Vec::new();
    let participant_set: std::collections::HashSet<&str> =
        participant_labels.iter().map(|s| s.as_str()).collect();

    for row in grid {
        // Find contiguous text runs
        let mut run_start = None;
        for (col_idx, &ch) in row.iter().enumerate() {
            let is_structure = BOX_HORIZONTAL.contains(&ch)
                || BOX_VERTICAL.contains(&ch)
                || BOX_CORNERS.contains(&ch)
                || ch == '>'
                || ch == '<'
                || ch == '─'
                || ch == '·'
                || ch == '┐'
                || ch == '┘';
            // Text is anything that's not a structural character and not a braille dot
            let is_text = !is_structure && !('\u{2800}'..='\u{28FF}').contains(&ch) && ch != '\0';

            if is_text {
                if run_start.is_none() {
                    run_start = Some(col_idx);
                }
            } else if let Some(start) = run_start {
                let text: String = row[start..col_idx].iter().collect();
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() && !participant_set.contains(trimmed.as_str()) {
                    labels.push(trimmed);
                }
                run_start = None;
            }
        }
        if let Some(start) = run_start {
            let text: String = row[start..].iter().collect();
            let trimmed = text.trim().to_string();
            if !trimmed.is_empty() && !participant_set.contains(trimmed.as_str()) {
                labels.push(trimmed);
            }
        }
    }

    // Deduplicate
    labels.sort();
    labels.dedup();
    labels
}

/// Extract fragment labels like [loop], [alt], [end] from the output.
fn extract_fragment_labels(grid: &[Vec<char>]) -> Vec<String> {
    let mut labels = Vec::new();
    for row in grid {
        let line: String = row.iter().collect();
        // Match patterns like [loop ...], [alt ...], [end]
        let mut i = 0;
        let chars: Vec<char> = line.chars().collect();
        while i < chars.len() {
            if chars[i] == '[' {
                let start = i;
                i += 1;
                while i < chars.len() && chars[i] != ']' {
                    i += 1;
                }
                if i < chars.len() {
                    let content: String = chars[start + 1..i].iter().collect();
                    let trimmed = content.trim().to_string();
                    if !trimmed.is_empty() {
                        labels.push(trimmed);
                    }
                }
            }
            i += 1;
        }
    }
    labels
}

/// Run TUI-specific structural checks for sequence diagrams.
pub fn check_tui_sequence_structure(
    tui: &TuiSequenceStructure,
    db: &crate::diagrams::sequence::SequenceDb,
) -> Vec<Issue> {
    let mut issues = Vec::new();

    check_tui_sequence_participants(tui, db, &mut issues);
    check_tui_sequence_messages(tui, db, &mut issues);
    check_tui_sequence_arrows(tui, db, &mut issues);
    check_tui_sequence_lifelines(tui, db, &mut issues);

    issues
}

/// Calculate a similarity score (0.0–1.0) for TUI pie chart output.
///
/// Factors:
/// - Section label presence (50%)
/// - Percentage value presence (30%)
/// - Title presence (10%)
/// - Bar character presence (10%)
pub fn calculate_tui_pie_similarity(output: &str, db: &crate::diagrams::pie::PieDb) -> f64 {
    let sections = db.get_sections();
    let total: f64 = sections.iter().map(|(_, v)| *v).sum();

    if sections.is_empty() || total <= 0.0 {
        // Empty chart — if output says "empty" or "no data", that's correct
        if output.contains("empty") || output.contains("no data") {
            return 1.0;
        }
        return 0.0;
    }

    let mut score = 0.0;

    // Label presence (50% weight)
    let label_count = sections
        .iter()
        .filter(|(label, _)| output.contains(label.as_str()))
        .count();
    score += 0.5 * (label_count as f64 / sections.len() as f64);

    // Percentage presence (30% weight)
    let pct_count = sections
        .iter()
        .filter(|(_, value)| {
            let pct = value / total * 100.0;
            let pct_str = format!("{:.1}%", pct);
            output.contains(&pct_str)
        })
        .count();
    score += 0.3 * (pct_count as f64 / sections.len() as f64);

    // Title presence (10% weight)
    if let Some(title) = db.get_diagram_title() {
        if output.contains(title) {
            score += 0.1;
        }
    } else {
        score += 0.1; // No title expected, full credit
    }

    // Bar characters (10% weight)
    if output.contains('█') || output.contains('▌') {
        score += 0.1;
    }

    score
}

/// Check that all participant labels appear in the TUI output.
fn check_tui_sequence_participants(
    tui: &TuiSequenceStructure,
    db: &crate::diagrams::sequence::SequenceDb,
    issues: &mut Vec<Issue>,
) {
    let actors = db.get_actors_in_order();
    let expected = actors.len();
    // Each participant appears twice (top + bottom), but dedup means unique count
    let actual = tui.participant_labels.len();

    if actual != expected {
        issues.push(
            Issue::error(
                "tui_seq_participant_count",
                format!(
                    "TUI participant count mismatch: expected {}, found {}",
                    expected, actual
                ),
            )
            .with_values(expected.to_string(), actual.to_string()),
        );
    }

    let tui_labels: std::collections::HashSet<&str> =
        tui.participant_labels.iter().map(|s| s.as_str()).collect();

    for actor in &actors {
        if !tui_labels.contains(actor.description.as_str()) {
            issues.push(Issue::error(
                "tui_seq_missing_participant",
                format!(
                    "TUI output missing participant label: '{}'",
                    actor.description
                ),
            ));
        }
    }
}

/// Check that message labels appear in the TUI output.
fn check_tui_sequence_messages(
    tui: &TuiSequenceStructure,
    db: &crate::diagrams::sequence::SequenceDb,
    issues: &mut Vec<Issue>,
) {
    let messages = db.get_messages();
    let tui_msg_set: std::collections::HashSet<&str> =
        tui.message_labels.iter().map(|s| s.as_str()).collect();

    for msg in messages {
        // Skip control structure messages
        if msg.from.is_none() || msg.to.is_none() {
            continue;
        }
        if msg.message.is_empty() {
            continue;
        }
        if !tui_msg_set.contains(msg.message.as_str()) {
            issues.push(Issue::warning(
                "tui_seq_missing_message",
                format!("TUI output missing message label: '{}'", msg.message),
            ));
        }
    }
}

/// Check that arrows are present for messages.
fn check_tui_sequence_arrows(
    tui: &TuiSequenceStructure,
    db: &crate::diagrams::sequence::SequenceDb,
    issues: &mut Vec<Issue>,
) {
    // Count actual messages (not control structures)
    let expected_messages = db
        .get_messages()
        .iter()
        .filter(|m| m.from.is_some() && m.to.is_some())
        .count();

    if expected_messages > 0 && tui.arrow_count == 0 {
        issues.push(Issue::error(
            "tui_seq_no_arrows",
            format!(
                "TUI output has no arrows (expected {} messages)",
                expected_messages
            ),
        ));
    }
}

/// Check that lifelines are present.
fn check_tui_sequence_lifelines(
    tui: &TuiSequenceStructure,
    db: &crate::diagrams::sequence::SequenceDb,
    issues: &mut Vec<Issue>,
) {
    let actors = db.get_actors_in_order();
    if !actors.is_empty() && tui.lifeline_count == 0 {
        issues.push(Issue::warning(
            "tui_seq_no_lifelines",
            "TUI output has no lifeline characters".to_string(),
        ));
    }
}

/// Calculate a similarity score (0.0–1.0) for sequence diagram TUI output.
pub fn calculate_tui_sequence_similarity(
    tui: &TuiSequenceStructure,
    db: &crate::diagrams::sequence::SequenceDb,
) -> f64 {
    let mut parts: Vec<f64> = Vec::new();

    // Participant match ratio
    let actors = db.get_actors_in_order();
    let expected_participants = actors.len();
    if expected_participants > 0 || !tui.participant_labels.is_empty() {
        let tui_set: std::collections::HashSet<&str> =
            tui.participant_labels.iter().map(|s| s.as_str()).collect();
        let db_set: std::collections::HashSet<&str> =
            actors.iter().map(|a| a.description.as_str()).collect();
        let common = tui_set.intersection(&db_set).count() as f64;
        let total = tui_set.len().max(db_set.len()) as f64;
        if total > 0.0 {
            parts.push(common / total);
        }
    }

    // Message label match ratio
    let messages = db.get_messages();
    let expected_msgs: Vec<&str> = messages
        .iter()
        .filter(|m| m.from.is_some() && m.to.is_some() && !m.message.is_empty())
        .map(|m| m.message.as_str())
        .collect();
    if !expected_msgs.is_empty() {
        let tui_msg_set: std::collections::HashSet<&str> =
            tui.message_labels.iter().map(|s| s.as_str()).collect();
        let found = expected_msgs
            .iter()
            .filter(|m| tui_msg_set.contains(*m))
            .count() as f64;
        parts.push(found / expected_msgs.len() as f64);
    }

    // Arrow presence
    let expected_message_count = messages
        .iter()
        .filter(|m| m.from.is_some() && m.to.is_some())
        .count();
    if expected_message_count > 0 {
        parts.push(if tui.arrow_count > 0 { 1.0 } else { 0.0 });
    }

    // Lifeline presence
    if !actors.is_empty() {
        parts.push(if tui.lifeline_count > 0 { 1.0 } else { 0.0 });
    }

    if parts.is_empty() {
        1.0
    } else {
        parts.iter().sum::<f64>() / parts.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a simple TUI output string with two boxed nodes
    fn simple_two_node_tui() -> String {
        [
            "┌───────┐",
            "│ Start │",
            "└───────┘",
            "    ⠁    ",
            "    ⠁    ",
            "    ▼    ",
            "┌───────┐",
            "│  End  │",
            "└───────┘",
        ]
        .join("\n")
    }

    fn single_node_tui() -> String {
        ["┌───────┐", "│ Hello │", "└───────┘"].join("\n")
    }

    fn diamond_tui() -> String {
        [
            "    /\\    ",
            "   /  \\   ",
            "  / Ok \\  ",
            "  \\    /  ",
            "   \\  /   ",
            "    \\/    ",
        ]
        .join("\n")
    }

    #[test]
    fn parse_single_node_finds_label() {
        let tui = parse_tui(&single_node_tui());
        assert_eq!(tui.labels, vec!["Hello"]);
    }

    #[test]
    fn parse_two_nodes_finds_labels() {
        let tui = parse_tui(&simple_two_node_tui());
        assert!(
            tui.labels.contains(&"Start".to_string()),
            "Should find Start label, got: {:?}",
            tui.labels
        );
        assert!(
            tui.labels.contains(&"End".to_string()),
            "Should find End label, got: {:?}",
            tui.labels
        );
    }

    #[test]
    fn parse_detects_arrows() {
        let tui = parse_tui(&simple_two_node_tui());
        assert_eq!(tui.arrow_count, 1, "Should find one arrow tip");
    }

    #[test]
    fn parse_detects_braille() {
        let tui = parse_tui(&simple_two_node_tui());
        assert!(tui.has_braille, "Should detect braille characters");
        assert!(tui.braille_count > 0);
    }

    #[test]
    fn parse_dimensions() {
        let tui = parse_tui(&single_node_tui());
        assert_eq!(tui.dimensions.0, 3, "Should have 3 rows");
        assert_eq!(tui.dimensions.1, 9, "Should have 9 cols");
    }

    #[test]
    fn parse_empty_output() {
        let tui = parse_tui("");
        assert!(tui.labels.is_empty());
        assert_eq!(tui.arrow_count, 0);
        assert!(!tui.has_braille);
        assert_eq!(tui.dimensions, (0, 0));
    }

    #[test]
    fn spatial_order_start_above_end() {
        let tui = parse_tui(&simple_two_node_tui());
        // Start should be at a lower row index than End
        let start_pos = tui
            .labels
            .iter()
            .zip(tui.label_positions.iter())
            .find(|(l, _)| *l == "Start")
            .map(|(_, p)| p);
        let end_pos = tui
            .labels
            .iter()
            .zip(tui.label_positions.iter())
            .find(|(l, _)| *l == "End")
            .map(|(_, p)| p);

        assert!(
            start_pos.unwrap().0 < end_pos.unwrap().0,
            "Start should be above End in TUI output"
        );
    }

    #[test]
    fn diamond_does_not_extract_labels() {
        // Diamond uses /\ characters not │, so no box labels extracted
        let tui = parse_tui(&diamond_tui());
        // Diamond text ("Ok") is between / and \, not │ — so it won't be extracted as a node label
        // This is expected behavior for now; diamond parsing can be enhanced later
        assert!(
            tui.labels.is_empty() || tui.labels.contains(&"Ok".to_string()),
            "Diamond label extraction is best-effort"
        );
    }

    #[test]
    fn edge_label_extraction() {
        let output = [
            "┌───────┐",
            "│ Start │",
            "└───────┘",
            "   Yes   ",
            "    ▼    ",
            "┌───────┐",
            "│  End  │",
            "└───────┘",
        ]
        .join("\n");
        let tui = parse_tui(&output);
        assert!(
            tui.edge_labels.contains(&"Yes".to_string()),
            "Should find edge label 'Yes', got: {:?}",
            tui.edge_labels
        );
    }

    fn make_two_node_graph() -> LayoutGraph {
        use crate::layout::{LayoutEdge, LayoutNode};

        let mut node_a = LayoutNode::new("A", 80.0, 32.0);
        node_a.label = Some("Start".to_string());
        node_a.x = Some(50.0);
        node_a.y = Some(10.0);

        let mut node_b = LayoutNode::new("B", 80.0, 32.0);
        node_b.label = Some("End".to_string());
        node_b.x = Some(50.0);
        node_b.y = Some(100.0);

        let edge = LayoutEdge::new("e1", "A", "B");

        let mut graph = LayoutGraph::new("test");
        graph.nodes.push(node_a);
        graph.nodes.push(node_b);
        graph.edges.push(edge);
        graph
    }

    #[test]
    fn similarity_perfect_match() {
        let graph = make_two_node_graph();
        let tui = parse_tui(&simple_two_node_tui());
        let sim = calculate_tui_similarity(&tui, &graph);
        assert!(
            sim > 0.5,
            "Similarity should be high for matching graph, got {}",
            sim
        );
    }

    #[test]
    fn check_tui_structure_no_issues_on_match() {
        let graph = make_two_node_graph();
        let tui = parse_tui(&simple_two_node_tui());
        let issues = check_tui_structure(&tui, &graph);
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.level == crate::eval::Level::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "Should have no errors for matching graph, got: {:?}",
            errors
        );
    }

    #[test]
    fn check_tui_structure_detects_missing_node() {
        use crate::layout::LayoutNode;

        let mut graph = make_two_node_graph();
        let mut node_c = LayoutNode::new("C", 80.0, 32.0);
        node_c.label = Some("Missing".to_string());
        graph.nodes.push(node_c);

        let tui = parse_tui(&simple_two_node_tui());
        let issues = check_tui_structure(&tui, &graph);
        let has_missing = issues.iter().any(|i| i.message.contains("Missing"));
        assert!(has_missing, "Should detect missing node 'Missing'");
    }

    // --- Integration tests: full parse → layout → TUI render → TUI eval pipeline ---

    fn parse_layout_render_tui(input: &str) -> (String, LayoutGraph) {
        use crate::layout::{CharacterSizeEstimator, ToLayoutGraph};
        use crate::render::tui::render_flowchart_tui;

        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Flowchart(db) => db,
            _ => panic!("Expected flowchart"),
        };
        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();
        let graph = crate::layout::layout(graph).unwrap();
        let tui_output = render_flowchart_tui(&db, &graph).unwrap();
        (tui_output, graph)
    }

    #[test]
    fn integration_single_node() {
        let (output, graph) = parse_layout_render_tui("flowchart TD\n    A[Hello]");
        let tui = parse_tui(&output);
        assert!(
            tui.labels.contains(&"Hello".to_string()),
            "Should find label 'Hello' in TUI output, got: {:?}\nOutput:\n{}",
            tui.labels,
            output
        );
        let issues = check_tui_structure(&tui, &graph);
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.level == crate::eval::Level::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "Single node should have no errors, got: {:?}",
            errors
        );
    }

    #[test]
    fn integration_two_nodes_with_edge() {
        let (output, graph) = parse_layout_render_tui("flowchart TD\n    A[Start] --> B[End]");
        let tui = parse_tui(&output);

        assert!(
            tui.labels.contains(&"Start".to_string()),
            "Should find 'Start', got: {:?}\nOutput:\n{}",
            tui.labels,
            output
        );
        assert!(
            tui.labels.contains(&"End".to_string()),
            "Should find 'End', got: {:?}",
            tui.labels
        );
        assert!(
            tui.arrow_count > 0 || tui.has_braille,
            "Should have edges rendered"
        );

        let sim = calculate_tui_similarity(&tui, &graph);
        assert!(sim > 0.5, "Similarity should be reasonable, got {}", sim);
    }

    #[test]
    fn integration_three_nodes_chain() {
        let (output, graph) =
            parse_layout_render_tui("flowchart TD\n    A[First] --> B[Second] --> C[Third]");
        let tui = parse_tui(&output);

        assert!(
            tui.labels.len() >= 3,
            "Should find 3 labels, got: {:?}",
            tui.labels
        );
        assert!(
            tui.arrow_count >= 2,
            "Should have at least 2 arrows for 2 edges, got {}",
            tui.arrow_count
        );
    }

    #[test]
    fn integration_edge_labels() {
        let (output, _graph) =
            parse_layout_render_tui("flowchart TD\n    A[Start] -->|Yes| B[End]");
        let _tui = parse_tui(&output);

        // The edge label "Yes" should appear somewhere in the output
        assert!(
            output.contains("Yes"),
            "TUI output should contain edge label 'Yes'\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn integration_spatial_ordering_preserved() {
        let (output, graph) = parse_layout_render_tui("flowchart TD\n    A[Top] --> B[Bottom]");
        let tui = parse_tui(&output);
        let issues = check_tui_structure(&tui, &graph);

        let ordering_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.check == "tui_spatial_order")
            .collect();
        assert!(
            ordering_issues.is_empty(),
            "TD flow should preserve top→bottom ordering, got: {:?}",
            ordering_issues
        );
    }
}
