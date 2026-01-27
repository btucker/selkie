//! State diagram renderer

use std::collections::HashMap;

use crate::diagrams::state::{NotePosition, State, StateDb, StateType};
use crate::error::Result;
use crate::layout::Point;
use crate::layout::{
    create_size_estimator, layout, LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode,
    LayoutOptions, LayoutRanker, NodeShape, NodeSizeConfig, Padding, SizeEstimator, ToLayoutGraph,
};
use crate::render::svg::edges::{build_curved_path, build_curved_path_with_options};
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Generate SVG path for a rounded rectangle
/// This is used instead of <rect> elements to match mermaid reference output
fn rounded_rect_path(x: f64, y: f64, width: f64, height: f64, rx: f64, ry: f64) -> String {
    let right = x + width;
    let bottom = y + height;
    format!(
        "M {} {} H {} A {} {} 0 0 1 {} {} V {} A {} {} 0 0 1 {} {} H {} A {} {} 0 0 1 {} {} V {} A {} {} 0 0 1 {} {} Z",
        x + rx,
        y,
        right - rx,
        rx,
        ry,
        right,
        y + ry,
        bottom - ry,
        rx,
        ry,
        right - rx,
        bottom,
        x + rx,
        rx,
        ry,
        x,
        bottom - ry,
        y + ry,
        rx,
        ry,
        x + rx,
        y
    )
}

/// Generate SVG path for a circle
/// This is used instead of <circle> elements to match mermaid reference output
/// which uses rough.js that renders all shapes as paths
fn circle_path(cx: f64, cy: f64, r: f64) -> String {
    // Draw a circle using two arc commands
    // Start at (cx - r, cy), draw top half arc, then bottom half arc
    format!(
        "M {} {} A {} {} 0 1 0 {} {} A {} {} 0 1 0 {} {} Z",
        cx - r,
        cy,
        r,
        r,
        cx + r,
        cy,
        r,
        r,
        cx - r,
        cy
    )
}

/// Calculate the rendered bounds of a composite state
/// Returns (x, y, width, height) of the composite box
///
/// This function recursively calculates bounds from state_positions to ensure
/// all positions are consistent after any post-processing shifts.
fn calculate_composite_bounds(
    composite_id: &str,
    db: &StateDb,
    state_positions: &HashMap<String, (f64, f64, f64, f64)>,
) -> Option<(f64, f64, f64, f64)> {
    calculate_composite_bounds_recursive(composite_id, db, state_positions)
}

/// Recursively calculate composite bounds from state_positions only
fn calculate_composite_bounds_recursive(
    composite_id: &str,
    db: &StateDb,
    state_positions: &HashMap<String, (f64, f64, f64, f64)>,
) -> Option<(f64, f64, f64, f64)> {
    let states = db.get_states();
    let child_ids: Vec<&str> = states
        .iter()
        .filter(|(_, state)| state.parent.as_deref() == Some(composite_id))
        .map(|(id, _)| id.as_str())
        .collect();

    if child_ids.is_empty() {
        return None;
    }

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for child_id in &child_ids {
        if let Some(&(x, y, w, h)) = state_positions.get(*child_id) {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + w);
            max_y = max_y.max(y + h);
        }

        // If this child is a composite, recursively include its bounds
        let is_composite = states
            .values()
            .any(|s| s.parent.as_deref() == Some(child_id));
        if is_composite {
            if let Some((nested_x, nested_y, nested_w, nested_h)) =
                calculate_composite_bounds_recursive(child_id, db, state_positions)
            {
                min_x = min_x.min(nested_x);
                min_y = min_y.min(nested_y);
                max_x = max_x.max(nested_x + nested_w);
                max_y = max_y.max(nested_y + nested_h);
            }
        }
    }

    if min_x == f64::MAX {
        return None;
    }

    // Apply padding matching render_composite_state
    let padding = 12.0; // Balance between mermaid's 8px and visual spacing needs
    let title_height = 25.0;
    min_x -= padding;
    min_y -= padding + title_height;
    max_x += padding;
    max_y += padding;

    Some((min_x, min_y, max_x - min_x, max_y - min_y))
}

/// Result of recursively laying out a level of the state diagram
/// Mermaid's approach: each composite level gets its own dagre layout
#[derive(Clone, Debug)]
struct LevelLayout {
    /// Width of this level's content (after expansion for rendering)
    width: f64,
    /// Height of this level's content
    height: f64,
    /// Positions of nodes at this level (relative to level origin)
    positions: HashMap<String, (f64, f64, f64, f64)>,
    /// Edges at this level with their bend points
    edges: Vec<LayoutEdge>,
}

/// Recursively compute layout for a level of the state diagram
/// This mimics mermaid's renderDoc() which creates a dagre graph per composite level
fn compute_level_layout(
    parent_id: Option<&str>,
    db: &StateDb,
    size_estimator: &dyn SizeEstimator,
    level_layouts: &mut HashMap<String, LevelLayout>,
    depth: usize, // Nesting depth for spacing adjustment
) -> Result<Option<LevelLayout>> {
    use std::collections::HashSet;

    let states = db.get_states();
    let relations = db.get_relations();

    // Find states at this level (direct children of parent_id, or root-level if None)
    let level_state_ids: HashSet<&str> = states
        .iter()
        .filter(|(_, s)| s.parent.as_deref() == parent_id)
        .map(|(id, _)| id.as_str())
        .collect();

    if level_state_ids.is_empty() {
        return Ok(None);
    }

    // Identify which of these states are composites (have children)
    let composite_ids: HashSet<&str> = states
        .values()
        .filter_map(|s| s.parent.as_deref())
        .filter(|parent| level_state_ids.contains(parent))
        .collect();

    // First, recursively compute layouts for composite children at this level
    for composite_id in &composite_ids {
        if let Some(mut inner_layout) = compute_level_layout(
            Some(composite_id),
            db,
            size_estimator,
            level_layouts,
            depth + 1, // Increment depth for nested composites
        )? {
            // Check if this is a leaf composite (no nested composites in its positions)
            let is_leaf_composite = !inner_layout
                .positions
                .keys()
                .any(|child_id| level_layouts.contains_key(child_id));

            // Apply width expansion to match mermaid's getBBox() behavior.
            // Mermaid measures the full rendered cluster including internal padding and margins.
            // Leaf composites need more expansion for visual padding balance.
            // Non-leaf composites also need significant expansion to match mermaid's wider sizing.
            let expansion_factor = if is_leaf_composite { 1.6 } else { 1.4 };
            let original_width = inner_layout.width;
            let expanded_width = original_width * expansion_factor;
            let width_offset = (expanded_width - original_width) / 2.0;

            // Shift all positions to center content within expanded width
            for (_, (x, _, _, _)) in inner_layout.positions.iter_mut() {
                *x += width_offset;
            }
            // Shift edge points and label positions
            for edge in &mut inner_layout.edges {
                for point in &mut edge.bend_points {
                    point.x += width_offset;
                }
                if let Some(ref mut pos) = edge.label_position {
                    pos.x += width_offset;
                }
            }
            inner_layout.width = expanded_width;

            level_layouts.insert(composite_id.to_string(), inner_layout);
        }
    }

    // Now create layout graph for this level
    // Mermaid's state nodes use 10px font (g.stateGroup text) and 24px height.
    // We use 16px font for better readability, but reduce min dimensions to
    // get closer to mermaid's overall sizing.
    let config = NodeSizeConfig {
        font_size: 16.0,         // Keep readable font size
        padding_horizontal: 6.0, // Reduced from 8.0 to tighten horizontal spacing
        padding_vertical: 6.0,   // Reduced from 8.0 to get closer to mermaid's 24px height
        min_width: 35.0,         // Reduced from 40.0 to allow narrower nodes
        min_height: 24.0,        // Match mermaid's node height
        max_width: Some(200.0),
    };

    let mut graph = LayoutGraph::new(parent_id.unwrap_or("root"));

    // Mermaid uses rankSpacing = 50 default. The per-level increment is kept lower
    // to prevent excessive height in deeply nested diagrams while maintaining
    // reasonable spacing at the root level.
    let base_ranksep = 50.0;
    let ranksep_per_level = 15.0; // Lower than mermaid's implicit +25 to control nested height
    let layer_spacing = base_ranksep + (depth as f64 * ranksep_per_level);

    graph.options = LayoutOptions {
        direction: db.preferred_direction(),
        node_spacing: 50.0, // mermaid's nodeSep
        layer_spacing,      // mermaid-style: ranksep + 25 per nesting level
        padding: Padding::uniform(8.0),
        ranker: LayoutRanker::LongestPath, // mermaid uses 'tight-tree', LongestPath is equivalent
    };

    let start_end_states = determine_start_end_states(db);

    // Add nodes for states at this level
    for state_id in &level_state_ids {
        let state = states.get(*state_id).unwrap();

        let (shape, label) = match state.state_type {
            StateType::Start => (NodeShape::Circle, None),
            StateType::End => (NodeShape::DoubleCircle, None),
            StateType::Fork | StateType::Join => (NodeShape::Rectangle, None),
            StateType::Choice => (NodeShape::Diamond, None),
            StateType::Divider => (NodeShape::Rectangle, None),
            StateType::Default => {
                // Check if this is a start/end state (ID ends with _start or _end)
                if let Some(info) = start_end_states.get(*state_id) {
                    if info.is_start {
                        (NodeShape::Circle, None)
                    } else {
                        (NodeShape::DoubleCircle, None)
                    }
                } else {
                    let desc = state.description.as_deref().filter(|d| !d.is_empty());
                    (NodeShape::RoundedRect, desc.or(Some(*state_id)))
                }
            }
        };

        // Start/end states have no label (IDs end with _start or _end)
        let is_start_end = start_end_states.contains_key(*state_id);
        let label_text = label.unwrap_or(if is_start_end { "" } else { *state_id });

        // Determine dimensions - use pre-computed layout for composites
        // Use the full `width` (including expansion) so parent composites
        // properly include expanded children.
        let (width, height) = if composite_ids.contains(state_id) {
            if let Some(inner) = level_layouts.get(*state_id) {
                let padding = 12.0;
                let title_height = 25.0;
                (
                    inner.width + 2.0 * padding,
                    inner.height + 2.0 * padding + title_height,
                )
            } else {
                (100.0, 60.0)
            }
        } else if is_start_end || matches!(state.state_type, StateType::Start | StateType::End) {
            (14.0, 14.0)
        } else if matches!(state.state_type, StateType::Fork | StateType::Join) {
            (70.0, 10.0)
        } else {
            size_estimator.estimate_node_size(Some(label_text), shape, &config)
        };

        let mut node = LayoutNode::new(*state_id, width, height).with_shape(shape);
        if !label_text.is_empty() {
            node = node.with_label(label_text);
        }
        if composite_ids.contains(state_id) {
            node.metadata
                .insert("is_group".to_string(), "true".to_string());
        }
        node.metadata
            .insert("state_type".to_string(), format!("{:?}", state.state_type));

        graph.add_node(node);
    }

    // Add edges for relations where BOTH endpoints are at this level
    for (i, relation) in relations.iter().enumerate() {
        let s1 = relation.state1.as_str();
        let s2 = relation.state2.as_str();

        if level_state_ids.contains(s1) && level_state_ids.contains(s2) {
            let mut edge = LayoutEdge::new(
                format!("e{}", i),
                relation.state1.clone(),
                relation.state2.clone(),
            );
            if let Some(ref desc) = relation.description {
                edge = edge.with_label(desc);
            }
            graph.add_edge(edge);
        }
    }

    // Handle cross-level edges: when a state at this level receives edges from
    // or sends edges to states inside composites, add virtual edges to/from the
    // outermost composite at this level. This ensures proper positioning.
    let mut virtual_edge_count = 0;
    for relation in relations.iter() {
        let s1 = relation.state1.as_str();
        let s2 = relation.state2.as_str();

        let s1_at_level = level_state_ids.contains(s1);
        let s2_at_level = level_state_ids.contains(s2);

        // Skip if both at this level (already handled) or neither at this level
        if s1_at_level == s2_at_level {
            continue;
        }

        // Find which composite at this level contains the "other" state
        let find_containing_composite = |state_id: &str| -> Option<&str> {
            // Walk up the parent chain until we find a composite at this level
            let mut current = states.get(state_id)?.parent.as_deref();
            while let Some(p) = current {
                if level_state_ids.contains(p) && composite_ids.contains(p) {
                    return Some(p);
                }
                current = states.get(p)?.parent.as_deref();
            }
            None
        };

        if s1_at_level && !s2_at_level {
            // Source at this level, target inside a composite
            // Add virtual edge: source -> composite (to position source before composite)
            if let Some(composite_id) = find_containing_composite(s2) {
                let mut edge = LayoutEdge::new(
                    format!("virtual_{}", virtual_edge_count),
                    s1.to_string(),
                    composite_id.to_string(),
                );
                // Preserve edge label for layout - affects horizontal spacing
                if let Some(ref desc) = relation.description {
                    edge = edge.with_label(desc);
                }
                graph.add_edge(edge);
                virtual_edge_count += 1;
            }
        } else if !s1_at_level && s2_at_level {
            // Source inside a composite, target at this level
            // Add virtual edge: composite -> target (to position target after composite)
            if let Some(composite_id) = find_containing_composite(s1) {
                let mut edge = LayoutEdge::new(
                    format!("virtual_{}", virtual_edge_count),
                    composite_id.to_string(),
                    s2.to_string(),
                );
                // Preserve edge label for layout - affects horizontal spacing
                if let Some(ref desc) = relation.description {
                    edge = edge.with_label(desc);
                }
                graph.add_edge(edge);
                virtual_edge_count += 1;
            }
        }
    }

    // Run layout
    let layout_result = layout(graph)?;

    // Extract positions and compute bounding box
    let mut positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for node in &layout_result.nodes {
        if let (Some(x), Some(y)) = (node.x, node.y) {
            positions.insert(node.id.clone(), (x, y, node.width, node.height));
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + node.width);
            max_y = max_y.max(y + node.height);
        }
    }

    // Include edge label bounds - mermaid includes labels in cluster sizing
    for edge in &layout_result.edges {
        if let Some(label_pos) = &edge.label_position {
            if edge.label_width > 0.0 {
                // Labels are centered on their position
                let label_left = label_pos.x - edge.label_width / 2.0;
                let label_right = label_pos.x + edge.label_width / 2.0;
                let label_top = label_pos.y - edge.label_height / 2.0;
                let label_bottom = label_pos.y + edge.label_height / 2.0;

                min_x = min_x.min(label_left);
                max_x = max_x.max(label_right);
                min_y = min_y.min(label_top);
                max_y = max_y.max(label_bottom);
            }
        }
    }

    if min_x == f64::MAX {
        return Ok(None);
    }

    // Normalize positions to start from (0, 0)
    for (_, (x, y, _, _)) in positions.iter_mut() {
        *x -= min_x;
        *y -= min_y;
    }

    // Also normalize edge bend points and label positions
    let mut edges = layout_result.edges;
    for edge in &mut edges {
        for point in &mut edge.bend_points {
            point.x -= min_x;
            point.y -= min_y;
        }
        if let Some(ref mut pos) = edge.label_position {
            pos.x -= min_x;
            pos.y -= min_y;
        }
    }

    let computed_width = max_x - min_x;
    Ok(Some(LevelLayout {
        width: computed_width,
        height: max_y - min_y,
        positions,
        edges,
    }))
}

