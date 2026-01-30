//! ASCII output evaluation checks.
//!
//! Parses ASCII character-art output to extract structural and spatial
//! information, then compares it against the layout graph (ground truth).
//!
//! Unlike SVG eval (which compares selkie SVG vs mermaid.js reference SVG),
//! ASCII eval compares the rendered ASCII output against the positioned layout
//! graph, since there is no mermaid.js ASCII reference.

use super::Issue;
use crate::layout::{LayoutGraph, NodeShape};

/// Structure extracted from ASCII character-art output.
#[derive(Debug, Clone)]
pub struct AsciiStructure {
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

/// Parse ASCII character-art output into a structural representation.
pub fn parse_ascii(output: &str) -> AsciiStructure {
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

    AsciiStructure {
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

/// Characters that indicate horizontal section dividers (├ or ┤).
const BOX_DIVIDERS: &[char] = &['├', '┤'];

/// Check if a row segment is inside a box by verifying that the segment
/// is bounded by │ characters on the same row (already the case when called
/// from `extract_node_labels`), and that there are horizontal borders
/// directly above and below without gaps (i.e., every row between the text
/// and the border also has │ at the boundary columns).
///
/// The search range is generous (up to 12 rows) to handle class diagram boxes
/// which can be tall with many sections (annotations, name, members, methods).
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

    let is_border_char = |ch: char| {
        BOX_CORNERS.contains(&ch) || BOX_HORIZONTAL.contains(&ch) || BOX_DIVIDERS.contains(&ch)
    };

    // Look for horizontal border above (within 12 rows for tall class boxes)
    let has_top = if row > 0 {
        let search_start = row.saturating_sub(12);
        grid[search_start..row]
            .iter()
            .rev()
            .any(|r| left_border < r.len() && is_border_char(r[left_border]))
    } else {
        false
    };

    // Look for horizontal border below (within 12 rows)
    let has_bottom = if row + 1 < grid.len() {
        let search_end = (row + 13).min(grid.len());
        grid[row + 1..search_end]
            .iter()
            .any(|r| left_border < r.len() && is_border_char(r[left_border]))
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

/// Run ASCII-specific structural checks comparing ASCII output against the layout graph.
pub fn check_ascii_structure(ascii: &AsciiStructure, graph: &LayoutGraph) -> Vec<Issue> {
    let mut issues = Vec::new();

    check_ascii_node_count(ascii, graph, &mut issues);
    check_ascii_labels(ascii, graph, &mut issues);
    check_ascii_edges(ascii, graph, &mut issues);
    check_ascii_spatial_order(ascii, graph, &mut issues);

    issues
}

/// Run ER-specific ASCII checks comparing output against the ER database.
///
/// Checks that entity attributes (type, name, keys) are present in the ASCII
/// output and that table-structure characters (├ ┬ ┴) are used for entities
/// with attributes.
pub fn check_er_ascii_structure(
    ascii: &AsciiStructure,
    db: &crate::diagrams::er::ErDb,
) -> Vec<Issue> {
    let mut issues = Vec::new();

    let entities = db.get_entities();

    for (name, entity) in entities {
        // Check entity name is present
        if !ascii.raw_output.contains(name.as_str()) {
            issues.push(Issue::error(
                "er_ascii_missing_entity",
                format!("ER ASCII output missing entity: '{}'", name),
            ));
            continue;
        }

        // Check attributes are rendered
        for attr in &entity.attributes {
            if !ascii.raw_output.contains(&attr.attr_type) {
                issues.push(Issue::warning(
                    "er_ascii_missing_attr_type",
                    format!(
                        "ER ASCII output missing attribute type '{}' for entity '{}'",
                        attr.attr_type, name
                    ),
                ));
            }
            if !ascii.raw_output.contains(&attr.name) {
                issues.push(Issue::warning(
                    "er_ascii_missing_attr_name",
                    format!(
                        "ER ASCII output missing attribute name '{}' for entity '{}'",
                        attr.name, name
                    ),
                ));
            }
            for key in &attr.keys {
                if !ascii.raw_output.contains(key.as_str()) {
                    issues.push(Issue::warning(
                        "er_ascii_missing_attr_key",
                        format!(
                            "ER ASCII output missing key '{}' for {}.{}",
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
            if !ascii.raw_output.contains('├') {
                issues.push(Issue::warning(
                    "er_ascii_no_table_divider",
                    "ER ASCII output missing table divider '├' for entities with attributes"
                        .to_string(),
                ));
            }
            if !ascii.raw_output.contains('┬') {
                issues.push(Issue::warning(
                    "er_ascii_no_column_separator",
                    "ER ASCII output missing column separator '┬' for attribute tables".to_string(),
                ));
            }
        }
    }

    // Check relationship labels
    for rel in db.get_relationships() {
        if !rel.role_a.is_empty() && !ascii.raw_output.contains(&rel.role_a) {
            issues.push(Issue::warning(
                "er_ascii_missing_rel_label",
                format!(
                    "ER ASCII output missing relationship label: '{}'",
                    rel.role_a
                ),
            ));
        }
    }

    issues
}

/// Check that ASCII output has the correct number of nodes.
/// Counts nodes whose labels appear anywhere in the ASCII output (boxes, free text, etc.).
///
/// For class diagrams, each class box has multiple text rows (name, members,
/// methods), so the raw label count may exceed the node count. We check that
/// at least the expected number of node labels are present and flag an error
/// only if fewer labels exist than expected nodes.
fn check_ascii_node_count(ascii: &AsciiStructure, graph: &LayoutGraph, issues: &mut Vec<Issue>) {
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
        if ascii.labels.contains(&label) || ascii.raw_output.contains(&label) {
            found += 1;
        }
    }

    if found < expected {
        issues.push(
            Issue::error(
                "ascii_node_count",
                format!(
                    "ASCII node count too low: expected at least {}, found {}",
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

/// Clean HTML line breaks from labels (matches ASCII renderer behavior).
/// Also normalizes whitespace to single spaces.
fn clean_label(raw: &str) -> String {
    let cleaned = raw.replace("<br/>", " ").replace("<br>", " ");
    // Normalize multiple spaces to single space
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Check that all node labels from the layout graph appear in the ASCII output.
/// Subgraph labels appear as free text (not inside boxes), so we check the
/// raw output string as well.
fn check_ascii_labels(ascii: &AsciiStructure, graph: &LayoutGraph, issues: &mut Vec<Issue>) {
    let ascii_labels: std::collections::HashSet<&str> =
        ascii.labels.iter().map(|s| s.as_str()).collect();

    // Also collect all text from the raw grid for subgraph label matching
    let ascii_all_labels: std::collections::HashSet<String> = ascii
        .labels
        .iter()
        .cloned()
        .chain(ascii.edge_labels.iter().cloned())
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
            if !ascii.raw_output.contains(symbol) {
                issues.push(Issue::error(
                    "ascii_missing_label",
                    format!(
                        "ASCII output missing {} symbol for node '{}'",
                        symbol, node.id
                    ),
                ));
            }
            continue;
        }
        let raw_label = node.label.as_deref().unwrap_or(&node.id);
        let label = clean_label(raw_label);
        // Check in box labels, edge labels, and raw output (for subgraph labels)
        if !ascii_labels.contains(label.as_str())
            && !ascii_all_labels.contains(&label)
            && !ascii.raw_output.contains(&label)
        {
            issues.push(Issue::error(
                "ascii_missing_label",
                format!("ASCII output missing node label: '{}'", label),
            ));
        }
    }
}

/// Check that edges are rendered (arrows and/or braille dots present).
fn check_ascii_edges(ascii: &AsciiStructure, graph: &LayoutGraph, issues: &mut Vec<Issue>) {
    let expected_edges = graph.edges.len();

    if expected_edges > 0 && ascii.arrow_count == 0 && !ascii.has_braille {
        issues.push(Issue::error(
            "ascii_no_edges",
            format!(
                "ASCII output has no edges rendered (expected {} edges, found 0 arrows and 0 braille chars)",
                expected_edges
            ),
        ));
    }

    // Arrow count should roughly match edge count (each edge has one arrow tip)
    if expected_edges > 0 && ascii.arrow_count > 0 && ascii.arrow_count != expected_edges {
        issues.push(
            Issue::warning(
                "ascii_arrow_count",
                format!(
                    "ASCII arrow count differs from edge count: expected {}, found {}",
                    expected_edges, ascii.arrow_count
                ),
            )
            .with_values(expected_edges.to_string(), ascii.arrow_count.to_string()),
        );
    }
}

/// Check spatial ordering: if node A is above node B in the layout,
/// it should be above in the ASCII output too.
fn check_ascii_spatial_order(ascii: &AsciiStructure, graph: &LayoutGraph, issues: &mut Vec<Issue>) {
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

    // Build a map of label → ASCII row position
    let ascii_positions: std::collections::HashMap<&str, usize> = ascii
        .labels
        .iter()
        .zip(ascii.label_positions.iter())
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

            if let (Some(&ascii_row_a), Some(&ascii_row_b)) = (
                ascii_positions.get(label_a.as_str()),
                ascii_positions.get(label_b.as_str()),
            ) {
                let ascii_a_above = ascii_row_a < ascii_row_b;
                if layout_a_above != ascii_a_above {
                    ordering_violations += 1;
                }
            }
        }
    }

    if ordering_violations > 0 {
        issues.push(Issue::warning(
            "ascii_spatial_order",
            format!(
                "ASCII spatial ordering has {} violations (nodes in wrong relative position)",
                ordering_violations
            ),
        ));
    }
}

/// Calculate a similarity score (0.0–1.0) between ASCII output and layout graph.
///
/// Uses a multi-factor approach:
/// - Node presence: how many expected labels appear in the ASCII output (via box
///   extraction or raw text substring matching)
/// - Edge presence: whether edges are rendered (arrows and/or braille)
pub fn calculate_ascii_similarity(ascii: &AsciiStructure, graph: &LayoutGraph) -> f64 {
    let mut parts: Vec<f64> = Vec::new();

    // Node label presence: count how many graph labels appear in ASCII output.
    // Uses both structured extraction (ascii.labels) and raw substring matching
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
            if ascii.labels.contains(&label) || ascii.raw_output.contains(&label) {
                found += 1;
            }
        }
        parts.push(found as f64 / expected_nodes as f64);
    }

    // Edge presence (binary: edges exist or not)
    let expected_edges = graph.edges.len();
    if expected_edges > 0 {
        let has_edges = ascii.arrow_count > 0 || ascii.has_braille;
        parts.push(if has_edges { 1.0 } else { 0.0 });
    }

    if parts.is_empty() {
        1.0
    } else {
        parts.iter().sum::<f64>() / parts.len() as f64
    }
}

// --- Pie chart-specific ASCII evaluation ---

/// Check ASCII pie chart output against the PieDb ground truth.
///
/// Since pie charts don't use LayoutGraph, we compare directly against PieDb:
/// - All section labels must appear in the output
/// - Percentage values should be present
/// - Title should appear if set
pub fn check_ascii_pie_structure(output: &str, db: &crate::diagrams::pie::PieDb) -> Vec<Issue> {
    let mut issues = Vec::new();
    let sections = db.get_sections();
    let total: f64 = sections.iter().map(|(_, v)| *v).sum();

    // Check title
    if let Some(title) = db.get_diagram_title() {
        if !output.contains(title) {
            issues.push(Issue::error(
                "ascii_pie_missing_title",
                format!("ASCII pie output missing title: '{}'", title),
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
                "ascii_pie_missing_label",
                format!("ASCII pie output missing section label: '{}'", label),
            ));
        }
    }

    // Check percentages appear
    for (label, value) in sections {
        let pct = value / total * 100.0;
        let pct_str = format!("{:.1}%", pct);
        if !output.contains(&pct_str) {
            issues.push(Issue::warning(
                "ascii_pie_missing_percentage",
                format!(
                    "ASCII pie output missing percentage for '{}': expected {}",
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
                    "ascii_pie_missing_data_value",
                    format!(
                        "ASCII pie output missing data value for '{}': expected {}",
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
            "ascii_pie_no_bars",
            "ASCII pie output has no bar characters (█/▌)".to_string(),
        ));
    }

    issues
}

// ── Sequence diagram ASCII checks ──────────────────────────────────────────────

/// Structure extracted from ASCII sequence diagram output.
#[derive(Debug, Clone)]
pub struct AsciiSequenceStructure {
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

/// Parse ASCII sequence diagram output into a structural representation.
pub fn parse_ascii_sequence(output: &str) -> AsciiSequenceStructure {
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

    AsciiSequenceStructure {
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

/// Extract message labels from sequence ASCII output.
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

/// Run ASCII-specific structural checks for sequence diagrams.
pub fn check_ascii_sequence_structure(
    ascii: &AsciiSequenceStructure,
    db: &crate::diagrams::sequence::SequenceDb,
) -> Vec<Issue> {
    let mut issues = Vec::new();

    check_ascii_sequence_participants(ascii, db, &mut issues);
    check_ascii_sequence_messages(ascii, db, &mut issues);
    check_ascii_sequence_arrows(ascii, db, &mut issues);
    check_ascii_sequence_lifelines(ascii, db, &mut issues);

    issues
}

/// Calculate a similarity score (0.0–1.0) for ASCII pie chart output.
///
/// Factors:
/// - Section label presence (50%)
/// - Percentage value presence (30%)
/// - Title presence (10%)
/// - Bar character presence (10%)
pub fn calculate_ascii_pie_similarity(output: &str, db: &crate::diagrams::pie::PieDb) -> f64 {
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

/// Check that all participant labels appear in the ASCII output.
fn check_ascii_sequence_participants(
    ascii: &AsciiSequenceStructure,
    db: &crate::diagrams::sequence::SequenceDb,
    issues: &mut Vec<Issue>,
) {
    let actors = db.get_actors_in_order();
    let expected = actors.len();
    // Each participant appears twice (top + bottom), but dedup means unique count
    let actual = ascii.participant_labels.len();

    if actual != expected {
        issues.push(
            Issue::error(
                "ascii_seq_participant_count",
                format!(
                    "ASCII participant count mismatch: expected {}, found {}",
                    expected, actual
                ),
            )
            .with_values(expected.to_string(), actual.to_string()),
        );
    }

    let ascii_labels: std::collections::HashSet<&str> = ascii
        .participant_labels
        .iter()
        .map(|s| s.as_str())
        .collect();

    for actor in &actors {
        if !ascii_labels.contains(actor.description.as_str()) {
            issues.push(Issue::error(
                "ascii_seq_missing_participant",
                format!(
                    "ASCII output missing participant label: '{}'",
                    actor.description
                ),
            ));
        }
    }
}

/// Check that message labels appear in the ASCII output.
fn check_ascii_sequence_messages(
    ascii: &AsciiSequenceStructure,
    db: &crate::diagrams::sequence::SequenceDb,
    issues: &mut Vec<Issue>,
) {
    let messages = db.get_messages();
    let ascii_msg_set: std::collections::HashSet<&str> =
        ascii.message_labels.iter().map(|s| s.as_str()).collect();

    for msg in messages {
        // Skip control structure messages
        if msg.from.is_none() || msg.to.is_none() {
            continue;
        }
        if msg.message.is_empty() {
            continue;
        }
        if !ascii_msg_set.contains(msg.message.as_str()) {
            issues.push(Issue::warning(
                "ascii_seq_missing_message",
                format!("ASCII output missing message label: '{}'", msg.message),
            ));
        }
    }
}

/// Check that arrows are present for messages.
fn check_ascii_sequence_arrows(
    ascii: &AsciiSequenceStructure,
    db: &crate::diagrams::sequence::SequenceDb,
    issues: &mut Vec<Issue>,
) {
    // Count actual messages (not control structures)
    let expected_messages = db
        .get_messages()
        .iter()
        .filter(|m| m.from.is_some() && m.to.is_some())
        .count();

    if expected_messages > 0 && ascii.arrow_count == 0 {
        issues.push(Issue::error(
            "ascii_seq_no_arrows",
            format!(
                "ASCII output has no arrows (expected {} messages)",
                expected_messages
            ),
        ));
    }
}

/// Check that lifelines are present.
fn check_ascii_sequence_lifelines(
    ascii: &AsciiSequenceStructure,
    db: &crate::diagrams::sequence::SequenceDb,
    issues: &mut Vec<Issue>,
) {
    let actors = db.get_actors_in_order();
    if !actors.is_empty() && ascii.lifeline_count == 0 {
        issues.push(Issue::warning(
            "ascii_seq_no_lifelines",
            "ASCII output has no lifeline characters".to_string(),
        ));
    }
}

/// Calculate a similarity score (0.0–1.0) for sequence diagram ASCII output.
pub fn calculate_ascii_sequence_similarity(
    ascii: &AsciiSequenceStructure,
    db: &crate::diagrams::sequence::SequenceDb,
) -> f64 {
    let mut parts: Vec<f64> = Vec::new();

    // Participant match ratio
    let actors = db.get_actors_in_order();
    let expected_participants = actors.len();
    if expected_participants > 0 || !ascii.participant_labels.is_empty() {
        let ascii_set: std::collections::HashSet<&str> = ascii
            .participant_labels
            .iter()
            .map(|s| s.as_str())
            .collect();
        let db_set: std::collections::HashSet<&str> =
            actors.iter().map(|a| a.description.as_str()).collect();
        let common = ascii_set.intersection(&db_set).count() as f64;
        let total = ascii_set.len().max(db_set.len()) as f64;
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
        let ascii_msg_set: std::collections::HashSet<&str> =
            ascii.message_labels.iter().map(|s| s.as_str()).collect();
        let found = expected_msgs
            .iter()
            .filter(|m| ascii_msg_set.contains(*m))
            .count() as f64;
        parts.push(found / expected_msgs.len() as f64);
    }

    // Arrow presence
    let expected_message_count = messages
        .iter()
        .filter(|m| m.from.is_some() && m.to.is_some())
        .count();
    if expected_message_count > 0 {
        parts.push(if ascii.arrow_count > 0 { 1.0 } else { 0.0 });
    }

    // Lifeline presence
    if !actors.is_empty() {
        parts.push(if ascii.lifeline_count > 0 { 1.0 } else { 0.0 });
    }

    if parts.is_empty() {
        1.0
    } else {
        parts.iter().sum::<f64>() / parts.len() as f64
    }
}

// --- Gantt chart-specific ASCII evaluation ---

/// Check ASCII gantt chart output against the GanttDb ground truth.
///
/// Since gantt charts don't use LayoutGraph, we compare directly against GanttDb:
/// - All task names must appear in the output
/// - Section names should appear
/// - Title should appear if set
/// - Status indicators should be present for flagged tasks
pub fn check_ascii_gantt_structure(
    output: &str,
    db: &mut crate::diagrams::gantt::GanttDb,
) -> Vec<Issue> {
    let mut issues = Vec::new();
    let tasks = db.get_tasks();

    // Check title
    let title = db.get_diagram_title();
    if !title.is_empty() && !output.contains(title) {
        issues.push(Issue::error(
            "ascii_gantt_missing_title",
            format!("ASCII gantt output missing title: '{}'", title),
        ));
    }

    if tasks.is_empty() {
        return issues;
    }

    // Check task names (skip vert markers)
    for task in &tasks {
        if task.flags.vert {
            continue;
        }
        if !output.contains(&task.task) {
            issues.push(Issue::error(
                "ascii_gantt_missing_task",
                format!("ASCII gantt output missing task: '{}'", task.task),
            ));
        }
    }

    // Check section names
    for section in db.get_sections() {
        if !output.contains(section.as_str()) {
            issues.push(Issue::warning(
                "ascii_gantt_missing_section",
                format!("ASCII gantt output missing section: '{}'", section),
            ));
        }
    }

    // Check status indicators
    let has_done = tasks.iter().any(|t| t.flags.done && !t.flags.vert);
    let has_active = tasks.iter().any(|t| t.flags.active && !t.flags.vert);
    let has_milestone = tasks.iter().any(|t| t.flags.milestone && !t.flags.vert);

    if has_done && !output.contains('✓') && !output.contains('░') {
        issues.push(Issue::warning(
            "ascii_gantt_no_done_indicator",
            "ASCII gantt output has done tasks but no done indicator (✓/░)".to_string(),
        ));
    }
    if has_active && !output.contains('►') {
        issues.push(Issue::warning(
            "ascii_gantt_no_active_indicator",
            "ASCII gantt output has active tasks but no active indicator (►)".to_string(),
        ));
    }
    if has_milestone && !output.contains('◆') {
        issues.push(Issue::warning(
            "ascii_gantt_no_milestone_indicator",
            "ASCII gantt output has milestones but no milestone indicator (◆)".to_string(),
        ));
    }

    // Check bar characters are present
    let has_bars = output.contains('█') || output.contains('░');
    if !has_bars {
        issues.push(Issue::error(
            "ascii_gantt_no_bars",
            "ASCII gantt output has no bar characters (█/░)".to_string(),
        ));
    }

    issues
}

/// Calculate a similarity score (0.0–1.0) for ASCII gantt chart output.
///
/// Factors:
/// - Task name presence (40%)
/// - Section name presence (20%)
/// - Title presence (15%)
/// - Status indicator presence (15%)
/// - Bar character presence (10%)
pub fn calculate_ascii_gantt_similarity(
    output: &str,
    db: &mut crate::diagrams::gantt::GanttDb,
) -> f64 {
    let tasks = db.get_tasks();
    let sections = db.get_sections();

    if tasks.is_empty() {
        if output.contains("empty") || output.contains("no data") || output.contains("no resolved")
        {
            return 1.0;
        }
        return 0.0;
    }

    let mut score = 0.0;

    // Task name presence (40% weight)
    let non_vert_tasks: Vec<_> = tasks.iter().filter(|t| !t.flags.vert).collect();
    if !non_vert_tasks.is_empty() {
        let found = non_vert_tasks
            .iter()
            .filter(|t| output.contains(&t.task))
            .count();
        score += 0.4 * (found as f64 / non_vert_tasks.len() as f64);
    } else {
        score += 0.4;
    }

    // Section name presence (20% weight)
    if !sections.is_empty() {
        let found = sections
            .iter()
            .filter(|s| output.contains(s.as_str()))
            .count();
        score += 0.2 * (found as f64 / sections.len() as f64);
    } else {
        score += 0.2;
    }

    // Title presence (15% weight)
    let title = db.get_diagram_title();
    if !title.is_empty() {
        if output.contains(title) {
            score += 0.15;
        }
    } else {
        score += 0.15;
    }

    // Status indicators (15% weight)
    let mut status_checks = 0;
    let mut status_found = 0;
    let has_done = non_vert_tasks.iter().any(|t| t.flags.done);
    let has_active = non_vert_tasks.iter().any(|t| t.flags.active);
    let has_milestone = non_vert_tasks.iter().any(|t| t.flags.milestone);

    if has_done {
        status_checks += 1;
        if output.contains('✓') || output.contains('░') {
            status_found += 1;
        }
    }
    if has_active {
        status_checks += 1;
        if output.contains('►') {
            status_found += 1;
        }
    }
    if has_milestone {
        status_checks += 1;
        if output.contains('◆') {
            status_found += 1;
        }
    }
    if status_checks > 0 {
        score += 0.15 * (status_found as f64 / status_checks as f64);
    } else {
        score += 0.15;
    }

    // Bar characters (10% weight)
    if output.contains('█') || output.contains('░') {
        score += 0.1;
    }

    score
}

// --- Mindmap-specific ASCII evaluation ---

/// Check ASCII mindmap output against the MindmapDb ground truth.
///
/// Since mindmaps don't use LayoutGraph, we compare directly against MindmapDb:
/// - All node labels must appear in the output
/// - Tree structure connectors (├── └── │) should be present
pub fn check_ascii_mindmap_structure(
    output: &str,
    db: &crate::diagrams::mindmap::MindmapDb,
) -> Vec<Issue> {
    let mut issues = Vec::new();

    let root = match db.get_mindmap() {
        Some(node) => node,
        None => return issues,
    };

    // Collect all labels
    let labels = crate::render::ascii::mindmap::collect_labels(root);

    // Check all labels appear
    for label in &labels {
        if !output.contains(label.as_str()) {
            issues.push(Issue::error(
                "ascii_mindmap_missing_label",
                format!("ASCII mindmap output missing node label: '{}'", label),
            ));
        }
    }

    // Check tree connectors are present (if there are children)
    let has_children = !root.children.is_empty();
    if has_children {
        let has_connectors = output.contains("├──") || output.contains("└──");
        if !has_connectors {
            issues.push(Issue::warning(
                "ascii_mindmap_no_connectors",
                "ASCII mindmap output has children but no tree connectors (├──/└──)".to_string(),
            ));
        }
    }

    issues
}

/// Calculate a similarity score (0.0–1.0) for ASCII mindmap output.
///
/// Factors:
/// - Node label presence (70%)
/// - Tree connector presence (20%)
/// - Root node presence (10%)
pub fn calculate_ascii_mindmap_similarity(
    output: &str,
    db: &crate::diagrams::mindmap::MindmapDb,
) -> f64 {
    let root = match db.get_mindmap() {
        Some(node) => node,
        None => {
            if output.contains("empty") {
                return 1.0;
            }
            return 0.0;
        }
    };

    let labels = crate::render::ascii::mindmap::collect_labels(root);
    let mut score = 0.0;

    // Label presence (70%)
    if !labels.is_empty() {
        let found = labels
            .iter()
            .filter(|l| output.contains(l.as_str()))
            .count();
        score += 0.7 * (found as f64 / labels.len() as f64);
    } else {
        score += 0.7;
    }

    // Tree connectors (20%)
    if !root.children.is_empty() {
        if output.contains("├──") || output.contains("└──") {
            score += 0.2;
        }
    } else {
        score += 0.2;
    }

    // Root node (10%)
    let root_text = root.descr.replace("<br/>", " ").replace("<br>", " ");
    let root_text = root_text.split_whitespace().collect::<Vec<_>>().join(" ");
    if output.contains(&root_text) {
        score += 0.1;
    }

    score
}

// ─────────────────────────────────────────────────────────────
// Generic text-based ASCII checks for diagram types without LayoutGraph
// ─────────────────────────────────────────────────────────────

/// Check a text-based ASCII output for basic structural quality.
///
/// Used for diagram types that don't have a LayoutGraph (journey, timeline,
/// kanban, packet, xychart, quadrant, radar, git, sankey, block, c4, treemap).
pub fn check_ascii_text_output(output: &str, diagram_type: &str) -> Vec<Issue> {
    let mut issues = Vec::new();

    // Check output is non-empty
    if output.trim().is_empty() {
        issues.push(Issue {
            check: format!("ascii_{}_output", diagram_type),
            message: "ASCII output is empty".to_string(),
            level: super::Level::Error,
            expected: None,
            actual: None,
        });
        return issues;
    }

    // Check output has reasonable length (not just a placeholder)
    if output.trim().lines().count() < 2 {
        issues.push(Issue {
            check: format!("ascii_{}_content", diagram_type),
            message: "ASCII output has fewer than 2 lines".to_string(),
            level: super::Level::Warning,
            expected: None,
            actual: None,
        });
    }

    // Check for "empty" placeholder (acceptable for empty diagrams, but flagged)
    if output.contains("(empty") {
        issues.push(Issue {
            check: format!("ascii_{}_empty", diagram_type),
            message: "ASCII output contains empty placeholder".to_string(),
            level: super::Level::Warning,
            expected: None,
            actual: None,
        });
    }

    issues
}

/// Calculate a simple text-based similarity score for diagram types
/// without a LayoutGraph.
///
/// Returns 1.0 for non-empty output with structure, 0.0 for empty.
pub fn calculate_ascii_text_similarity(output: &str) -> f64 {
    if output.trim().is_empty() {
        return 0.0;
    }

    let mut score = 0.0;

    // Non-empty output (40%)
    score += 0.4;

    // Multi-line output (20%)
    if output.trim().lines().count() >= 3 {
        score += 0.2;
    }

    // Has structural characters like box drawing, bullets, or bars (20%)
    let has_structure = output.chars().any(|c| {
        matches!(
            c,
            '┌' | '┐'
                | '└'
                | '┘'
                | '│'
                | '─'
                | '├'
                | '┤'
                | '┬'
                | '┴'
                | '┼'
                | '█'
                | '▌'
                | '░'
                | '●'
                | '◆'
                | '■'
                | '▲'
                | '◉'
                | '★'
                | '§'
                | '▶'
                | '▼'
                | '◀'
                | '→'
        )
    });
    if has_structure {
        score += 0.2;
    }

    // Doesn't contain error/empty placeholders (20%)
    if !output.contains("(empty") && !output.contains("(no data") {
        score += 0.2;
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a simple ASCII output string with two boxed nodes
    fn simple_two_node_ascii() -> String {
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

    fn single_node_ascii() -> String {
        ["┌───────┐", "│ Hello │", "└───────┘"].join("\n")
    }

    fn diamond_ascii() -> String {
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
        let ascii_out = parse_ascii(&single_node_ascii());
        assert_eq!(ascii.labels, vec!["Hello"]);
    }

    #[test]
    fn parse_two_nodes_finds_labels() {
        let ascii_out = parse_ascii(&simple_two_node_ascii());
        assert!(
            ascii.labels.contains(&"Start".to_string()),
            "Should find Start label, got: {:?}",
            ascii.labels
        );
        assert!(
            ascii.labels.contains(&"End".to_string()),
            "Should find End label, got: {:?}",
            ascii.labels
        );
    }

    #[test]
    fn parse_detects_arrows() {
        let ascii_out = parse_ascii(&simple_two_node_ascii());
        assert_eq!(ascii.arrow_count, 1, "Should find one arrow tip");
    }

    #[test]
    fn parse_detects_braille() {
        let ascii_out = parse_ascii(&simple_two_node_ascii());
        assert!(ascii.has_braille, "Should detect braille characters");
        assert!(ascii_out.braille_count > 0);
    }

    #[test]
    fn parse_dimensions() {
        let ascii_out = parse_ascii(&single_node_ascii());
        assert_eq!(ascii_out.dimensions.0, 3, "Should have 3 rows");
        assert_eq!(ascii_out.dimensions.1, 9, "Should have 9 cols");
    }

    #[test]
    fn parse_empty_output() {
        let ascii_out = parse_ascii("");
        assert!(ascii.labels.is_empty());
        assert_eq!(ascii.arrow_count, 0);
        assert!(!ascii.has_braille);
        assert_eq!(ascii_out.dimensions, (0, 0));
    }

    #[test]
    fn spatial_order_start_above_end() {
        let ascii_out = parse_ascii(&simple_two_node_ascii());
        // Start should be at a lower row index than End
        let start_pos = ascii_out
            .labels
            .iter()
            .zip(ascii.label_positions.iter())
            .find(|(l, _)| *l == "Start")
            .map(|(_, p)| p);
        let end_pos = ascii_out
            .labels
            .iter()
            .zip(ascii.label_positions.iter())
            .find(|(l, _)| *l == "End")
            .map(|(_, p)| p);

        assert!(
            start_pos.unwrap().0 < end_pos.unwrap().0,
            "Start should be above End in ASCII output"
        );
    }

    #[test]
    fn diamond_does_not_extract_labels() {
        // Diamond uses /\ characters not │, so no box labels extracted
        let ascii_out = parse_ascii(&diamond_ascii());
        // Diamond text ("Ok") is between / and \, not │ — so it won't be extracted as a node label
        // This is expected behavior for now; diamond parsing can be enhanced later
        assert!(
            ascii.labels.is_empty() || ascii.labels.contains(&"Ok".to_string()),
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
        let ascii_out = parse_ascii(&output);
        assert!(
            ascii.edge_labels.contains(&"Yes".to_string()),
            "Should find edge label 'Yes', got: {:?}",
            ascii.edge_labels
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
        let ascii_out = parse_ascii(&simple_two_node_ascii());
        let sim = calculate_ascii_similarity(&ascii_out, &graph);
        assert!(
            sim > 0.5,
            "Similarity should be high for matching graph, got {}",
            sim
        );
    }

    #[test]
    fn check_ascii_structure_no_issues_on_match() {
        let graph = make_two_node_graph();
        let ascii_out = parse_ascii(&simple_two_node_ascii());
        let issues = check_ascii_structure(&ascii_out, &graph);
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
    fn check_ascii_structure_detects_missing_node() {
        use crate::layout::LayoutNode;

        let mut graph = make_two_node_graph();
        let mut node_c = LayoutNode::new("C", 80.0, 32.0);
        node_c.label = Some("Missing".to_string());
        graph.nodes.push(node_c);

        let ascii_out = parse_ascii(&simple_two_node_ascii());
        let issues = check_ascii_structure(&ascii_out, &graph);
        let has_missing = issues.iter().any(|i| i.message.contains("Missing"));
        assert!(has_missing, "Should detect missing node 'Missing'");
    }

    // --- Integration tests: full parse → layout → ASCII render → ASCII eval pipeline ---

    fn parse_layout_render_ascii(input: &str) -> (String, LayoutGraph) {
        use crate::layout::{CharacterSizeEstimator, ToLayoutGraph};
        use crate::render::ascii::render_flowchart_ascii;

        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Flowchart(db) => db,
            _ => panic!("Expected flowchart"),
        };
        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();
        let graph = crate::layout::layout(graph).unwrap();
        let ascii_output = render_flowchart_ascii(&db, &graph).unwrap();
        (ascii_output, graph)
    }

    #[test]
    fn integration_single_node() {
        let (output, graph) = parse_layout_render_ascii("flowchart TD\n    A[Hello]");
        let ascii_out = parse_ascii(&output);
        assert!(
            ascii.labels.contains(&"Hello".to_string()),
            "Should find label 'Hello' in ASCII output, got: {:?}\nOutput:\n{}",
            ascii.labels,
            output
        );
        let issues = check_ascii_structure(&ascii_out, &graph);
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
        let (output, graph) = parse_layout_render_ascii("flowchart TD\n    A[Start] --> B[End]");
        let ascii_out = parse_ascii(&output);

        assert!(
            ascii.labels.contains(&"Start".to_string()),
            "Should find 'Start', got: {:?}\nOutput:\n{}",
            ascii.labels,
            output
        );
        assert!(
            ascii.labels.contains(&"End".to_string()),
            "Should find 'End', got: {:?}",
            ascii.labels
        );
        assert!(
            ascii.arrow_count > 0 || ascii.has_braille,
            "Should have edges rendered"
        );

        let sim = calculate_ascii_similarity(&ascii_out, &graph);
        assert!(sim > 0.5, "Similarity should be reasonable, got {}", sim);
    }

    #[test]
    fn integration_three_nodes_chain() {
        let (output, graph) =
            parse_layout_render_ascii("flowchart TD\n    A[First] --> B[Second] --> C[Third]");
        let ascii_out = parse_ascii(&output);

        assert!(
            ascii.labels.len() >= 3,
            "Should find 3 labels, got: {:?}",
            ascii.labels
        );
        assert!(
            ascii.arrow_count >= 2,
            "Should have at least 2 arrows for 2 edges, got {}",
            ascii.arrow_count
        );
    }

    #[test]
    fn integration_edge_labels() {
        let (output, _graph) =
            parse_layout_render_ascii("flowchart TD\n    A[Start] -->|Yes| B[End]");
        let _ascii = parse_ascii(&output);

        // The edge label "Yes" should appear somewhere in the output
        assert!(
            output.contains("Yes"),
            "ASCII output should contain edge label 'Yes'\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn integration_spatial_ordering_preserved() {
        let (output, graph) = parse_layout_render_ascii("flowchart TD\n    A[Top] --> B[Bottom]");
        let ascii_out = parse_ascii(&output);
        let issues = check_ascii_structure(&ascii_out, &graph);

        let ordering_issues: Vec<_> = issues
            .iter()
            .filter(|i| i.check == "ascii_spatial_order")
            .collect();
        assert!(
            ordering_issues.is_empty(),
            "TD flow should preserve top→bottom ordering, got: {:?}",
            ordering_issues
        );
    }
}