/// Implement ToLayoutGraph for StateDb to enable proper DAG layout
impl ToLayoutGraph for StateDb {
    fn to_layout_graph(&self, size_estimator: &dyn SizeEstimator) -> Result<LayoutGraph> {
        use std::collections::HashSet;

        // Mermaid's state nodes use 10px font (g.stateGroup text) and 24px height.
        // We use 16px font for better readability, but reduce min dimensions to
        // get closer to mermaid's overall sizing.
        let config = NodeSizeConfig {
            font_size: 16.0,         // Keep readable font size
            padding_horizontal: 6.0, // Reduced from 8.0 to tighten horizontal spacing
            padding_vertical: 6.0,   // Reduced from 8.0 to get closer to mermaid's 24px height
            min_width: 35.0,         // Reduced from 40.0 to allow narrower nodes
            min_height: 24.0,        // Match mermaid's node height
            max_width: Some(200.0),
        };
        let mut graph = LayoutGraph::new("state");

        // Set layout options from diagram direction
        // Following mermaid's state diagram config (stateRenderer-v3-unified.ts line 64-65):
        // nodeSpacing = 50, rankSpacing = 50
        graph.options = LayoutOptions {
            direction: self.preferred_direction(),
            node_spacing: 50.0,                // mermaid's nodeSpacing
            layer_spacing: 50.0,               // mermaid's rankSpacing
            padding: Padding::uniform(8.0),    // mermaid's marginx/marginy
            ranker: LayoutRanker::LongestPath, // Use longest-path (mermaid's tight-tree base)
        };

        // Determine start/end states based on transitions
        let start_end_states = determine_start_end_states(self);

        // Identify composite states (states that are parents of other states)
        let states = self.get_states();
        let composite_states: HashSet<&str> = states
            .values()
            .filter_map(|s| s.parent.as_deref())
            .collect();

        // Add composite state nodes FIRST (like flowchart subgraphs)
        // These have zero dimensions initially - dagre expands them based on children
        for composite_id in &composite_states {
            if let Some(state) = states.get(*composite_id) {
                let mut node = LayoutNode::new(*composite_id, 0.0, 0.0)
                    .with_shape(NodeShape::Rectangle)
                    .with_label(&state.id);

                // Set parent if this composite is nested inside another
                if let Some(parent_id) = &state.parent {
                    node = node.with_parent(parent_id);
                }

                // Mark as a group (consistent with flowchart subgraphs)
                node.metadata
                    .insert("is_group".to_string(), "true".to_string());
                node.metadata
                    .insert("state_type".to_string(), format!("{:?}", state.state_type));

                graph.add_node(node);
            }
        }

        // Convert non-composite states to layout nodes (sorted for deterministic order)
        let mut state_ids: Vec<&String> = states.keys().collect();
        state_ids.sort();

        for id in state_ids {
            // Skip composite states - already added above
            if composite_states.contains(id.as_str()) {
                continue;
            }

            let state = states.get(id).unwrap();

            // Determine shape based on state type
            let (shape, label) = match state.state_type {
                StateType::Start => (NodeShape::Circle, None),
                StateType::End => (NodeShape::DoubleCircle, None),
                StateType::Fork | StateType::Join => (NodeShape::Rectangle, None),
                StateType::Choice => (NodeShape::Diamond, None),
                StateType::Divider => (NodeShape::Rectangle, None),
                StateType::Default => {
                    // Check if this is a start/end state (ID ends with _start or _end)
                    if let Some(state_info) = start_end_states.get(id.as_str()) {
                        if state_info.is_start {
                            (NodeShape::Circle, None)
                        } else {
                            (NodeShape::DoubleCircle, None)
                        }
                    } else {
                        let desc = state.descriptions.first().map(|s| s.as_str());
                        (NodeShape::RoundedRect, desc.or(Some(id.as_str())))
                    }
                }
            };

            // Start/end states have no label (IDs end with _start or _end)
            let is_start_end = start_end_states.contains_key(id.as_str());
            let label_text = label.unwrap_or(if is_start_end { "" } else { &state.id });

            // Size based on state type
            let (width, height) =
                if is_start_end || matches!(state.state_type, StateType::Start | StateType::End) {
                    // Small fixed size for start/end circles (r=7, diameter=14)
                    (14.0, 14.0)
                } else if matches!(state.state_type, StateType::Fork | StateType::Join) {
                    // Horizontal bar matching mermaid reference (70×10)
                    (70.0, 10.0)
                } else {
                    size_estimator.estimate_node_size(Some(label_text), shape, &config)
                };

            let mut node = LayoutNode::new(id, width, height).with_shape(shape);

            if !label_text.is_empty() {
                node = node.with_label(label_text);
            }

            // Set parent relationship for compound graph layout
            if let Some(parent_id) = &state.parent {
                node = node.with_parent(parent_id);
            }

            // Store state type in metadata
            node.metadata
                .insert("state_type".to_string(), format!("{:?}", state.state_type));

            graph.add_node(node);
        }

        // Convert relations to edges
        for (i, relation) in self.get_relations().iter().enumerate() {
            let edge_id = format!("transition-{}", i);
            let mut edge = LayoutEdge::new(&edge_id, &relation.state1, &relation.state2);

            if let Some(desc) = &relation.description {
                edge = edge.with_label(desc);
            }

            graph.add_edge(edge);
        }

        Ok(graph)
    }

    fn preferred_direction(&self) -> LayoutDirection {
        self.get_direction().into()
    }
}

/// Center all composite states on a common vertical axis
///
/// Mermaid's state diagrams center all composite states at the same X position.
/// This post-processing step shifts composites and their children to align on
/// a common vertical centerline, creating a narrower, more balanced diagram.
/// Returns a HashMap of composite ID to X offset applied, so edge bend points can be shifted.
fn center_composite_states(
    db: &StateDb,
    state_positions: &mut HashMap<String, (f64, f64, f64, f64)>,
    composite_ids: &std::collections::HashSet<&str>,
) -> HashMap<String, f64> {
    let mut composite_offsets: HashMap<String, f64> = HashMap::new();

    if composite_ids.is_empty() {
        return composite_offsets;
    }

    let states = db.get_states();

    // Find top-level composites (those not nested inside other composites)
    let top_level_composites: Vec<&str> = composite_ids
        .iter()
        .filter(|id| {
            // Check if this composite's parent is also a composite
            states
                .get(**id)
                .and_then(|s| s.parent.as_deref())
                .map(|parent| !composite_ids.contains(parent))
                .unwrap_or(true)
        })
        .copied()
        .collect();

    if top_level_composites.is_empty() {
        return composite_offsets;
    }

    // Calculate the bounds of each composite (including nested children)
    // and find the diagram's ideal center X
    let mut composite_bounds: HashMap<&str, (f64, f64)> = HashMap::new(); // (left_x, right_x)
    let mut diagram_min_x = f64::MAX;
    let mut diagram_max_x = f64::MIN;

    for &comp_id in &top_level_composites {
        // Calculate the bounding box of this composite and all its children
        let (left, right) =
            calculate_composite_x_bounds(comp_id, db, state_positions, composite_ids);
        if left < f64::MAX && right > f64::MIN {
            composite_bounds.insert(comp_id, (left, right));
            diagram_min_x = diagram_min_x.min(left);
            diagram_max_x = diagram_max_x.max(right);
        }
    }

    // Also include non-composite nodes at root level (start/end, fork/join)
    for (id, (x, _, w, _)) in state_positions.iter() {
        let parent = states.get(id).and_then(|s| s.parent.as_deref());
        if parent.is_none() && !composite_ids.contains(id.as_str()) {
            diagram_min_x = diagram_min_x.min(*x);
            diagram_max_x = diagram_max_x.max(*x + w);
        }
    }

    if diagram_min_x >= diagram_max_x {
        return composite_offsets;
    }

    // The diagram center X
    let diagram_center_x = (diagram_min_x + diagram_max_x) / 2.0;

    // Center each top-level composite at the diagram center
    for &comp_id in &top_level_composites {
        if let Some(&(left, right)) = composite_bounds.get(comp_id) {
            let comp_center = (left + right) / 2.0;
            let offset_x = diagram_center_x - comp_center;

            // Track the offset applied to this composite
            composite_offsets.insert(comp_id.to_string(), offset_x);

            // Shift all nodes belonging to this composite (and nested composites)
            shift_composite_and_children(comp_id, offset_x, db, state_positions, composite_ids);
        }
    }

    composite_offsets
}

/// Calculate the X bounds of a composite state including all nested children
/// Calculate x bounds for a composite's content (excluding nested composites for centering)
/// This is used during centering to find where to center nested composites within
fn calculate_composite_x_bounds_for_centering(
    comp_id: &str,
    db: &StateDb,
    state_positions: &HashMap<String, (f64, f64, f64, f64)>,
    composite_ids: &std::collections::HashSet<&str>,
) -> (f64, f64) {
    let states = db.get_states();
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;

    // For centering, we only consider NON-composite direct children
    // Nested composites will be centered within these bounds, so don't include them
    for (id, state) in states.iter() {
        if state.parent.as_deref() == Some(comp_id) {
            let is_nested_composite = composite_ids.contains(id.as_str());

            if !is_nested_composite {
                // For non-composite children, use state_positions directly
                if let Some(&(x, _, w, _)) = state_positions.get(id) {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x + w);
                }
            }
        }
    }

    (min_x, max_x)
}

fn calculate_composite_x_bounds(
    comp_id: &str,
    db: &StateDb,
    state_positions: &HashMap<String, (f64, f64, f64, f64)>,
    composite_ids: &std::collections::HashSet<&str>,
) -> (f64, f64) {
    let states = db.get_states();
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;

    // Find all states that belong to this composite (direct children)
    for (id, state) in states.iter() {
        if state.parent.as_deref() == Some(comp_id) {
            // Check if this child is a nested composite
            let is_nested_composite = composite_ids.contains(id.as_str());

            if is_nested_composite {
                // For nested composites, ONLY use recursive bounds calculation
                // Don't use state_positions entry as it's the dagre node position,
                // which may not match the rendered bounds after post-processing
                let (nested_min, nested_max) =
                    calculate_composite_x_bounds(id, db, state_positions, composite_ids);
                min_x = min_x.min(nested_min);
                max_x = max_x.max(nested_max);
            } else {
                // For non-composite children, use state_positions directly
                if let Some(&(x, _, w, _)) = state_positions.get(id) {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x + w);
                }
            }
        }
    }

    (min_x, max_x)
}

/// Shift a composite and all its children by offset_x
fn shift_composite_and_children(
    comp_id: &str,
    offset_x: f64,
    db: &StateDb,
    state_positions: &mut HashMap<String, (f64, f64, f64, f64)>,
    composite_ids: &std::collections::HashSet<&str>,
) {
    let states = db.get_states();

    // Shift all states that belong to this composite
    for (id, state) in states.iter() {
        if state.parent.as_deref() == Some(comp_id) {
            if let Some((x, y, w, h)) = state_positions.get(id).copied() {
                state_positions.insert(id.clone(), (x + offset_x, y, w, h));
            }

            // If this is a nested composite, recursively shift its children
            if composite_ids.contains(id.as_str()) {
                shift_composite_and_children(id, offset_x, db, state_positions, composite_ids);
            }
        }
    }
}

/// Center nested composite states within their parent composites
///
/// After dagre layout, nested composites may not be horizontally centered within
/// their parent. This post-processing step shifts nested composites (and their
/// children) so they are centered within their parent's bounds.
/// Returns a HashMap of nested composite ID to X offset applied.
fn center_nested_composites(
    db: &StateDb,
    state_positions: &mut HashMap<String, (f64, f64, f64, f64)>,
    composite_ids: &std::collections::HashSet<&str>,
) -> HashMap<String, f64> {
    let mut nested_offsets: HashMap<String, f64> = HashMap::new();
    let states = db.get_states();

    // Find nested composites (composites whose parent is also a composite)
    // Sort by depth so we process deepest nested first (innermost to outermost)
    let mut nested_composites: Vec<(&str, &str, usize)> = composite_ids
        .iter()
        .filter_map(|&id| {
            let parent = states.get(id)?.parent.as_deref()?;
            if composite_ids.contains(parent) {
                // Calculate depth
                let mut depth = 1;
                let mut current = parent;
                while let Some(grandparent) = states.get(current).and_then(|s| s.parent.as_deref())
                {
                    if composite_ids.contains(grandparent) {
                        depth += 1;
                    }
                    current = grandparent;
                }
                Some((id, parent, depth))
            } else {
                None
            }
        })
        .collect();

    // Sort by depth descending (deepest first) so inner composites are centered
    // before their parents, ensuring consistent bounds calculations
    nested_composites.sort_by(|a, b| b.2.cmp(&a.2));

    // Process from innermost to outermost
    for (nested_id, parent_id, _depth) in nested_composites {
        // Calculate the parent's bounds from NON-COMPOSITE children only
        // This gives us the "anchor" content that the nested composite should be centered with
        let (parent_non_comp_min_x, parent_non_comp_max_x) =
            calculate_composite_x_bounds_for_centering(
                parent_id,
                db,
                state_positions,
                composite_ids,
            );

        // Calculate nested composite's RENDERED bounds (with padding)
        let nested_rendered = calculate_composite_bounds_recursive(nested_id, db, state_positions);

        let Some((nested_x, _, nested_w, _)) = nested_rendered else {
            continue;
        };

        // If there are non-composite children, center the nested composite with them
        // Otherwise, the nested composite is the only content and doesn't need centering
        if parent_non_comp_min_x >= parent_non_comp_max_x {
            continue;
        }

        // The non-composite children define the "anchor" center for the parent
        // The nested composite should be centered around this same center
        let non_comp_center_x = (parent_non_comp_min_x + parent_non_comp_max_x) / 2.0;

        // The nested composite's current center
        let nested_center_x = nested_x + nested_w / 2.0;

        // Calculate offset to align nested composite's center with non-composite center
        let offset_x = non_comp_center_x - nested_center_x;

        // Only shift if there's a meaningful offset
        if offset_x.abs() > 0.5 {
            // Track the offset applied to this nested composite
            nested_offsets.insert(nested_id.to_string(), offset_x);

            // Shift the nested composite itself
            if let Some((x, y, w, h)) = state_positions.get(nested_id).copied() {
                state_positions.insert(nested_id.to_string(), (x + offset_x, y, w, h));
            }

            // Shift all children of the nested composite
            shift_composite_and_children(nested_id, offset_x, db, state_positions, composite_ids);
        }
    }

    nested_offsets
}

/// Render a state diagram to SVG
pub fn render_state(db: &StateDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    // Layout constants (matching mermaid reference: r=7 for start/end circles)
    let start_end_radius = 7.0;
    let margin = 8.0; // Match mermaid's viewport padding (8px)

    // Determine which [*] states are starts vs ends based on transitions
    let start_end_states = determine_start_end_states(db);

    let states = db.get_states();

    if states.is_empty() {
        // Empty diagram
        doc.set_size(400.0, 200.0);
        if !db.diagram_title.is_empty() {
            let title_elem = SvgElement::Text {
                x: 200.0,
                y: 30.0,
                content: db.diagram_title.clone(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_class("state-title")
                    .with_attr("font-size", "20")
                    .with_attr("font-weight", "bold"),
            };
            doc.add_element(title_elem);
        }
        return Ok(doc.to_string());
    }

    // Use recursive layout like mermaid - each composite level gets its own dagre layout
    // This naturally accumulates spacing at each nesting level
    // Try to use font-based measurements for accuracy, fall back to character estimates
    let size_estimator = create_size_estimator();

    // Compute recursive layouts starting from root level
    let mut level_layouts: HashMap<String, LevelLayout> = HashMap::new();
    let root_layout = compute_level_layout(None, db, &*size_estimator, &mut level_layouts, 0)?
        .ok_or_else(|| {
            crate::error::MermaidError::LayoutError("No states to layout".to_string())
        })?;

    // Build final state_positions by combining all level layouts
    let mut state_positions: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();

    // Position all content starting from root AND collect edges with proper offsets
    // We need to track offsets for each level so we can transform edge bend points
    let mut level_offsets: HashMap<String, (f64, f64)> = HashMap::new();
    level_offsets.insert("root".to_string(), (0.0, 0.0));

    fn position_level_content_with_offsets(
        level_layout: &LevelLayout,
        offset_x: f64,
        offset_y: f64,
        level_layouts: &HashMap<String, LevelLayout>,
        state_positions: &mut HashMap<String, (f64, f64, f64, f64)>,
        level_offsets: &mut HashMap<String, (f64, f64)>,
    ) {
        for (id, (x, y, w, h)) in &level_layout.positions {
            let final_x = offset_x + x;
            let final_y = offset_y + y;
            state_positions.insert(id.clone(), (final_x, final_y, *w, *h));

            // If this is a composite, recursively position its contents
            if let Some(inner_layout) = level_layouts.get(id) {
                let padding = 12.0;
                let title_height = 25.0;
                let inner_offset_x = final_x + padding;
                let inner_offset_y = final_y + padding + title_height;
                level_offsets.insert(id.clone(), (inner_offset_x, inner_offset_y));
                position_level_content_with_offsets(
                    inner_layout,
                    inner_offset_x,
                    inner_offset_y,
                    level_layouts,
                    state_positions,
                    level_offsets,
                );
            }
        }
    }

    position_level_content_with_offsets(
        &root_layout,
        0.0,
        0.0,
        &level_layouts,
        &mut state_positions,
        &mut level_offsets,
    );

    // Collect all edges from all levels, applying parent offsets to bend points
    let mut all_edges: Vec<LayoutEdge> = root_layout.edges.clone();
    // Root level edges have no offset (already at origin)

    // Add edges from nested levels with their offsets applied
    for (level_id, level_layout) in &level_layouts {
        if let Some(&(off_x, off_y)) = level_offsets.get(level_id) {
            for edge in &level_layout.edges {
                let mut transformed_edge = edge.clone();
                for point in &mut transformed_edge.bend_points {
                    point.x += off_x;
                    point.y += off_y;
                }
                if let Some(ref mut pos) = transformed_edge.label_position {
                    pos.x += off_x;
                    pos.y += off_y;
                }
                all_edges.push(transformed_edge);
            }
        }
    }

    // Identify composite states
    let composite_ids: std::collections::HashSet<&str> =
        level_layouts.keys().map(|s| s.as_str()).collect();

    // Post-process: Center all composite states on a common vertical axis
    // This matches mermaid's rendering where all composites align vertically
    // Returns offsets per composite so we can shift edge bend points too
    let mut composite_offsets = center_composite_states(db, &mut state_positions, &composite_ids);

    // Post-process: Center nested composites within their parent composites
    // Dagre layout doesn't inherently center nested content, so we do it here
    // Merge nested offsets into composite_offsets for edge bend point adjustment
    // Note: nested offsets need to include parent's offset for correct edge positioning
    let nested_offsets = center_nested_composites(db, &mut state_positions, &composite_ids);
    for (id, nested_offset) in nested_offsets {
        // Get the parent's total offset (if any) and add to this nested offset
        let parent_offset = states
            .get(&id)
            .and_then(|s| s.parent.as_ref())
            .and_then(|parent| composite_offsets.get(parent))
            .copied()
            .unwrap_or(0.0);
        composite_offsets.insert(id, parent_offset + nested_offset);
    }

    // Post-process: Center start nodes above the composite states they connect to
    // This improves visual alignment matching the mermaid reference

    // Track which start nodes were repositioned and by how much
    let mut start_node_offsets: HashMap<String, f64> = HashMap::new();

    for edge in &all_edges {
        if let (Some(source), Some(target)) = (edge.source(), edge.target()) {
            // Check if this is a start node connecting to a composite
            // Start node IDs are in format {parent}_start (e.g., root_start, Idle_start)
            if source.ends_with("_start") && composite_ids.contains(target) {
                // Use calculate_composite_bounds to get actual rendered composite bounds
                // (not dagre's placeholder position which doesn't account for children)
                if let (Some((start_x, start_y, start_w, start_h)), Some((comp_x, _, comp_w, _))) = (
                    state_positions.get(source).copied(),
                    calculate_composite_bounds(target, db, &state_positions),
                ) {
                    // Center the start node horizontally above the composite
                    let comp_center_x = comp_x + comp_w / 2.0;
                    let new_start_x = comp_center_x - start_w / 2.0;
                    let offset_x = new_start_x - start_x;
                    state_positions
                        .insert(source.to_string(), (new_start_x, start_y, start_w, start_h));
                    start_node_offsets.insert(source.to_string(), offset_x);
                }
            }
        }
    }

    // Post-process: Center fork/join bars between their parallel branches
    // Fork bars should be centered between their target states
    // Join bars should be centered between their source states
    let mut fork_join_offsets: HashMap<String, f64> = HashMap::new();

    // Collect fork and join states
    let fork_join_ids: Vec<&str> = states
        .iter()
        .filter(|(_, s)| matches!(s.state_type, StateType::Fork | StateType::Join))
        .map(|(id, _)| id.as_str())
        .collect();

    for fj_id in &fork_join_ids {
        let is_fork = states
            .get(*fj_id)
            .map(|s| matches!(s.state_type, StateType::Fork))
            .unwrap_or(false);

        // Collect connected state centers (targets for fork, sources for join)
        let mut connected_centers: Vec<f64> = Vec::new();

        for edge in &all_edges {
            if let (Some(source), Some(target)) = (edge.source(), edge.target()) {
                if is_fork && source == *fj_id {
                    // Fork: collect target centers
                    if let Some(&(x, _, w, _)) = state_positions.get(target) {
                        connected_centers.push(x + w / 2.0);
                    }
                } else if !is_fork && target == *fj_id {
                    // Join: collect source centers
                    if let Some(&(x, _, w, _)) = state_positions.get(source) {
                        connected_centers.push(x + w / 2.0);
                    }
                }
            }
        }

        // Center fork/join between connected states
        if connected_centers.len() >= 2 {
            let min_x = connected_centers.iter().copied().fold(f64::MAX, f64::min);
            let max_x = connected_centers.iter().copied().fold(f64::MIN, f64::max);
            let ideal_center = (min_x + max_x) / 2.0;

            if let Some(&(fj_x, fj_y, fj_w, fj_h)) = state_positions.get(*fj_id) {
                let current_center = fj_x + fj_w / 2.0;
                let offset_x = ideal_center - current_center;
                let new_x = fj_x + offset_x;
                state_positions.insert(fj_id.to_string(), (new_x, fj_y, fj_w, fj_h));
                fork_join_offsets.insert(fj_id.to_string(), offset_x);
            }
        }
    }

    // Extract edge bend points and label positions from layout
    // Adjust bend points for edges from repositioned start nodes
    let mut edge_bend_points: HashMap<(String, String), Vec<Point>> = HashMap::new();
    let mut edge_label_positions: HashMap<(String, String), Point> = HashMap::new();
    for edge in &all_edges {
        if let (Some(source), Some(target)) = (edge.source(), edge.target()) {
            let mut bend_points = edge.bend_points.clone();
            let mut label_pos = edge.label_position;

            // If both source and target are inside a shifted composite,
            // apply the composite offset to all edge bend points and label position
            // This keeps edges aligned with their shifted parent states
            // We need to accumulate offsets from ALL ancestor composites, not just the direct parent
            if !bend_points.is_empty() {
                let source_parent = states.get(source).and_then(|s| s.parent.as_deref());
                let target_parent = states.get(target).and_then(|s| s.parent.as_deref());

                // Check if they share a common ancestor composite that was shifted
                if let (Some(sp), Some(tp)) = (source_parent, target_parent) {
                    // If both have the same parent composite
                    if sp == tp {
                        // Accumulate offsets from this parent and all ancestor composites
                        let mut total_offset = 0.0;
                        let mut current_composite: Option<&str> = Some(sp);
                        while let Some(comp) = current_composite {
                            if let Some(&offset_x) = composite_offsets.get(comp) {
                                total_offset += offset_x;
                            }
                            // Move to parent composite
                            current_composite = states
                                .get(comp)
                                .and_then(|s| s.parent.as_deref())
                                .filter(|p| composite_ids.contains(p));
                        }

                        if total_offset.abs() > 0.001 {
                            for point in &mut bend_points {
                                point.x += total_offset;
                            }
                            // Also shift the label position
                            if let Some(ref mut lp) = label_pos {
                                lp.x += total_offset;
                            }
                        }
                    }
                }
            }

            // If the source is a repositioned start node going to a composite,
            // make the edge a straight vertical line by aligning all points
            if let Some(&offset_x) = start_node_offsets.get(source) {
                if composite_ids.contains(target) && !bend_points.is_empty() {
                    // Get the new start position x coordinate (center)
                    let new_x = bend_points[0].x + offset_x;
                    // Make all points vertically aligned for a straight edge
                    for point in &mut bend_points {
                        point.x = new_x;
                    }
                } else if !bend_points.is_empty() {
                    // For non-composite targets, just shift the first point
                    bend_points[0].x += offset_x;
                }
            }

            // If the source is a composite state, adjust the first bend point
            // to exit from the center-bottom of the rendered composite box
            if composite_ids.contains(source) && !bend_points.is_empty() {
                if let Some((comp_x, comp_y, comp_w, comp_h)) =
                    calculate_composite_bounds(source, db, &state_positions)
                {
                    let comp_center_x = comp_x + comp_w / 2.0;
                    let comp_bottom_y = comp_y + comp_h;

                    // If target is a fork/join, use the fork's center X for a straight edge
                    // Otherwise use the composite's center
                    let target_x = if fork_join_ids.contains(&target) {
                        state_positions
                            .get(target)
                            .map(|(x, _, w, _)| x + w / 2.0)
                            .unwrap_or(comp_center_x)
                    } else {
                        comp_center_x
                    };

                    // Set the first bend point to exit from composite at target's X
                    bend_points[0].x = target_x;
                    bend_points[0].y = comp_bottom_y;

                    // If target is a fork/join, make all points align vertically
                    // for a straight edge (matches mermaid behavior)
                    if fork_join_ids.contains(&target) {
                        for point in &mut bend_points {
                            point.x = target_x;
                        }
                    }
                }
            }

            // If the target is a fork/join state, use rectangle intersection algorithm
            // (same approach as mermaid reference: intersect-rect.js)
            if fork_join_ids.contains(&target) && !bend_points.is_empty() {
                if let Some(&(fj_x, fj_y, fj_w, fj_h)) = state_positions.get(target) {
                    let last_idx = bend_points.len() - 1;
                    // Get the source point for intersection calculation
                    let (point_x, point_y) = if bend_points.len() > 1 {
                        (bend_points[last_idx - 1].x, bend_points[last_idx - 1].y)
                    } else if let Some(&(sx, sy, sw, sh)) = state_positions.get(source) {
                        (sx + sw / 2.0, sy + sh / 2.0)
                    } else {
                        (bend_points[last_idx].x, bend_points[last_idx].y - 50.0)
                    };

                    // Rectangle intersection algorithm (mermaid style)
                    let node_x = fj_x + fj_w / 2.0;
                    let node_y = fj_y + fj_h / 2.0;
                    let dx = point_x - node_x;
                    let dy = point_y - node_y;
                    let w = fj_w / 2.0;
                    let h = fj_h / 2.0;

                    let (sx, sy) = if (dy.abs() * w) > (dx.abs() * h) {
                        // Intersection is top or bottom
                        let h_signed = if dy < 0.0 { -h } else { h };
                        let sx = if dy.abs() < 0.001 {
                            0.0
                        } else {
                            (h_signed * dx) / dy
                        };
                        (sx, h_signed)
                    } else {
                        // Intersection is left or right
                        let w_signed = if dx < 0.0 { -w } else { w };
                        let sy = if dx.abs() < 0.001 {
                            0.0
                        } else {
                            (w_signed * dy) / dx
                        };
                        (w_signed, sy)
                    };

                    bend_points[last_idx].x = node_x + sx;
                    bend_points[last_idx].y = node_y + sy;
                }
            }

            // If the target is a composite state, adjust the last bend point
            // to hit the top-center of the rendered composite box (not the border node)
            if composite_ids.contains(target) && !bend_points.is_empty() {
                if let Some((comp_x, comp_y, comp_w, _comp_h)) =
                    calculate_composite_bounds(target, db, &state_positions)
                {
                    let comp_center_x = comp_x + comp_w / 2.0;
                    let comp_top_y = comp_y;

                    // If source is a fork/join, use the fork's center X for a straight edge
                    // Otherwise use the composite's center
                    let source_x = if fork_join_ids.contains(&source) {
                        state_positions
                            .get(source)
                            .map(|(x, _, w, _)| x + w / 2.0)
                            .unwrap_or(comp_center_x)
                    } else {
                        comp_center_x
                    };

                    let last_idx = bend_points.len() - 1;
                    bend_points[last_idx].x = source_x;
                    bend_points[last_idx].y = comp_top_y;

                    // If source is a fork/join, make all points align vertically
                    // for a straight edge (matches mermaid behavior)
                    if fork_join_ids.contains(&source) {
                        for point in &mut bend_points {
                            point.x = source_x;
                        }
                    }
                }
            }

            // If the source is a fork/join state, use rectangle intersection algorithm
            // (same approach as mermaid reference: intersect-rect.js)
            if fork_join_ids.contains(&source) && !bend_points.is_empty() {
                if let Some(&(fj_x, fj_y, fj_w, fj_h)) = state_positions.get(source) {
                    // Get the target point for intersection calculation
                    let (point_x, point_y) = if bend_points.len() > 1 {
                        (bend_points[1].x, bend_points[1].y)
                    } else if let Some(&(tx, ty, tw, th)) = state_positions.get(target) {
                        (tx + tw / 2.0, ty + th / 2.0)
                    } else {
                        (bend_points[0].x, bend_points[0].y + 50.0)
                    };

                    // Rectangle intersection algorithm (mermaid style)
                    let node_x = fj_x + fj_w / 2.0;
                    let node_y = fj_y + fj_h / 2.0;
                    let dx = point_x - node_x;
                    let dy = point_y - node_y;
                    let w = fj_w / 2.0;
                    let h = fj_h / 2.0;

                    let (sx, sy) = if (dy.abs() * w) > (dx.abs() * h) {
                        // Intersection is top or bottom
                        let h_signed = if dy < 0.0 { -h } else { h };
                        let sx = if dy.abs() < 0.001 {
                            0.0
                        } else {
                            (h_signed * dx) / dy
                        };
                        (sx, h_signed)
                    } else {
                        // Intersection is left or right
                        let w_signed = if dx < 0.0 { -w } else { w };
                        let sy = if dx.abs() < 0.001 {
                            0.0
                        } else {
                            (w_signed * dy) / dx
                        };
                        (w_signed, sy)
                    };

                    bend_points[0].x = node_x + sx;
                    bend_points[0].y = node_y + sy;
                }
            }

            edge_bend_points.insert((source.to_string(), target.to_string()), bend_points);
            // Use adjusted label position (shifted with composite offset if applicable)
            if let Some(lp) = label_pos {
                edge_label_positions.insert((source.to_string(), target.to_string()), lp);
            }
        }
    }

    // Title offset
    let title_offset = if !db.diagram_title.is_empty() {
        40.0
    } else {
        0.0
    };

    // Calculate diagram bounds from state positions, edge bend points, and labels
    // Initialize bounds from all positioned elements
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    // Include all state positions
    for &(x, y, w, h) in state_positions.values() {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    // Include composite state bounds (they may extend beyond individual state positions)
    for composite_id in &composite_ids {
        if let Some((cx, cy, cw, ch)) =
            calculate_composite_bounds(composite_id, db, &state_positions)
        {
            min_x = min_x.min(cx);
            min_y = min_y.min(cy);
            max_x = max_x.max(cx + cw);
            max_y = max_y.max(cy + ch);
        }
    }

    // Include edge bend points and labels
    for points in edge_bend_points.values() {
        for point in points {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
    }

    // Include edge label positions (estimate label size based on typical label dimensions)
    // Edge labels are positioned on edges which already fall within state bounds,
    // so we only need minimal padding for the label text itself (not 50px!)
    // Typical short labels (5 chars) at ~8px/char = ~40px, so half-width ~20px
    // But since edges are between states, labels shouldn't extend much past them
    let label_half_width = 15.0; // Conservative for most single-word labels
    let label_half_height = 10.0;
    for pos in edge_label_positions.values() {
        min_x = min_x.min(pos.x - label_half_width);
        min_y = min_y.min(pos.y - label_half_height);
        max_x = max_x.max(pos.x + label_half_width);
        max_y = max_y.max(pos.y + label_half_height);
    }

    // Handle empty diagram case
    if min_x == f64::MAX {
        min_x = 0.0;
        min_y = 0.0;
        max_x = 400.0;
        max_y = 200.0;
    }

    let bounds_x = min_x;
    let bounds_y = min_y;
    let max_width = (max_x - min_x) + margin * 2.0;
    let max_height = (max_y - min_y) + margin * 2.0 + title_offset;

    // ViewBox origin accounts for margin offset from content bounds
    let view_x = bounds_x - margin;
    let view_y = bounds_y - margin - title_offset;
    doc.set_size_with_origin(view_x, view_y, max_width, max_height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_state_css(&config.theme));
    }

    // Add arrow marker (uses theme colors)
    doc.add_defs(vec![create_arrow_marker(&config.theme)]);

    // Render title
    if !db.diagram_title.is_empty() {
        let title_elem = SvgElement::Text {
            x: max_width / 2.0,
            y: 25.0,
            content: db.diagram_title.clone(),
            attrs: Attrs::new()
                .with_attr("text-anchor", "middle")
                .with_class("state-title")
                .with_attr("font-size", "20")
                .with_attr("font-weight", "bold"),
        };
        doc.add_element(title_elem);
    }

    // Identify composite states (states that are parents of other states)
    let composite_states: std::collections::HashSet<&str> = states
        .values()
        .filter_map(|s| s.parent.as_deref())
        .collect();

    // Sort states for consistent ordering
    let mut sorted_states: Vec<_> = states.iter().collect();
    sorted_states.sort_by(|a, b| a.0.cmp(b.0));

    // Sort composite states by nesting depth (parents before children)
    // This ensures correct z-order: parent composites render first, children on top
    let mut sorted_composites: Vec<&str> = composite_states.iter().copied().collect();
    sorted_composites.sort_by(|a, b| {
        // Calculate depth for each composite
        let depth_a = {
            let mut depth = 0;
            let mut current = *a;
            while let Some(parent) = states.get(current).and_then(|s| s.parent.as_deref()) {
                if composite_states.contains(parent) {
                    depth += 1;
                }
                current = parent;
            }
            depth
        };
        let depth_b = {
            let mut depth = 0;
            let mut current = *b;
            while let Some(parent) = states.get(current).and_then(|s| s.parent.as_deref()) {
                if composite_states.contains(parent) {
                    depth += 1;
                }
                current = parent;
            }
            depth
        };
        // Sort by depth first (parents before children), then by name for consistency
        depth_a.cmp(&depth_b).then_with(|| a.cmp(b))
    });

    // First, render composite state containers (behind everything else)
    // Sorted by depth ensures parents render before children for correct z-order
    for composite_id in &sorted_composites {
        if let Some(composite_elem) = render_composite_state(
            composite_id,
            db,
            &state_positions,
            &composite_states,
            &level_layouts,
        ) {
            doc.add_element(composite_elem);
        }
    }

    // Render each non-composite state
    for (id, state) in &sorted_states {
        // Skip composite states - they're rendered as containers above
        if composite_states.contains(id.as_str()) {
            continue;
        }

        if let Some(&(x, y, width, height)) = state_positions.get(*id) {
            // Check if this [*] state is a start or end
            let state_info = start_end_states.get(id.as_str());
            let is_end_state = state_info.map(|info| !info.is_start).unwrap_or(false);

            let state_elem = render_state_node(
                state,
                x,
                y,
                width,
                height,
                start_end_radius,
                70.0, // fork_join_width (horizontal bar)
                10.0, // fork_join_height
                is_end_state,
                &config.theme,
            );
            doc.add_element(state_elem);

            // Render note if present
            if let Some(note) = &state.note {
                let note_x = match note.position {
                    NotePosition::LeftOf => x - 120.0,
                    NotePosition::RightOf => x + width + 20.0,
                };
                let note_elem = render_note(note_x, y, &note.text);
                doc.add_element(note_elem);
            }
        }
    }

    // Render transitions
    for relation in db.get_relations() {
        if let (Some(&(x1, y1, w1, h1)), Some(&(x2, y2, w2, h2))) = (
            state_positions.get(&relation.state1),
            state_positions.get(&relation.state2),
        ) {
            let state1 = states.get(&relation.state1);
            let state2 = states.get(&relation.state2);

            // Get bend points and label position from layout
            let edge_key = (relation.state1.clone(), relation.state2.clone());
            let bend_points = edge_bend_points.get(&edge_key);
            let label_position = edge_label_positions.get(&edge_key);

            let transition_elem = render_transition(
                x1,
                y1,
                x2,
                y2,
                w1,
                h1,
                w2,
                h2,
                start_end_radius,
                70.0, // fork_join_width (horizontal bar)
                10.0, // fork_join_height
                state1.map(|s| s.state_type),
                state2.map(|s| s.state_type),
                relation.description.as_deref(),
                bend_points,
                label_position,
            );
            doc.add_element(transition_elem);
        }
    }

    Ok(doc.to_string())
}

/// Render a composite state as a container box with a title
fn render_composite_state(
    composite_id: &str,
    db: &StateDb,
    state_positions: &HashMap<String, (f64, f64, f64, f64)>,
    composite_states: &std::collections::HashSet<&str>,
    level_layouts: &HashMap<String, LevelLayout>,
) -> Option<SvgElement> {
    // Find all child states (states whose parent is this composite)
    let states = db.get_states();

    // Check if this composite is nested inside another composite (for alternate styling)
    let is_nested = states
        .get(composite_id)
        .and_then(|s| s.parent.as_deref())
        .map(|parent_id| composite_states.contains(parent_id))
        .unwrap_or(false);
    let child_ids: Vec<&str> = states
        .iter()
        .filter(|(_, state)| state.parent.as_deref() == Some(composite_id))
        .map(|(id, _)| id.as_str())
        .collect();

    if child_ids.is_empty() {
        return None;
    }

    // Calculate bounding box from child positions
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for child_id in &child_ids {
        if let Some(&(x, y, w, h)) = state_positions.get(*child_id) {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + w);
            max_y = max_y.max(y + h);
        }
    }

    // Also include nested composite states' bounds
    // Use recursive calculation from state_positions to ensure consistency after shifts
    for child_id in &child_ids {
        // Check if this child is a composite (has children)
        let is_composite = states
            .values()
            .any(|s| s.parent.as_deref() == Some(*child_id));

        if is_composite {
            // Recursively calculate the nested composite's bounds
            if let Some((nested_x, nested_y, nested_w, nested_h)) =
                calculate_composite_bounds_recursive(child_id, db, state_positions)
            {
                min_x = min_x.min(nested_x);
                min_y = min_y.min(nested_y);
                max_x = max_x.max(nested_x + nested_w);
                max_y = max_y.max(nested_y + nested_h);
            }
        }
    }

    if min_x == f64::MAX {
        return None;
    }

    // Add padding around child states
    let padding = 12.0; // Balance between mermaid's 8px and visual spacing needs
    let title_height = 25.0;
    min_x -= padding;
    min_y -= padding + title_height;
    max_x += padding;
    max_y += padding;

    let content_width = max_x - min_x;
    let height = max_y - min_y;

    // Use the expanded width from level_layouts instead of the computed bounds.
    // This applies the expansion that was calculated during layout (both for leaf
    // composites at 1.5x and non-leaf composites at 1.35x).
    let width = if let Some(layout) = level_layouts.get(composite_id) {
        // The expanded layout width plus padding (which we've already subtracted from bounds)
        let expanded_total = layout.width + 2.0 * padding;
        // Center the content within the expanded width
        let width_expansion = (expanded_total - content_width) / 2.0;
        if width_expansion > 0.0 {
            min_x -= width_expansion;
            // Note: max_x is implicitly min_x + expanded_total, used via width
        }
        expanded_total
    } else {
        content_width
    };

    // Create the outer rect (border with fill matching reference)
    let outer_rect = SvgElement::Rect {
        x: min_x,
        y: min_y,
        width,
        height,
        rx: Some(5.0),
        ry: Some(5.0),
        attrs: Attrs::new().with_class("state-composite-outer"),
    };

    // Create the inner rect (white fill, or gray for nested composites)
    // The inner rect is inset horizontally so the outer rect's border shows through
    // on the left and right sides. It's also padded at the bottom.
    let stroke_width = 1.0;
    let bottom_padding = 4.0;
    let inner_class = if is_nested {
        "state-composite-inner-alt"
    } else {
        "state-composite-inner"
    };
    let inner_rect = SvgElement::Rect {
        x: min_x + stroke_width,
        y: min_y + title_height - 4.0, // Start below the title
        width: width - 2.0 * stroke_width,
        height: height - title_height + 4.0 - bottom_padding,
        rx: Some(0.0),
        ry: Some(0.0),
        attrs: Attrs::new().with_class(inner_class),
    };

    // Get the composite state's label (name or description)
    let label = states
        .get(composite_id)
        .and_then(|s| s.descriptions.first().cloned())
        .unwrap_or_else(|| composite_id.to_string());

    // Create the title label (centered horizontally, matching mermaid reference)
    let title = SvgElement::Text {
        x: min_x + width / 2.0, // Center horizontally
        y: min_y + 16.0,
        content: label,
        attrs: Attrs::new()
            .with_class("state-composite-label")
            .with_attr("font-size", "14")
            .with_attr("text-anchor", "middle"), // Center the text
    };

    // Create a divider path between title and content (use path instead of line for SVG consistency)
    let divider_y = min_y + title_height - 4.0;
    let divider = SvgElement::Path {
        d: format!(
            "M {} {} L {} {}",
            min_x,
            divider_y,
            min_x + width,
            divider_y
        ),
        attrs: Attrs::new().with_class("state-composite-divider"),
    };

    // Wrap in a group - outer first, then inner, then divider, then title on top
    Some(SvgElement::Group {
        children: vec![outer_rect, inner_rect, divider, title],
        attrs: Attrs::new()
            .with_class("composite-state")
            .with_id(&format!("composite-{}", composite_id)),
    })
}

/// Render a state node based on its type
#[allow(clippy::too_many_arguments)]
fn render_state_node(
    state: &State,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    start_end_radius: f64,
    fork_join_width: f64,
    fork_join_height: f64,
    is_end_state: bool,
    theme: &crate::render::svg::Theme,
) -> SvgElement {
    let mut children = Vec::new();

    match state.state_type {
        StateType::Start => {
            // Filled circle for start state (specialStateColor = lineColor)
            children.push(SvgElement::Circle {
                cx: x + width / 2.0,
                cy: y + height / 2.0,
                r: start_end_radius,
                attrs: Attrs::new()
                    .with_fill(&theme.line_color)
                    .with_class("state-start"),
            });
        }
        StateType::End => {
            render_end_state_bullseye(&mut children, x, y, width, height, start_end_radius, theme);
        }
        StateType::Fork | StateType::Join => {
            // Black bar for fork/join (specialStateColor = lineColor)
            // Use path instead of rect to match mermaid reference (rough.js generates paths)
            let bar_x = x + (width - fork_join_width) / 2.0;
            let bar_y = y + (height - fork_join_height) / 2.0;
            let bar_height = fork_join_height.min(height);
            children.push(SvgElement::Path {
                d: rounded_rect_path(bar_x, bar_y, fork_join_width, bar_height, 2.0, 2.0),
                attrs: Attrs::new()
                    .with_fill(&theme.line_color)
                    .with_class("state-fork-join"),
            });
        }
        StateType::Choice => {
            // Diamond for choice/decision
            let cx = x + width / 2.0;
            let cy = y + height / 2.0;
            let size = height / 2.0;

            children.push(SvgElement::Polygon {
                points: vec![
                    crate::layout::Point {
                        x: cx,
                        y: cy - size,
                    },
                    crate::layout::Point {
                        x: cx + size,
                        y: cy,
                    },
                    crate::layout::Point {
                        x: cx,
                        y: cy + size,
                    },
                    crate::layout::Point {
                        x: cx - size,
                        y: cy,
                    },
                ],
                attrs: Attrs::new()
                    .with_fill(&theme.primary_color)
                    .with_stroke(&theme.line_color)
                    .with_stroke_width(1.0)
                    .with_class("state-choice"),
            });
        }
        StateType::Divider => {
            // Horizontal path for divider (use path instead of line for SVG consistency)
            let divider_y = y + height / 2.0;
            children.push(SvgElement::Path {
                d: format!("M {} {} L {} {}", x, divider_y, x + width, divider_y),
                attrs: Attrs::new()
                    .with_stroke(&theme.line_color)
                    .with_stroke_width(2.0)
                    .with_stroke_dasharray("5,5")
                    .with_class("state-divider"),
            });
        }
        StateType::Default => {
            // Check if this is a start/end state (ID ends with _start or _end)
            let is_start_state = state.id.ends_with("_start");
            let is_end_state_by_id = state.id.ends_with("_end");
            if is_start_state {
                // Start state: filled circle (specialStateColor = lineColor)
                children.push(SvgElement::Circle {
                    cx: x + width / 2.0,
                    cy: y + height / 2.0,
                    r: start_end_radius,
                    attrs: Attrs::new()
                        .with_fill(&theme.line_color)
                        .with_class("state-start"),
                });
            } else if is_end_state || is_end_state_by_id {
                // End state: double circle (bullseye)
                render_end_state_bullseye(
                    &mut children,
                    x,
                    y,
                    width,
                    height,
                    start_end_radius,
                    theme,
                );
            } else {
                // Rounded rectangle for regular state (stateBkg + stateBorder)
                // Use path instead of rect to match mermaid reference output
                children.push(SvgElement::Path {
                    d: rounded_rect_path(x, y, width, height, 5.0, 5.0),
                    attrs: Attrs::new()
                        .with_fill(&theme.primary_color)
                        .with_stroke(&theme.primary_border_color)
                        .with_stroke_width(1.0)
                        .with_class("state-box"),
                });

                // State label
                let label = state.alias.as_ref().unwrap_or(&state.id);
                children.push(SvgElement::Text {
                    x: x + width / 2.0,
                    y: y + height / 2.0 + 5.0,
                    content: label.clone(),
                    attrs: Attrs::new()
                        .with_attr("text-anchor", "middle")
                        .with_class("state-label")
                        .with_attr("font-size", "16"),
                });

                // State descriptions
                if !state.descriptions.is_empty() {
                    let desc_y = y + height / 2.0 + 18.0;
                    for (i, desc) in state.descriptions.iter().enumerate() {
                        children.push(SvgElement::Text {
                            x: x + width / 2.0,
                            y: desc_y + (i as f64) * 14.0,
                            content: desc.clone(),
                            attrs: Attrs::new()
                                .with_attr("text-anchor", "middle")
                                .with_class("state-description")
                                .with_attr("font-size", "10"),
                        });
                    }
                }
            }
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new()
            .with_class("state-node")
            .with_id(&format!("state-{}", state.id)),
    }
}

/// Render a transition between two states
#[allow(clippy::too_many_arguments)]
fn render_transition(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    w1: f64,
    h1: f64,
    w2: f64,
    h2: f64,
    start_end_radius: f64,
    _fork_join_width: f64,
    _fork_join_height: f64,
    state1_type: Option<StateType>,
    state2_type: Option<StateType>,
    label: Option<&str>,
    bend_points: Option<&Vec<Point>>,
    label_position: Option<&Point>,
) -> SvgElement {
    let mut children = Vec::new();

    // Use bend points from layout if available, otherwise calculate connection points
    let (path_d, label_x, label_y) = if let Some(points) = bend_points {
        if !points.is_empty() {
            // Check if this edge involves a fork/join state
            // Fork/join edges need curves, so skip simplification (matching mermaid behavior)
            let is_fork_join_edge =
                matches!(state1_type, Some(StateType::Fork) | Some(StateType::Join))
                    || matches!(state2_type, Some(StateType::Fork) | Some(StateType::Join));

            // Use dagre's bend points to create a curved path
            // Skip simplification for fork/join edges to preserve the fan-out curve
            let curved_path = build_curved_path_with_options(points, !is_fork_join_edge);

            // Use layout-provided label position if available, otherwise calculate midpoint
            let (lx, ly) = if let Some(pos) = label_position {
                (pos.x, pos.y)
            } else {
                // Calculate label position at midpoint of the path
                let mid_idx = points.len() / 2;
                if points.len() > 1 && mid_idx > 0 {
                    let p1 = &points[mid_idx - 1];
                    let p2 = &points[mid_idx.min(points.len() - 1)];
                    ((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0)
                } else {
                    (points[0].x, points[0].y)
                }
            };

            (curved_path, lx, ly)
        } else {
            // Empty bend points - fallback to calculated path
            calculate_fallback_path(
                x1,
                y1,
                x2,
                y2,
                w1,
                h1,
                w2,
                h2,
                start_end_radius,
                state1_type,
                state2_type,
            )
        }
    } else {
        // No bend points - fallback to calculated path
        calculate_fallback_path(
            x1,
            y1,
            x2,
            y2,
            w1,
            h1,
            w2,
            h2,
            start_end_radius,
            state1_type,
            state2_type,
        )
    };

    // Transition path (curved) - colors from CSS via theme
    // Use stroke-width 1.0 to match mermaid's CSS default (.transition { stroke-width: 1; })
    // Note: mermaid's inline styles may override to 1.3, but CSS base is 1.0
    children.push(SvgElement::Path {
        d: path_d,
        attrs: Attrs::new()
            .with_stroke_width(1.0)
            .with_fill("none")
            .with_attr("marker-end", "url(#arrow)")
            .with_class("transition-path"),
    });

    // Transition label with background (matching flowchart edge label style)
    if let Some(text) = label {
        if !text.is_empty() {
            // Estimate text dimensions for background
            let font_size = 16.0;
            let char_width_ratio = 0.5; // Tighter estimate for proportional fonts
            let text_width = text.len() as f64 * font_size * char_width_ratio;
            let text_height = font_size * 1.1; // Tighter for SVG text
            let padding = 2.0;

            // Background as path (centered on label position)
            // Using path instead of rect to match mermaid reference (rough.js generates paths)
            let bg_x = label_x - text_width / 2.0 - padding;
            let bg_y = label_y - text_height / 2.0 - padding;
            let bg_w = text_width + padding * 2.0;
            let bg_h = text_height + padding * 2.0;
            children.push(SvgElement::Path {
                d: rounded_rect_path(bg_x, bg_y, bg_w, bg_h, 2.0, 2.0),
                attrs: Attrs::new().with_class("transition-label-bg"),
            });

            // Label text (centered with dominant-baseline)
            children.push(SvgElement::Text {
                x: label_x,
                y: label_y,
                content: text.to_string(),
                attrs: Attrs::new()
                    .with_attr("text-anchor", "middle")
                    .with_attr("dominant-baseline", "central")
                    .with_class("transition-label")
                    .with_attr("font-size", "16"),
            });
        }
    }

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("transition"),
    }
}

/// Calculate fallback path when no bend points are available
#[allow(clippy::too_many_arguments)]
fn calculate_fallback_path(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    w1: f64,
    h1: f64,
    w2: f64,
    h2: f64,
    start_end_radius: f64,
    state1_type: Option<StateType>,
    state2_type: Option<StateType>,
) -> (String, f64, f64) {
    // Calculate connection points based on state types
    let (start_x, start_y) = calculate_exit_point(
        x1,
        y1,
        w1,
        h1,
        x2 + w2 / 2.0,
        y2 + h2 / 2.0,
        state1_type,
        start_end_radius,
    );

    let (end_x, end_y) = calculate_entry_point(
        x2,
        y2,
        w2,
        h2,
        x1 + w1 / 2.0,
        y1 + h1 / 2.0,
        state2_type,
        start_end_radius,
    );

    // Create a curved path between the two points
    // Add a control point in the middle to create a slight curve
    let mid_x = (start_x + end_x) / 2.0;
    let mid_y = (start_y + end_y) / 2.0;

    // For simple two-point path, create a slight curve
    let points = vec![
        Point::new(start_x, start_y),
        Point::new(mid_x, mid_y),
        Point::new(end_x, end_y),
    ];

    let path = build_curved_path(&points);
    (path, mid_x, mid_y)
}

/// Calculate exit point from a state
#[allow(clippy::too_many_arguments)]
fn calculate_exit_point(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    target_x: f64,
    target_y: f64,
    state_type: Option<StateType>,
    start_end_radius: f64,
) -> (f64, f64) {
    let cx = x + width / 2.0;
    let cy = y + height / 2.0;

    match state_type {
        Some(StateType::Start) | Some(StateType::End) => {
            // Circle - calculate intersection
            let dx = target_x - cx;
            let dy = target_y - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 0.0 {
                (
                    cx + dx / dist * start_end_radius,
                    cy + dy / dist * start_end_radius,
                )
            } else {
                (cx + start_end_radius, cy)
            }
        }
        _ => {
            // Rectangle - calculate edge intersection
            let dx = target_x - cx;
            let dy = target_y - cy;

            if dx.abs() > dy.abs() {
                if dx > 0.0 {
                    (x + width, cy)
                } else {
                    (x, cy)
                }
            } else if dy > 0.0 {
                (cx, y + height)
            } else {
                (cx, y)
            }
        }
    }
}

/// Calculate entry point into a state
#[allow(clippy::too_many_arguments)]
fn calculate_entry_point(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    source_x: f64,
    source_y: f64,
    state_type: Option<StateType>,
    start_end_radius: f64,
) -> (f64, f64) {
    let cx = x + width / 2.0;
    let cy = y + height / 2.0;

    match state_type {
        Some(StateType::Start) | Some(StateType::End) => {
            // Circle - calculate intersection
            let dx = source_x - cx;
            let dy = source_y - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 0.0 {
                (
                    cx + dx / dist * start_end_radius,
                    cy + dy / dist * start_end_radius,
                )
            } else {
                (cx - start_end_radius, cy)
            }
        }
        _ => {
            // Rectangle - calculate edge intersection
            let dx = source_x - cx;
            let dy = source_y - cy;

            if dx.abs() > dy.abs() {
                if dx > 0.0 {
                    (x + width, cy)
                } else {
                    (x, cy)
                }
            } else if dy > 0.0 {
                (cx, y + height)
            } else {
                (cx, y)
            }
        }
    }
}

/// Render a note
fn render_note(x: f64, y: f64, text: &str) -> SvgElement {
    let note_width = 100.0;
    let note_height = 40.0;
    let fold_size = 8.0;

    let mut children = Vec::new();

    // Note box with folded corner
    let path = format!(
        "M {} {} L {} {} L {} {} L {} {} L {} {} Z",
        x,
        y,
        x + note_width - fold_size,
        y,
        x + note_width,
        y + fold_size,
        x + note_width,
        y + note_height,
        x,
        y + note_height
    );

    // Note box - colors from CSS via theme
    children.push(SvgElement::Path {
        d: path,
        attrs: Attrs::new().with_stroke_width(1.0).with_class("note-box"),
    });

    // Fold line - uses same stroke color as note-box
    let fold_path = format!(
        "M {} {} L {} {} L {} {}",
        x + note_width - fold_size,
        y,
        x + note_width - fold_size,
        y + fold_size,
        x + note_width,
        y + fold_size
    );

    children.push(SvgElement::Path {
        d: fold_path,
        attrs: Attrs::new()
            .with_fill("none")
            .with_stroke_width(1.0)
            .with_class("note-box"),
    });

    // Note text
    children.push(SvgElement::Text {
        x: x + note_width / 2.0,
        y: y + note_height / 2.0 + 4.0,
        content: text.to_string(),
        attrs: Attrs::new()
            .with_attr("text-anchor", "middle")
            .with_class("note-text")
            .with_attr("font-size", "16"),
    });

    SvgElement::Group {
        children,
        attrs: Attrs::new().with_class("note"),
    }
}

/// Create arrow marker (matches mermaid barbEnd marker)
/// Uses theme line_color for fill/stroke (SVG markers don't inherit CSS)
fn create_arrow_marker(theme: &crate::render::svg::Theme) -> SvgElement {
    SvgElement::Marker {
        id: "arrow".to_string(),
        view_box: "0 0 20 14".to_string(),
        ref_x: 19.0,
        ref_y: 7.0,
        marker_width: 20.0,
        marker_height: 14.0,
        orient: "auto".to_string(),
        marker_units: Some("userSpaceOnUse".to_string()),
        children: vec![SvgElement::Path {
            // Barbed arrow shape matching mermaid reference: M 19,7 L9,13 L14,7 L9,1 Z
            d: "M 19,7 L9,13 L14,7 L9,1 Z".to_string(),
            attrs: Attrs::new()
                .with_fill(&theme.line_color)
                .with_stroke(&theme.line_color)
                .with_stroke_width(1.0),
        }],
    }
}

/// Info about whether a [*] state is a start or end state
#[derive(Clone, Copy)]
struct StartEndInfo {
    is_start: bool,
}

/// Determine which [*] states are start vs end states based on their ID suffix
/// IDs follow mermaid's pattern: {parent}_start or {parent}_end
fn determine_start_end_states(db: &StateDb) -> HashMap<&str, StartEndInfo> {
    let mut result = HashMap::new();

    // Classify states based on ID suffix (mermaid-style naming)
    for id in db.get_states().keys() {
        if id.ends_with("_start") {
            result.insert(id.as_str(), StartEndInfo { is_start: true });
        } else if id.ends_with("_end") {
            result.insert(id.as_str(), StartEndInfo { is_start: false });
        }
    }

    result
}

/// Render end state bullseye (outer ring + filled inner circle)
/// Mermaid stateEnd.ts: outer=stroke only (lineColor), inner=filled (stateBorder)
fn render_end_state_bullseye(
    children: &mut Vec<SvgElement>,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    start_end_radius: f64,
    theme: &crate::render::svg::Theme,
) {
    // Outer circle as path: stroke only with primary_border_color (purple), no fill
    // Using path instead of circle to match mermaid reference (rough.js generates paths)
    let cx = x + width / 2.0;
    let cy = y + height / 2.0;
    children.push(SvgElement::Path {
        d: circle_path(cx, cy, start_end_radius),
        attrs: Attrs::new()
            .with_fill("none")
            .with_stroke(&theme.primary_border_color)
            .with_stroke_width(2.0)
            .with_class("state-end-outer"),
    });
    // Inner circle as path: filled with primary_border_color (creates the center dot)
    // Mermaid uses diameter ratio 5:14, so radius ratio ~0.36
    children.push(SvgElement::Path {
        d: circle_path(cx, cy, start_end_radius * 0.36),
        attrs: Attrs::new()
            .with_fill(&theme.primary_border_color)
            .with_stroke(&theme.primary_border_color)
            .with_stroke_width(2.0)
            .with_class("state-end-inner"),
    });
}

fn generate_state_css(theme: &crate::render::svg::Theme) -> String {
    format!(
        r#"
.state-title {{
  fill: {text_color};
}}

.state-box {{
  fill: {primary_color};
  stroke: {primary_border_color};
}}

.state-label {{
  fill: {text_color};
}}

.state-description {{
  fill: #666666;
}}

.state-start {{
  fill: {line_color};
}}

.state-end-outer {{
  fill: none;
  stroke: {primary_border_color};
  stroke-width: 2;
}}

.state-end-inner {{
  fill: {primary_border_color};
  stroke: {primary_border_color};
  stroke-width: 2;
}}

.state-fork-join {{
  fill: {line_color};
}}

.state-choice {{
  fill: {primary_color};
  stroke: {line_color};
}}

.state-divider {{
  stroke: {line_color};
  stroke-dasharray: 5, 5;
  fill: none;
}}

.transition-path {{
  stroke: {line_color};
  fill: none;
}}

.transition-label {{
  fill: {text_color};
}}

.transition-label-bg {{
  fill: {edge_label_background};
  stroke: none;
}}

.note-box {{
  fill: {note_bkg_color};
  stroke: {line_color};
}}

.note-text {{
  fill: {text_color};
}}

.state-composite-outer {{
  fill: {primary_color};
  stroke: {primary_border_color};
  stroke-width: 1px;
}}

.state-composite-inner {{
  fill: {background};
  stroke: none;
}}

.state-composite-inner-alt {{
  fill: #f0f0f0;
  stroke: none;
}}

.state-composite-label {{
  fill: {text_color};
  font-weight: bold;
}}

.state-composite-divider {{
  stroke: {primary_border_color};
  stroke-width: 1px;
  fill: none;
}}
"#,
        text_color = theme.primary_text_color,
        primary_color = theme.primary_color,
        primary_border_color = theme.primary_border_color,
        line_color = theme.line_color,
        background = theme.background,
        edge_label_background = theme.edge_label_background,
        note_bkg_color = theme.note_bkg_color,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagrams::state::parser::parse;
    use crate::layout::CharacterSizeEstimator;

    #[test]
    fn test_composite_state_has_parent_id_in_layout() {
        // Parse a simple composite state diagram
        let input = r#"stateDiagram-v2
    state Idle {
        [*] --> Ready
        Ready --> Processing
    }
"#;
        let db = parse(input).expect("Should parse");
        let size_estimator = CharacterSizeEstimator::default();
        let graph = db
            .to_layout_graph(&size_estimator)
            .expect("Should create layout graph");

        // Find the Ready and Processing nodes
        let ready_node = graph
            .nodes
            .iter()
            .find(|n| n.id == "Ready")
            .expect("Should have Ready node");
        let processing_node = graph
            .nodes
            .iter()
            .find(|n| n.id == "Processing")
            .expect("Should have Processing node");

        // They should have Idle as their parent
        assert_eq!(
            ready_node.parent_id.as_deref(),
            Some("Idle"),
            "Ready should have Idle as parent"
        );
        assert_eq!(
            processing_node.parent_id.as_deref(),
            Some("Idle"),
            "Processing should have Idle as parent"
        );

        // Idle should be marked as composite
        let idle_node = graph
            .nodes
            .iter()
            .find(|n| n.id == "Idle")
            .expect("Should have Idle node");
        assert_eq!(
            idle_node.metadata.get("is_group").map(String::as_str),
            Some("true"),
            "Idle should be marked as composite"
        );
    }

    #[test]
    fn test_composite_state_has_zero_initial_size() {
        // Composite states get zero initial dimensions - dagre expands them based on children
        // This matches flowchart subgraph behavior for proper compound graph layout
        let input = r#"stateDiagram-v2
    state Parent {
        [*] --> Child
    }
"#;
        let db = parse(input).expect("Should parse");
        let size_estimator = CharacterSizeEstimator::default();
        let graph = db
            .to_layout_graph(&size_estimator)
            .expect("Should create layout graph");

        let parent_node = graph
            .nodes
            .iter()
            .find(|n| n.id == "Parent")
            .expect("Should have Parent node");
        // Zero initial size allows compound layout to compute correct bounds
        assert_eq!(
            parent_node.width, 0.0,
            "Composite state should have zero initial width"
        );
        assert_eq!(
            parent_node.height, 0.0,
            "Composite state should have zero initial height"
        );
        // Should be marked as a group
        assert_eq!(
            parent_node.metadata.get("is_group").map(String::as_str),
            Some("true"),
            "Composite state should be marked as a group"
        );
    }

    #[test]
    fn test_nested_composite_states() {
        // Test multi-level nesting
        let input = r#"stateDiagram-v2
    state Outer {
        state Inner {
            [*] --> Deep
        }
    }
"#;
        let db = parse(input).expect("Should parse");
        let size_estimator = CharacterSizeEstimator::default();
        let graph = db
            .to_layout_graph(&size_estimator)
            .expect("Should create layout graph");

        let inner_node = graph
            .nodes
            .iter()
            .find(|n| n.id == "Inner")
            .expect("Should have Inner node");
        let deep_node = graph
            .nodes
            .iter()
            .find(|n| n.id == "Deep")
            .expect("Should have Deep node");

        // Inner should have Outer as parent
        assert_eq!(inner_node.parent_id.as_deref(), Some("Outer"));
        // Deep should have Inner as parent
        assert_eq!(deep_node.parent_id.as_deref(), Some("Inner"));
    }

    #[test]
    fn test_complex_nested_with_fork_join() {
        // This test case previously triggered a recursion overflow bug in network simplex
        // when there are:
        // 1. Multiple composite states
        // 2. Fork/join states
        // 3. 3-level nesting with transitions inside deepest composite
        let input = r#"stateDiagram-v2
    [*] --> Idle
    state Idle {
        [*] --> Ready
        Ready --> Active: Start Job
    }
    state fork_state <<fork>>
    Idle --> fork_state
    fork_state --> Validation
    fork_state --> B
    state join_state <<join>>
    Validation --> join_state
    B --> join_state
    join_state --> Processing
    state Processing {
        [*] --> Checking
        Checking --> Executing
        state Executing {
            [*] --> Init
            Init --> Done
        }
    }
"#;
        let db = parse(input).expect("Should parse");
        let size_estimator = CharacterSizeEstimator::default();
        let graph = db
            .to_layout_graph(&size_estimator)
            .expect("Should create layout graph");

        // Verify composite states are marked correctly
        let executing = graph.nodes.iter().find(|n| n.id == "Executing").unwrap();
        assert_eq!(
            executing.metadata.get("is_group").map(String::as_str),
            Some("true")
        );

        // This should not panic with recursion overflow
        let result = layout(graph);
        assert!(
            result.is_ok(),
            "Layout should succeed without recursion overflow"
        );

        // Check for invalid coordinates
        let layout_graph = result.unwrap();
        let mut invalid_coords = Vec::new();
        for node in &layout_graph.nodes {
            let x_invalid = node
                .x
                .map(|x| x.is_nan() || x.is_infinite())
                .unwrap_or(true);
            let y_invalid = node
                .y
                .map(|y| y.is_nan() || y.is_infinite())
                .unwrap_or(true);
            if x_invalid || y_invalid {
                invalid_coords.push((node.id.clone(), node.x, node.y));
            }
        }
        if !invalid_coords.is_empty() {
            eprintln!("Nodes with invalid coordinates:");
            for (id, x, y) in &invalid_coords {
                eprintln!("  {} -> x={:?}, y={:?}", id, x, y);
            }
        }
        assert!(
            invalid_coords.is_empty(),
            "All nodes should have valid coordinates"
        );
    }

    #[test]
    fn test_debug_compound_graph_structure() {
        // Debug test to understand compound graph structure
        let input = r#"stateDiagram-v2
    [*] --> Idle
    state Idle {
        [*] --> Ready
        Ready --> Active: Start Job
    }
    state fork_state <<fork>>
    state join_state <<join>>
    Idle --> fork_state
    fork_state --> Validation
    fork_state --> ResourceAlloc
    Validation --> join_state
    ResourceAlloc --> join_state
    join_state --> Processing
    state Processing {
        [*] --> Validating
        Validating --> Executing
        state Executing {
            [*] --> Init
            Init --> Done
        }
    }
"#;
        let db = parse(input).expect("Should parse");
        let size_estimator = CharacterSizeEstimator::default();
        let graph = db
            .to_layout_graph(&size_estimator)
            .expect("Should create layout graph");

        // Print structure
        eprintln!("\n=== Input Structure ===");
        for node in &graph.nodes {
            eprintln!(
                "  {} (w={}, h={}, parent={:?}, is_group={:?})",
                node.id,
                node.width,
                node.height,
                node.parent_id,
                node.metadata.get("is_group")
            );
        }

        // Print edges
        eprintln!("\n=== Edges ===");
        for edge in &graph.edges {
            eprintln!(
                "  {} -> {}",
                edge.source().unwrap_or("?"),
                edge.target().unwrap_or("?")
            );
        }

        // Run layout
        let result = layout(graph).expect("Layout should succeed");

        // Print final positions
        eprintln!("\n=== Final Positions ===");
        let mut sorted_nodes: Vec<_> = result.nodes.iter().collect();
        sorted_nodes.sort_by(|a, b| a.id.cmp(&b.id));
        for node in sorted_nodes {
            eprintln!(
                "  {} -> x={:?}, y={:?}, w={}, h={}",
                node.id, node.x, node.y, node.width, node.height
            );
        }
    }

    #[test]
    fn test_dagre_graph_compound_structure() {
        // Test that DagreGraph is correctly set up with compound structure
        use crate::layout::dagre::graph::DagreGraph;
        use crate::layout::dagre::graph::NodeLabel;

        let mut dg = DagreGraph::new();

        // Set up simple compound: Parent contains Child
        dg.set_node(
            "Parent",
            NodeLabel {
                width: 0.0,
                height: 0.0,
                ..Default::default()
            },
        );
        dg.set_node(
            "Child",
            NodeLabel {
                width: 50.0,
                height: 30.0,
                ..Default::default()
            },
        );
        dg.set_parent("Child", "Parent");

        eprintln!("is_compound: {}", dg.is_compound());
        eprintln!("Parent children: {:?}", dg.children("Parent"));
        eprintln!("Child parent: {:?}", dg.parent("Child"));

        assert!(dg.is_compound(), "Graph should be compound");
        assert!(dg.children("Parent").contains(&&"Child".to_string()));
        assert_eq!(dg.parent("Child"), Some(&"Parent".to_string()));
    }

    #[test]
    fn test_simple_compound_layout() {
        // Minimal test to debug compound graph layout
        let input = r#"stateDiagram-v2
    state Parent {
        [*] --> Child
        Child --> Done
    }
"#;
        let db = parse(input).expect("Should parse");
        let size_estimator = CharacterSizeEstimator::default();
        let graph = db
            .to_layout_graph(&size_estimator)
            .expect("Should create layout graph");

        // Print structure
        eprintln!("\n=== Input Structure ===");
        for node in &graph.nodes {
            eprintln!(
                "  {} (w={}, h={}, parent={:?}, is_group={:?})",
                node.id,
                node.width,
                node.height,
                node.parent_id,
                node.metadata.get("is_group")
            );
        }

        // Run layout
        let result = layout(graph).expect("Layout should succeed");

        // Print final positions
        eprintln!("\n=== Final Positions ===");
        let mut sorted_nodes: Vec<_> = result.nodes.iter().collect();
        sorted_nodes.sort_by(|a, b| a.id.cmp(&b.id));
        for node in sorted_nodes {
            eprintln!(
                "  {} -> x={:?}, y={:?}, w={}, h={}",
                node.id, node.x, node.y, node.width, node.height
            );
        }

        // Verify Parent has non-zero dimensions after layout
        let parent = result.nodes.iter().find(|n| n.id == "Parent").unwrap();
        assert!(
            parent.width > 0.0,
            "Parent compound should have width > 0 after layout, got {}",
            parent.width
        );
    }

    #[test]
    fn test_nested_compound_layout() {
        // Test nested composite state layout (state_complex2 pattern)
        // Processing contains Running (nested composite)
        let input = r#"stateDiagram-v2
[*] --> Idle

state Idle {
    [*] --> Ready
    Ready --> Processing: Start Job
}

state Processing {
    [*] --> Validating
    Validating --> Queued: Valid
    Validating --> Failed: Invalid
    Queued --> Running: Worker Available
    Running --> Completed: Success
    Running --> Failed: Error
    Running --> Paused: Pause Request

    state Running {
        [*] --> Initializing
        Initializing --> Executing
        Executing --> Finalizing
        Finalizing --> [*]
    }
}

state Paused {
    [*] --> WaitingResume
    WaitingResume --> Timeout: 1 hour
}

Paused --> Running: Resume
Paused --> Cancelled: Cancel Request
Timeout --> Cancelled

Completed --> Idle: Reset
Failed --> Idle: Retry
Cancelled --> Idle: Reset

Completed --> [*]
Cancelled --> [*]
"#;
        let db = parse(input).expect("Should parse");
        let size_estimator = CharacterSizeEstimator::default();

        // Test compound graph approach
        let graph = db
            .to_layout_graph(&size_estimator)
            .expect("Should create layout graph");

        eprintln!("\n=== Nested Compound Structure ===");
        for node in &graph.nodes {
            eprintln!(
                "  {} (w={:.1}, h={:.1}, parent={:?}, is_group={:?})",
                node.id,
                node.width,
                node.height,
                node.parent_id,
                node.metadata.get("is_group")
            );
        }
        eprintln!("Node count: {}", graph.nodes.len());
        eprintln!("Edge count: {}", graph.edges.len());

        // Run layout
        let result = layout(graph).expect("Layout should succeed");

        eprintln!("\n=== Final Positions ===");
        for node in &result.nodes {
            if let (Some(x), Some(y)) = (node.x, node.y) {
                eprintln!(
                    "  {} -> x={:.1}, y={:.1}, w={:.1}, h={:.1}",
                    node.id, x, y, node.width, node.height
                );
            }
        }

        // Verify composite states have non-zero dimensions
        let processing = result.nodes.iter().find(|n| n.id == "Processing").unwrap();
        assert!(
            processing.width > 0.0,
            "Processing compound should have width > 0 after layout, got {}",
            processing.width
        );

        let running = result.nodes.iter().find(|n| n.id == "Running").unwrap();
        assert!(
            running.width > 0.0,
            "Running nested compound should have width > 0 after layout, got {}",
            running.width
        );
    }

    #[test]
    fn test_edge_labels_within_viewbox() {
        // Edge labels at the sides of the diagram should not be cut off
        // This tests that the SVG viewBox includes all edge label positions
        let input = r#"stateDiagram-v2
    [*] --> Idle
    Idle --> Running: start
    Running --> Idle: stop
    Running --> Error: error
    Error --> Idle: reset
    Error --> [*]
"#;
        let db = parse(input).expect("Should parse");
        let config = crate::render::RenderConfig::default();
        let svg = render_state(&db, &config).expect("Should render");

        // Extract viewBox dimensions
        let viewbox_re = regex::Regex::new(r#"viewBox="([^"]+)""#).unwrap();
        let viewbox_cap = viewbox_re.captures(&svg).expect("Should have viewBox");
        let viewbox_parts: Vec<f64> = viewbox_cap[1]
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        let (vb_x, vb_y, vb_width, vb_height) = (
            viewbox_parts[0],
            viewbox_parts[1],
            viewbox_parts[2],
            viewbox_parts[3],
        );

        // Extract all text elements and their positions
        let text_re = regex::Regex::new(r#"<text[^>]*x="([^"]+)"[^>]*>([^<]+)</text>"#).unwrap();
        for cap in text_re.captures_iter(&svg) {
            let x: f64 = cap[1].parse().unwrap();
            let label = &cap[2];

            // Skip non-label text (like state names which are centered)
            if ["Idle", "Running", "Error"].contains(&label) {
                continue;
            }

            // Edge labels should be within the viewBox
            // Account for label width (approximate)
            let approx_width = label.len() as f64 * 9.6; // 16px * 0.6 char ratio
            let label_left = x - approx_width / 2.0;
            let label_right = x + approx_width / 2.0;

            // Check left edge
            assert!(
                label_left >= vb_x - 5.0, // 5px tolerance
                "Label '{}' at x={} (left edge ~{}) extends beyond viewBox left {} (viewBox: {} {} {} {})",
                label, x, label_left, vb_x, vb_x, vb_y, vb_width, vb_height
            );

            // Check right edge
            assert!(
                label_right <= vb_x + vb_width + 5.0, // 5px tolerance
                "Label '{}' at x={} (right edge ~{}) extends beyond viewBox right {} (viewBox: {} {} {} {})",
                label, x, label_right, vb_x + vb_width, vb_x, vb_y, vb_width, vb_height
            );
        }
    }

    #[test]
    fn test_composite_state_bounds_include_all_children() {
        // Verify that the Idle composite state includes Active (all children)
        let input = r#"stateDiagram-v2
    [*] --> Idle
    state Idle {
        [*] --> Ready
        Ready --> Active: Start Job
    }
    Idle --> fork_state
    state fork_state <<fork>>
"#;
        let db = parse(input).expect("Should parse");

        // Check that Active has Idle as parent in the parsed data
        let active_state = db.get_state("Active");
        assert!(active_state.is_some(), "Active state should exist");
        assert_eq!(
            active_state.unwrap().parent.as_deref(),
            Some("Idle"),
            "Active should have Idle as parent"
        );

        let config = crate::render::RenderConfig::default();
        let svg = render_state(&db, &config).expect("Should render");

        // Extract Idle composite state bounds from SVG
        // Look for: <g class="composite-state" id="composite-Idle">
        //           <rect x="..." y="..." width="..." height="..."
        let idle_re = regex::Regex::new(
            r#"id="composite-Idle"[^>]*>\s*<rect[^>]*x="([^"]+)"[^>]*y="([^"]+)"[^>]*width="([^"]+)"[^>]*height="([^"]+)""#
        ).unwrap();

        let idle_cap = idle_re
            .captures(&svg)
            .expect("Should find Idle composite rect");
        let idle_x: f64 = idle_cap[1].parse().unwrap();
        let idle_y: f64 = idle_cap[2].parse().unwrap();
        let idle_w: f64 = idle_cap[3].parse().unwrap();
        let idle_h: f64 = idle_cap[4].parse().unwrap();

        eprintln!(
            "Idle bounds: x={}, y={}, w={}, h={}",
            idle_x, idle_y, idle_w, idle_h
        );

        // Extract Active state bounds from path element
        // Path format from rounded_rect_path: M {x+rx} {y} H {right-rx} A rx ry 0 0 1 {right} {y+ry} V {bottom-ry} A ...
        // We extract: first x coord (x+rx), y coord, right-rx (from H), and bottom-ry (from V)
        let active_re = regex::Regex::new(
            r#"id="state-Active"[^>]*>\s*<path[^>]*d="M ([0-9.]+) ([0-9.]+) H ([0-9.]+) A [0-9.]+ [0-9.]+ 0 0 1 [0-9.]+ [0-9.]+ V ([0-9.]+)"#
        ).unwrap();

        let active_cap = active_re
            .captures(&svg)
            .expect("Should find Active state path");
        let active_x_plus_rx: f64 = active_cap[1].parse().unwrap();
        let active_y: f64 = active_cap[2].parse().unwrap();
        let active_right_minus_rx: f64 = active_cap[3].parse().unwrap();
        let active_bottom_minus_ry: f64 = active_cap[4].parse().unwrap();
        let rx = 5.0; // We know this from the rounded_rect_path call
        let ry = 5.0;
        let active_x = active_x_plus_rx - rx;
        let active_w = active_right_minus_rx + rx - active_x;
        let active_h = active_bottom_minus_ry + ry - active_y;

        eprintln!(
            "Active bounds: x={}, y={}, w={}, h={}",
            active_x, active_y, active_w, active_h
        );

        // Verify Active is fully contained within Idle
        assert!(
            active_x >= idle_x,
            "Active left edge ({}) should be >= Idle left edge ({})",
            active_x,
            idle_x
        );
        assert!(
            active_y >= idle_y,
            "Active top edge ({}) should be >= Idle top edge ({})",
            active_y,
            idle_y
        );
        assert!(
            active_x + active_w <= idle_x + idle_w,
            "Active right edge ({}) should be <= Idle right edge ({})",
            active_x + active_w,
            idle_x + idle_w
        );
        assert!(
            active_y + active_h <= idle_y + idle_h,
            "Active bottom edge ({}) should be <= Idle bottom edge ({})",
            active_y + active_h,
            idle_y + idle_h
        );
    }

    #[test]
    fn test_nested_composite_has_alternate_background() {
        // Nested composite states (like Executing inside Processing) should have
        // gray alternate background (#f0f0f0) instead of white
        let input = r#"stateDiagram-v2
    state Processing {
        [*] --> Validating
        Validating --> Executing
        state Executing {
            [*] --> Init
            Init --> Done
        }
    }
"#;
        let db = parse(input).expect("Should parse");
        let config = crate::render::RenderConfig::default();
        let svg = render_state(&db, &config).expect("Should render");

        // Executing is nested inside Processing, so it should use alternate inner class
        assert!(
            svg.contains("state-composite-inner-alt"),
            "Nested composite state should use alternate inner class"
        );

        // Verify the CSS includes the alternate background color
        assert!(
            svg.contains("#f0f0f0"),
            "CSS should include alternate background color #f0f0f0"
        );
    }

    #[test]
    fn test_nested_composite_renders_after_parent() {
        // Nested composite states must be rendered AFTER their parent in SVG
        // to appear on top (correct z-order)
        let input = r#"stateDiagram-v2
    state Processing {
        [*] --> Validating
        Validating --> Executing
        state Executing {
            [*] --> Init
            Init --> Done
        }
    }
"#;
        let db = parse(input).expect("Should parse");
        let config = crate::render::RenderConfig::default();
        let svg = render_state(&db, &config).expect("Should render");

        // Find positions of composite states in SVG
        let processing_pos = svg
            .find("id=\"composite-Processing\"")
            .expect("Processing composite should exist");
        let executing_pos = svg
            .find("id=\"composite-Executing\"")
            .expect("Executing composite should exist");

        // Executing must come AFTER Processing for correct z-order
        assert!(
            executing_pos > processing_pos,
            "Nested composite (Executing) must be rendered after parent (Processing) for correct z-order. \
             Processing at {}, Executing at {}",
            processing_pos,
            executing_pos
        );
    }

    #[test]
    fn test_fork_edge_order_preserved_in_layout_graph() {
        // Test that fork edges are added to the layout graph in definition order.
        // This is critical for ensuring the first fork target appears on the left.
        let input = r#"stateDiagram-v2
    direction TB
    [*] --> Start
    state fork_state <<fork>>
    Start --> fork_state
    fork_state --> Validation
    fork_state --> ResourceAlloc
"#;
        let db = parse(input).expect("Should parse");

        // Check relations order in StateDb
        let relations = db.get_relations();
        let fork_edges: Vec<_> = relations
            .iter()
            .filter(|r| r.state1 == "fork_state")
            .collect();

        assert_eq!(fork_edges.len(), 2, "Should have 2 edges from fork_state");
        assert_eq!(
            fork_edges[0].state2, "Validation",
            "First fork edge should target Validation"
        );
        assert_eq!(
            fork_edges[1].state2, "ResourceAlloc",
            "Second fork edge should target ResourceAlloc"
        );

        // Check layout graph edge order
        let size_estimator = CharacterSizeEstimator::default();
        let graph = db
            .to_layout_graph(&size_estimator)
            .expect("Should create layout graph");

        // Find edges from fork_state
        let fork_edges: Vec<_> = graph
            .edges
            .iter()
            .filter(|e| e.source() == Some("fork_state"))
            .collect();

        assert_eq!(
            fork_edges.len(),
            2,
            "Should have 2 edges from fork_state in layout graph"
        );
        assert_eq!(
            fork_edges[0].target(),
            Some("Validation"),
            "First layout edge should target Validation"
        );
        assert_eq!(
            fork_edges[1].target(),
            Some("ResourceAlloc"),
            "Second layout edge should target ResourceAlloc"
        );
    }

    #[test]
    fn test_fork_order_after_dagre_layout() {
        // Test that fork targets maintain correct order after full dagre layout.
        // This is the actual rendering test - Validation should be on the LEFT of ResourceAlloc.
        let input = r#"stateDiagram-v2
    direction TB
    [*] --> Start
    state fork_state <<fork>>
    Start --> fork_state
    fork_state --> Validation
    fork_state --> ResourceAlloc
    state join_state <<join>>
    Validation --> join_state
    ResourceAlloc --> join_state
    join_state --> [*]
"#;
        let db = parse(input).expect("Should parse");
        let size_estimator = CharacterSizeEstimator::default();
        let graph = db
            .to_layout_graph(&size_estimator)
            .expect("Should create layout graph");

        // Debug: print layout graph edge order
        eprintln!("LayoutGraph edges from fork_state:");
        for (i, edge) in graph.edges.iter().enumerate() {
            if edge.source() == Some("fork_state") {
                eprintln!(
                    "  {}: {} -> {}",
                    i,
                    edge.source().unwrap_or("?"),
                    edge.target().unwrap_or("?")
                );
            }
        }

        // Run layout
        let result = layout(graph).expect("Layout should succeed");

        // Find Validation and ResourceAlloc nodes
        let validation = result
            .nodes
            .iter()
            .find(|n| n.id == "Validation")
            .expect("Should have Validation node");
        let resource = result
            .nodes
            .iter()
            .find(|n| n.id == "ResourceAlloc")
            .expect("Should have ResourceAlloc node");

        let val_x = validation.x.expect("Validation should have x position");
        let res_x = resource.x.expect("ResourceAlloc should have x position");

        eprintln!(
            "After layout: Validation.x={}, ResourceAlloc.x={}",
            val_x, res_x
        );

        // Validation (first edge target) should have smaller x (be on LEFT) in TB layout
        assert!(
            val_x < res_x,
            "Validation (first fork target) should be LEFT of ResourceAlloc. \
             Validation.x={}, ResourceAlloc.x={}",
            val_x,
            res_x
        );
    }

    #[test]
    fn test_transition_stroke_width_matches_mermaid() {
        // Mermaid's CSS uses .transition { stroke-width: 1; }
        // Our rendered SVG should match this for visual parity
        let input = r#"stateDiagram-v2
    [*] --> Idle
    Idle --> Running
"#;
        let db = parse(input).expect("Should parse");
        let config = crate::render::RenderConfig::default();
        let svg = render_state(&db, &config).expect("Should render");

        // The SVG should contain transition paths with stroke-width="1"
        // to match mermaid's CSS default
        assert!(
            svg.contains(r#"stroke-width="1""#),
            "Transition paths should have stroke-width=\"1\" to match mermaid CSS. \
             Found SVG content: {}",
            &svg[..svg.len().min(500)]
        );

        // Should not contain the old stroke-width value of 0.7 for transition paths
        // Note: Other elements may have different stroke-widths (e.g., end state uses 2)
        let transition_paths: Vec<&str> = svg
            .lines()
            .filter(|l| l.contains("transition-path"))
            .collect();
        for path in &transition_paths {
            assert!(
                !path.contains(r#"stroke-width="0.7""#),
                "Transition paths should not use stroke-width=\"0.7\" (old value). Found: {}",
                path
            );
        }
    }

    #[test]
    fn test_nested_composite_width_matches_mermaid_reference() {
        // Mermaid reference for state_complex diagram shows Processing composite at 191px wide.
        // Selkie was rendering at 129px (33% too narrow).
        // This test ensures nested composites have appropriate width expansion.
        // Reference: eval-report comparison showing "reference 191x673, selkie 129x534"
        let input = r#"stateDiagram-v2
    direction TB

    [*] --> Idle

    state Idle {
        [*] --> Ready
        Ready --> Active: Start Job
    }

    state fork_state <<fork>>
    state join_state <<join>>

    Idle --> fork_state
    fork_state --> Validation
    fork_state --> ResourceAlloc

    Validation --> join_state
    ResourceAlloc --> join_state
    join_state --> Processing

    state Processing {
        [*] --> Validating
        Validating --> Executing

        state Executing {
            [*] --> Init
            Init --> Done
        }
    }
"#;
        let db = parse(input).expect("Should parse");
        let config = crate::render::RenderConfig::default();
        let svg = render_state(&db, &config).expect("Should render");

        // Extract Processing composite width from rendered SVG
        // The composite outer rect pattern: <rect ... class="state-composite-outer"
        // Look for the Processing composite group and its dimensions
        let processing_re =
            regex::Regex::new(r#"id="composite-Processing"[^>]*>\s*<rect[^>]*width="([^"]+)""#)
                .unwrap();

        let processing_cap = processing_re
            .captures(&svg)
            .expect("Should find Processing composite rect");
        let processing_width: f64 = processing_cap[1].parse().unwrap();

        eprintln!("Processing composite width: {}", processing_width);

        // Mermaid reference width is 191px. We should be within 15% (at least 162px).
        // This is a significant improvement from the current 129px.
        let min_acceptable_width = 162.0; // 191 * 0.85
        assert!(
            processing_width >= min_acceptable_width,
            "Processing composite width ({:.1}px) should be at least {:.1}px \
             (within 15% of mermaid reference 191px). \
             Current implementation renders composites too narrow.",
            processing_width,
            min_acceptable_width
        );
    }
}
