//! Architecture diagram adapter for layout

use std::collections::{HashMap, HashSet, VecDeque};

use crate::diagrams::architecture::{
    ArchitectureAlignment, ArchitectureDb, ArchitectureDirection, ArchitectureEdge,
};
use crate::error::Result;
use crate::layout::{
    LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode, LayoutOptions, NodeShape, Padding,
    SizeEstimator, ToLayoutGraph,
};

pub const ARCH_ICON_SIZE: f64 = 80.0;
pub const ARCH_PADDING: f64 = 40.0;
pub const ARCH_FONT_SIZE: f64 = 16.0;
pub const ARCH_LABEL_HEIGHT: f64 = ARCH_FONT_SIZE + 1.0;
pub const ARCH_GROUP_ICON_SCALE: f64 = 0.75;
pub const ARCH_GROUP_PADDING_EXTRA: f64 = ARCH_PADDING / 16.0;
pub const ARCH_GROUP_PADDING: f64 = ARCH_PADDING + ARCH_GROUP_PADDING_EXTRA;
pub const ARCH_NODE_SPACING: f64 = ARCH_ICON_SIZE * 2.5;
pub const ARCH_EDGE_GROUP_LABEL_SHIFT: f64 = 18.0;
const ARCH_USE_FORCE_LAYOUT: bool = false;

impl ToLayoutGraph for ArchitectureDb {
    fn to_layout_graph(&self, _size_estimator: &dyn SizeEstimator) -> Result<LayoutGraph> {
        let mut graph = LayoutGraph::new("architecture");

        graph.options = LayoutOptions {
            direction: self.preferred_direction(),
            node_spacing: ARCH_NODE_SPACING,
            layer_spacing: ARCH_NODE_SPACING,
            padding: Padding::uniform(ARCH_PADDING),
            ranker: crate::layout::LayoutRanker::default(),
        };

        let mut groups = self.get_groups();
        groups.sort_by_key(|g| g.id.as_str());
        for group in groups {
            let mut node = LayoutNode::new(&group.id, 0.0, 0.0)
                .with_shape(NodeShape::Rectangle)
                .with_padding(Padding::uniform(ARCH_GROUP_PADDING));
            if let Some(parent) = group.parent.as_deref() {
                node = node.with_parent(parent);
            }
            if let Some(label) = group.title.as_deref().or(Some(group.id.as_str())) {
                node.metadata.insert("label".to_string(), label.to_string());
            }
            node.metadata
                .insert("is_group".to_string(), "true".to_string());
            graph.add_node(node);
        }

        let mut services = self.get_services();
        services.sort_by_key(|s| s.id.as_str());
        for service in services {
            let mut node = LayoutNode::new(&service.id, ARCH_ICON_SIZE, ARCH_ICON_SIZE)
                .with_shape(NodeShape::Rectangle);
            if let Some(parent) = service.parent.as_deref() {
                node = node.with_parent(parent);
            }
            node.metadata
                .insert("node_type".to_string(), "service".to_string());
            graph.add_node(node);
        }

        let mut junctions = self.get_junctions();
        junctions.sort_by_key(|j| j.id.as_str());
        for junction in junctions {
            let mut node = LayoutNode::new(&junction.id, ARCH_ICON_SIZE, ARCH_ICON_SIZE)
                .with_shape(NodeShape::Rectangle);
            if let Some(parent) = junction.parent.as_deref() {
                node = node.with_parent(parent);
            }
            node.metadata
                .insert("node_type".to_string(), "junction".to_string());
            graph.add_node(node);
        }

        for (idx, edge) in self.get_edges().iter().enumerate() {
            let edge_id = format!("edge-{}-{}-{}", idx, edge.lhs_id, edge.rhs_id);
            let mut layout_edge = LayoutEdge::new(&edge_id, &edge.lhs_id, &edge.rhs_id);
            if let Some(title) = edge.title.as_deref() {
                layout_edge = layout_edge.with_label(title);
            }
            layout_edge
                .metadata
                .insert("lhs_dir".to_string(), edge.lhs_dir.short_name().to_string());
            layout_edge
                .metadata
                .insert("rhs_dir".to_string(), edge.rhs_dir.short_name().to_string());
            layout_edge
                .metadata
                .insert("lhs_into".to_string(), edge.lhs_into.to_string());
            layout_edge
                .metadata
                .insert("rhs_into".to_string(), edge.rhs_into.to_string());
            layout_edge
                .metadata
                .insert("lhs_group".to_string(), edge.lhs_group.to_string());
            layout_edge
                .metadata
                .insert("rhs_group".to_string(), edge.rhs_group.to_string());
            graph.add_edge(layout_edge);
        }

        Ok(graph)
    }

    fn preferred_direction(&self) -> LayoutDirection {
        let mut horizontal = 0;
        let mut vertical = 0;
        for edge in self.get_edges() {
            match architecture_alignment(edge) {
                ArchitectureAlignment::Horizontal => horizontal += 1,
                ArchitectureAlignment::Vertical => vertical += 1,
                ArchitectureAlignment::Bend => {}
            }
        }
        if horizontal > vertical {
            LayoutDirection::LeftToRight
        } else {
            LayoutDirection::TopToBottom
        }
    }
}

fn architecture_alignment(edge: &ArchitectureEdge) -> ArchitectureAlignment {
    crate::diagrams::architecture::get_direction_alignment(edge.lhs_dir, edge.rhs_dir)
}

pub fn layout_architecture(
    db: &ArchitectureDb,
    size_estimator: &dyn SizeEstimator,
) -> Result<LayoutGraph> {
    let mut graph = db.to_layout_graph(size_estimator)?;
    apply_architecture_layout(db, &mut graph);
    Ok(graph)
}

fn apply_architecture_layout(db: &ArchitectureDb, graph: &mut LayoutGraph) {
    let node_ids: Vec<String> = db
        .get_services()
        .into_iter()
        .map(|s| s.id.clone())
        .chain(db.get_junctions().into_iter().map(|j| j.id.clone()))
        .collect();

    let adj = build_adjacency(db, &node_ids);
    let node_root_groups = build_node_root_group_map(db);
    let spatial_maps = build_spatial_maps(&adj, &node_ids, &node_root_groups);

    let mut positions: HashMap<String, (f64, f64)> = HashMap::new();
    let mut max_x = 0.0;

    // Initialize positions from grid layout (warm start)
    for spatial_map in spatial_maps {
        let mut component_positions: Vec<(String, f64, f64)> = Vec::new();
        let mut local_min_x = f64::MAX;
        let mut local_max_x = f64::MIN;

        for (id, (grid_x, grid_y)) in spatial_map {
            let cx = (grid_x as f64) * ARCH_NODE_SPACING;
            let cy = (-grid_y as f64) * ARCH_NODE_SPACING;
            local_min_x = local_min_x.min(cx);
            local_max_x = local_max_x.max(cx);
            component_positions.push((id, cx, cy));
        }

        let shift_x = if local_min_x == f64::MAX {
            0.0
        } else {
            max_x - local_min_x
        };

        for (id, cx, cy) in component_positions {
            positions.insert(id, (cx + shift_x, cy));
        }

        if local_min_x != f64::MAX {
            max_x = max_x + (local_max_x - local_min_x) + ARCH_NODE_SPACING * 2.0;
        }
    }

    if ARCH_USE_FORCE_LAYOUT {
        let mut sim = Simulation::new(db, positions.clone(), &adj);
        sim.run(300);
        positions = sim.get_positions();
    }
    apply_overlap_jitter(&mut positions, &node_root_groups);

    let half_icon = ARCH_ICON_SIZE / 2.0;
    for (id, (cx, cy)) in positions.iter() {
        if let Some(node) = graph.get_node_mut(id) {
            node.x = Some(*cx - half_icon);
            node.y = Some(*cy - half_icon);
        }
    }

    separate_group_overlaps(db, graph, &mut positions);
    separate_group_node_overlaps(db, graph, &mut positions);

    let group_bounds = compute_group_bounds(db, graph);
    for (group_id, bounds) in group_bounds {
        if let Some(node) = graph.get_node_mut(&group_id) {
            node.x = Some(bounds.x);
            node.y = Some(bounds.y);
            node.width = bounds.width;
            node.height = bounds.height;
        }
    }

    graph.compute_bounds();
}

fn apply_overlap_jitter(
    positions: &mut HashMap<String, (f64, f64)>,
    node_root_groups: &HashMap<String, Option<String>>,
) {
    let mut counts: HashMap<(i64, i64, Option<String>), usize> = HashMap::new();
    let mut ids: Vec<String> = positions.keys().cloned().collect();
    ids.sort();

    for id in ids {
        if let Some((x, y)) = positions.get_mut(&id) {
            let root = node_root_groups.get(&id).cloned().unwrap_or(None);
            let key = (x.round() as i64, y.round() as i64, root);
            let count = counts.entry(key).or_insert(0);
            if *count > 0 {
                let offset = ARCH_ICON_SIZE * 0.25 * (*count as f64);
                *x += offset;
            }
            *count += 1;
        }
    }
}

fn separate_group_overlaps(
    db: &ArchitectureDb,
    graph: &mut LayoutGraph,
    positions: &mut HashMap<String, (f64, f64)>,
) {
    let mut group_children: HashMap<String, Vec<String>> = HashMap::new();
    for group in db.get_groups() {
        if let Some(parent) = group.parent.as_deref() {
            group_children
                .entry(parent.to_string())
                .or_default()
                .push(group.id.clone());
        }
    }

    let mut direct_nodes: HashMap<String, Vec<String>> = HashMap::new();
    for service in db.get_services() {
        if let Some(parent) = service.parent.as_deref() {
            direct_nodes
                .entry(parent.to_string())
                .or_default()
                .push(service.id.clone());
        }
    }
    for junction in db.get_junctions() {
        if let Some(parent) = junction.parent.as_deref() {
            direct_nodes
                .entry(parent.to_string())
                .or_default()
                .push(junction.id.clone());
        }
    }

    let mut group_nodes: HashMap<String, Vec<String>> = HashMap::new();
    for group in db.get_groups() {
        group_nodes.insert(
            group.id.clone(),
            collect_group_nodes(&group.id, &group_children, &direct_nodes),
        );
    }

    let mut parent_map: HashMap<String, Option<String>> = HashMap::new();
    for group in db.get_groups() {
        parent_map.insert(group.id.clone(), group.parent.clone());
    }

    let group_preferences = build_group_preferences(db);

    let mut group_ids: Vec<String> = db
        .get_groups()
        .iter()
        .filter(|g| g.parent.is_none())
        .map(|g| g.id.clone())
        .collect();
    group_ids.sort();
    if group_ids.len() < 2 {
        return;
    }

    let max_iterations = 6;
    for _ in 0..max_iterations {
        let group_bounds = compute_group_bounds(db, graph);
        let mut shifted = false;

        for i in 0..group_ids.len() {
            for j in (i + 1)..group_ids.len() {
                let gid_a = &group_ids[i];
                let gid_b = &group_ids[j];
                if groups_related(gid_a, gid_b, &parent_map) {
                    continue;
                }
                let (Some(a), Some(b)) = (group_bounds.get(gid_a), group_bounds.get(gid_b)) else {
                    continue;
                };

                let overlap_x = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
                let overlap_y = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
                if overlap_x <= 0.0 || overlap_y <= 0.0 {
                    continue;
                }

                let center_ax = a.x + a.width / 2.0;
                let center_ay = a.y + a.height / 2.0;
                let center_bx = b.x + b.width / 2.0;
                let center_by = b.y + b.height / 2.0;
                let dx = center_bx - center_ax;
                let dy = center_by - center_ay;

                let (shift_x, shift_y, shift_group) = if let Some(pref) = group_preferences
                    .get(&(gid_a.clone(), gid_b.clone()))
                    .map(|pref| (gid_a.as_str(), gid_b.as_str(), *pref))
                    .or_else(|| {
                        group_preferences
                            .get(&(gid_b.clone(), gid_a.clone()))
                            .map(|pref| (gid_b.as_str(), gid_a.as_str(), *pref))
                    }) {
                    let (_lhs, rhs, pref) = pref;
                    let shift_group = rhs;
                    let (moving, fixed) = if shift_group == gid_a.as_str() {
                        (a, b)
                    } else {
                        (b, a)
                    };
                    let (shift_x, shift_y) =
                        compute_group_separation(moving, fixed, pref.axis, pref.dir);
                    (shift_x, shift_y, shift_group)
                } else if dx.abs() >= dy.abs() {
                    if center_ax <= center_bx {
                        let (shift_x, shift_y) =
                            compute_group_separation(b, a, GroupAxis::Horizontal, 1);
                        (shift_x, shift_y, gid_b.as_str())
                    } else {
                        let (shift_x, shift_y) =
                            compute_group_separation(a, b, GroupAxis::Horizontal, 1);
                        (shift_x, shift_y, gid_a.as_str())
                    }
                } else if center_ay <= center_by {
                    let (shift_x, shift_y) = compute_group_separation(b, a, GroupAxis::Vertical, 1);
                    (shift_x, shift_y, gid_b.as_str())
                } else {
                    let (shift_x, shift_y) = compute_group_separation(a, b, GroupAxis::Vertical, 1);
                    (shift_x, shift_y, gid_a.as_str())
                };

                if shift_x != 0.0 || shift_y != 0.0 {
                    shift_group_nodes(
                        shift_group,
                        shift_x,
                        shift_y,
                        positions,
                        graph,
                        &group_nodes,
                    );
                    shifted = true;
                }
            }
        }

        if !shifted {
            break;
        }
    }
}

fn separate_group_node_overlaps(
    db: &ArchitectureDb,
    graph: &mut LayoutGraph,
    positions: &mut HashMap<String, (f64, f64)>,
) {
    let node_groups = build_node_group_map(db);
    let group_ids: Vec<String> = db.get_groups().iter().map(|g| g.id.clone()).collect();
    if group_ids.is_empty() {
        return;
    }

    let max_iterations = 6;
    for _ in 0..max_iterations {
        let group_bounds = compute_group_bounds(db, graph);
        let mut shifted = false;

        for group_id in &group_ids {
            let Some(bounds) = group_bounds.get(group_id) else {
                continue;
            };
            let parent = db
                .get_groups()
                .iter()
                .find(|g| &g.id == group_id)
                .and_then(|g| g.parent.clone());

            for (node_id, node_parent) in &node_groups {
                if node_parent.as_deref() == Some(group_id.as_str()) {
                    continue;
                }
                if node_parent.as_deref() != parent.as_deref() {
                    continue;
                }

                let Some(node) = graph.get_node(node_id) else {
                    continue;
                };
                let (Some(nx), Some(ny)) = (node.x, node.y) else {
                    continue;
                };

                let node_min_x = nx;
                let node_max_x = nx + node.width;
                let node_min_y = ny;
                let node_max_y = ny + node.height;

                let overlap_x =
                    (node_max_x).min(bounds.x + bounds.width) - node_min_x.max(bounds.x);
                let overlap_y =
                    (node_max_y).min(bounds.y + bounds.height) - node_min_y.max(bounds.y);
                if overlap_x <= 0.0 || overlap_y <= 0.0 {
                    continue;
                }

                let node_center_x = node_min_x + node.width / 2.0;
                let node_center_y = node_min_y + node.height / 2.0;
                let group_center_x = bounds.x + bounds.width / 2.0;
                let group_center_y = bounds.y + bounds.height / 2.0;
                let dx = node_center_x - group_center_x;
                let dy = node_center_y - group_center_y;

                let (shift_x, shift_y) = if dx.abs() >= dy.abs() {
                    let delta = overlap_x + ARCH_GROUP_PADDING;
                    (if dx >= 0.0 { delta } else { -delta }, 0.0)
                } else {
                    let delta = overlap_y + ARCH_GROUP_PADDING;
                    (0.0, if dy >= 0.0 { delta } else { -delta })
                };

                if shift_x != 0.0 || shift_y != 0.0 {
                    if let Some((x, y)) = positions.get_mut(node_id) {
                        *x += shift_x;
                        *y += shift_y;
                    }
                    if let Some(node) = graph.get_node_mut(node_id) {
                        if let Some(x) = node.x {
                            node.x = Some(x + shift_x);
                        }
                        if let Some(y) = node.y {
                            node.y = Some(y + shift_y);
                        }
                    }
                    shifted = true;
                }
            }
        }

        if !shifted {
            break;
        }
    }
}

#[derive(Clone, Copy)]
enum GroupAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
struct GroupPreference {
    axis: GroupAxis,
    dir: i32,
}

fn compute_group_separation(
    moving: &GroupBounds,
    fixed: &GroupBounds,
    axis: GroupAxis,
    dir: i32,
) -> (f64, f64) {
    match axis {
        GroupAxis::Horizontal => {
            let shift = if dir >= 0 {
                (fixed.x + fixed.width) - moving.x + ARCH_GROUP_PADDING
            } else {
                fixed.x - (moving.x + moving.width) - ARCH_GROUP_PADDING
            };
            (shift, 0.0)
        }
        GroupAxis::Vertical => {
            let shift = if dir >= 0 {
                (fixed.y + fixed.height) - moving.y + ARCH_GROUP_PADDING
            } else {
                fixed.y - (moving.y + moving.height) - ARCH_GROUP_PADDING
            };
            (0.0, shift)
        }
    }
}

fn build_group_preferences(db: &ArchitectureDb) -> HashMap<(String, String), GroupPreference> {
    let node_groups = build_node_group_map(db);
    let mut prefs: HashMap<(String, String), GroupPreference> = HashMap::new();

    for edge in db.get_edges() {
        let lhs_group = node_groups.get(&edge.lhs_id).and_then(|g| g.as_deref());
        let rhs_group = node_groups.get(&edge.rhs_id).and_then(|g| g.as_deref());
        let (Some(lhs_group), Some(rhs_group)) = (lhs_group, rhs_group) else {
            continue;
        };
        if lhs_group == rhs_group {
            continue;
        }

        let alignment = architecture_alignment(edge);
        let (axis, dir) = match alignment {
            ArchitectureAlignment::Horizontal => {
                let dir = if edge.lhs_dir == ArchitectureDirection::Right {
                    1
                } else {
                    -1
                };
                (GroupAxis::Horizontal, dir)
            }
            ArchitectureAlignment::Vertical => {
                let dir = if edge.lhs_dir == ArchitectureDirection::Bottom {
                    1
                } else {
                    -1
                };
                (GroupAxis::Vertical, dir)
            }
            ArchitectureAlignment::Bend => continue,
        };

        prefs
            .entry((lhs_group.to_string(), rhs_group.to_string()))
            .or_insert(GroupPreference { axis, dir });
    }

    prefs
}

fn shift_group_nodes(
    group_id: &str,
    dx: f64,
    dy: f64,
    positions: &mut HashMap<String, (f64, f64)>,
    graph: &mut LayoutGraph,
    group_nodes: &HashMap<String, Vec<String>>,
) {
    let Some(nodes) = group_nodes.get(group_id) else {
        return;
    };

    for node_id in nodes {
        if let Some((x, y)) = positions.get_mut(node_id) {
            *x += dx;
            *y += dy;
        }
        if let Some(node) = graph.get_node_mut(node_id) {
            if let Some(x) = node.x {
                node.x = Some(x + dx);
            }
            if let Some(y) = node.y {
                node.y = Some(y + dy);
            }
        }
    }
}

fn groups_related(
    group_a: &str,
    group_b: &str,
    parent_map: &HashMap<String, Option<String>>,
) -> bool {
    if group_a == group_b {
        return true;
    }
    let mut curr = Some(group_a.to_string());
    while let Some(c) = curr {
        if c == group_b {
            return true;
        }
        curr = parent_map.get(&c).and_then(|p| p.clone());
    }
    let mut curr = Some(group_b.to_string());
    while let Some(c) = curr {
        if c == group_a {
            return true;
        }
        curr = parent_map.get(&c).and_then(|p| p.clone());
    }
    false
}

// --- Simulation Logic ---

const REPULSION_FORCE: f64 = 1_000_000.0;
const SPRING_STIFFNESS: f64 = 0.8;
const GROUP_GRAVITY: f64 = 0.02;
const GROUP_EXCLUSION_FORCE: f64 = 2_000.0; // Reduced to prevent explosion
const MAX_FORCE: f64 = 1_000.0; // Cap forces
const DAMPING: f64 = 0.8;
const DT: f64 = 0.5;

struct NodeState {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

struct Constraint {
    target: String, // from source to target
    dx: f64,
    dy: f64,
}

struct Simulation {
    nodes: HashMap<String, NodeState>,
    edges: HashMap<String, Vec<Constraint>>, // source -> [constraints]
    groups: HashMap<String, Vec<String>>,    // group_id -> [node_ids]
    node_to_group: HashMap<String, String>,  // node_id -> group_id
    group_parents: HashMap<String, String>,  // group_id -> parent_group_id
}

impl Simulation {
    fn new(
        db: &ArchitectureDb,
        initial_positions: HashMap<String, (f64, f64)>,
        adj: &HashMap<String, Vec<(ArchitectureDirectionPair, String, f64)>>,
    ) -> Self {
        let mut nodes = HashMap::new();
        for (id, (x, y)) in initial_positions {
            nodes.insert(
                id,
                NodeState {
                    x,
                    y,
                    vx: 0.0,
                    vy: 0.0,
                },
            );
        }

        let mut edges = HashMap::new();
        for (src, neighbors) in adj {
            for (pair, dst, distance) in neighbors {
                let mut dx = 0.0;
                let mut dy = 0.0;
                let dist = (*distance) * ARCH_NODE_SPACING;

                if pair.source == ArchitectureDirection::Top {
                    dy = -dist;
                } else if pair.source == ArchitectureDirection::Bottom {
                    dy = dist;
                } else if pair.source == ArchitectureDirection::Left {
                    dx = -dist;
                } else if pair.source == ArchitectureDirection::Right {
                    dx = dist;
                }

                edges
                    .entry(src.clone())
                    .or_insert(Vec::new())
                    .push(Constraint {
                        target: dst.clone(),
                        dx,
                        dy,
                    });
            }
        }

        // Build group maps
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        let mut node_to_group: HashMap<String, String> = HashMap::new();
        let mut group_parents: HashMap<String, String> = HashMap::new();

        let node_groups = build_node_group_map(db);
        for (node, group_opt) in node_groups {
            if let Some(group) = group_opt {
                groups.entry(group.clone()).or_default().push(node.clone());
                node_to_group.insert(node, group);
            }
        }

        for group in db.get_groups() {
            if let Some(parent) = &group.parent {
                group_parents.insert(group.id.clone(), parent.clone());
            }
        }

        Self {
            nodes,
            edges,
            groups,
            node_to_group,
            group_parents,
        }
    }

    fn run(&mut self, iterations: usize) {
        let keys: Vec<String> = self.nodes.keys().cloned().collect();

        for _ in 0..iterations {
            let mut forces: HashMap<String, (f64, f64)> = HashMap::new();

            // 1. Repulsion (All pairs)
            for i in 0..keys.len() {
                for j in (i + 1)..keys.len() {
                    let k1 = &keys[i];
                    let k2 = &keys[j];
                    let n1 = &self.nodes[k1];
                    let n2 = &self.nodes[k2];

                    let dx = n1.x - n2.x;
                    let dy = n1.y - n2.y;
                    let dist_sq = (dx * dx + dy * dy).max(1.0);
                    let dist = dist_sq.sqrt();

                    // Repulsion
                    let f = (REPULSION_FORCE / dist_sq).min(MAX_FORCE);
                    let fx = (dx / dist) * f;
                    let fy = (dy / dist) * f;

                    let f1 = forces.entry(k1.clone()).or_insert((0.0, 0.0));
                    f1.0 += fx;
                    f1.1 += fy;

                    let f2 = forces.entry(k2.clone()).or_insert((0.0, 0.0));
                    f2.0 -= fx;
                    f2.1 -= fy;
                }
            }

            // 2. Edge Constraints (Springs)
            for (src, constraints) in &self.edges {
                let n_src = &self.nodes[src];
                for c in constraints {
                    if let Some(n_tgt) = self.nodes.get(&c.target) {
                        let target_x = n_src.x + c.dx;
                        let target_y = n_src.y + c.dy;

                        let dx = n_tgt.x - target_x;
                        let dy = n_tgt.y - target_y;

                        let fx = (-SPRING_STIFFNESS * dx).clamp(-MAX_FORCE, MAX_FORCE);
                        let fy = (-SPRING_STIFFNESS * dy).clamp(-MAX_FORCE, MAX_FORCE);

                        // Apply to target
                        let f_tgt = forces.entry(c.target.clone()).or_insert((0.0, 0.0));
                        f_tgt.0 += fx;
                        f_tgt.1 += fy;

                        // Apply opposite to source
                        let f_src = forces.entry(src.clone()).or_insert((0.0, 0.0));
                        f_src.0 -= fx;
                        f_src.1 -= fy;
                    }
                }
            }

            // 3. Group Gravity (Keep groups compact)
            for members in self.groups.values() {
                if members.is_empty() {
                    continue;
                }
                // Calculate centroid
                let mut sum_x = 0.0;
                let mut sum_y = 0.0;
                for m in members {
                    if let Some(n) = self.nodes.get(m) {
                        sum_x += n.x;
                        sum_y += n.y;
                    }
                }
                let cx = sum_x / members.len() as f64;
                let cy = sum_y / members.len() as f64;

                for m in members {
                    if let Some(n) = self.nodes.get(m) {
                        let dx = cx - n.x;
                        let dy = cy - n.y;
                        let f = forces.entry(m.clone()).or_insert((0.0, 0.0));
                        f.0 += (dx * GROUP_GRAVITY).clamp(-MAX_FORCE, MAX_FORCE);
                        f.1 += (dy * GROUP_GRAVITY).clamp(-MAX_FORCE, MAX_FORCE);
                    }
                }
            }

            // 4. Group Exclusion (Prevent overlapping groups)
            // Calculate group bounds
            let mut group_bounds = HashMap::new();
            for (gid, members) in &self.groups {
                if members.is_empty() {
                    continue;
                }
                let mut min_x = f64::MAX;
                let mut max_x = f64::MIN;
                let mut min_y = f64::MAX;
                let mut max_y = f64::MIN;
                for m in members {
                    if let Some(n) = self.nodes.get(m) {
                        min_x = min_x.min(n.x);
                        max_x = max_x.max(n.x);
                        min_y = min_y.min(n.y);
                        max_y = max_y.max(n.y);
                    }
                }
                // Add padding for exclusion zone
                let padding = ARCH_ICON_SIZE * 0.75;
                group_bounds.insert(
                    gid,
                    (
                        min_x - padding,
                        min_y - padding,
                        max_x + padding,
                        max_y + padding,
                    ),
                );
            }

            for (nid, node) in &self.nodes {
                let my_group = self.node_to_group.get(nid).map(|s| s.as_str());

                for (gid, (min_x, min_y, max_x, max_y)) in &group_bounds {
                    // Skip if related (same group or ancestry)
                    if self.are_groups_related(my_group, Some(gid)) {
                        continue;
                    }

                    // Check if node is inside foreign group bounds
                    if node.x > *min_x && node.x < *max_x && node.y > *min_y && node.y < *max_y {
                        // Push out
                        let cx = (min_x + max_x) / 2.0;
                        let cy = (min_y + max_y) / 2.0;
                        let mut dx = node.x - cx;
                        let mut dy = node.y - cy;

                        // Avoid zero vector
                        if dx.abs() < 0.1 {
                            dx = 1.0;
                        }
                        if dy.abs() < 0.1 {
                            dy = 1.0;
                        }

                        let dist = (dx * dx + dy * dy).sqrt();
                        let f = GROUP_EXCLUSION_FORCE.min(MAX_FORCE);
                        let fx = (dx / dist) * f;
                        let fy = (dy / dist) * f;

                        let f_node = forces.entry(nid.clone()).or_insert((0.0, 0.0));
                        f_node.0 += fx;
                        f_node.1 += fy;
                    }
                }
            }

            // Update positions
            for (id, node) in self.nodes.iter_mut() {
                if let Some((fx, fy)) = forces.get(id) {
                    node.vx = (node.vx + fx * DT) * DAMPING;
                    node.vy = (node.vy + fy * DT) * DAMPING;
                    node.x += node.vx * DT;
                    node.y += node.vy * DT;
                }
            }
        }
    }

    fn are_groups_related(&self, g1: Option<&str>, g2: Option<&str>) -> bool {
        match (g1, g2) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                if a == b {
                    return true;
                }
                // a ancestor of b?
                let mut curr = Some(b.to_string());
                while let Some(c) = curr {
                    if c == a {
                        return true;
                    }
                    curr = self.group_parents.get(&c).cloned();
                }
                // b ancestor of a?
                let mut curr = Some(a.to_string());
                while let Some(c) = curr {
                    if c == b {
                        return true;
                    }
                    curr = self.group_parents.get(&c).cloned();
                }
                false
            }
            _ => false,
        }
    }

    fn get_positions(&self) -> HashMap<String, (f64, f64)> {
        self.nodes
            .iter()
            .map(|(k, v)| (k.clone(), (v.x, v.y)))
            .collect()
    }
}

fn build_adjacency(
    db: &ArchitectureDb,
    node_ids: &[String],
) -> HashMap<String, Vec<(ArchitectureDirectionPair, String, f64)>> {
    let mut adj: HashMap<String, Vec<(ArchitectureDirectionPair, String, f64)>> = HashMap::new();
    for id in node_ids {
        adj.insert(id.clone(), Vec::new());
    }

    for edge in db.get_edges() {
        let distance = 1.0;

        if let Some(pair) = ArchitectureDirectionPair::new(edge.lhs_dir, edge.rhs_dir) {
            let entry = adj.entry(edge.lhs_id.clone()).or_default();
            if let Some(existing) = entry.iter_mut().find(|(p, _, _)| *p == pair) {
                *existing = (pair, edge.rhs_id.clone(), distance);
            } else {
                entry.push((pair, edge.rhs_id.clone(), distance));
            }
        }
        if let Some(pair) = ArchitectureDirectionPair::new(edge.rhs_dir, edge.lhs_dir) {
            let entry = adj.entry(edge.rhs_id.clone()).or_default();
            if let Some(existing) = entry.iter_mut().find(|(p, _, _)| *p == pair) {
                *existing = (pair, edge.lhs_id.clone(), distance);
            } else {
                entry.push((pair, edge.lhs_id.clone(), distance));
            }
        }
    }

    adj
}

fn build_spatial_maps(
    adj: &HashMap<String, Vec<(ArchitectureDirectionPair, String, f64)>>,
    node_ids: &[String],
    node_root_groups: &HashMap<String, Option<String>>,
) -> Vec<HashMap<String, (i32, i32)>> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut maps = Vec::new();

    for id in node_ids {
        if visited.contains(id) {
            continue;
        }

        let mut spatial_map: HashMap<String, (i32, i32)> = HashMap::new();
        let mut insertion_order: Vec<String> = Vec::new();
        let mut placement_pairs: HashMap<String, ArchitectureDirectionPair> = HashMap::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        spatial_map.insert(id.clone(), (0, 0));
        insertion_order.push(id.clone());
        queue.push_back(id.clone());

        while let Some(curr) = queue.pop_front() {
            if visited.contains(&curr) {
                continue;
            }
            visited.insert(curr.clone());
            let (x, y) = spatial_map.get(&curr).copied().unwrap_or((0, 0));

            if let Some(neighbors) = adj.get(&curr) {
                for (pair, neighbor, distance) in neighbors {
                    if visited.contains(neighbor) {
                        continue;
                    }

                    if spatial_map.contains_key(neighbor) {
                        continue;
                    }

                    let next_pos = pair.shift_position(x, y, distance.round() as i32);
                    spatial_map.insert(neighbor.clone(), next_pos);
                    insertion_order.push(neighbor.clone());
                    placement_pairs.insert(neighbor.clone(), *pair);
                    queue.push_back(neighbor.clone());
                }
            }
        }

        resolve_spatial_collisions(
            &mut spatial_map,
            &insertion_order,
            &placement_pairs,
            node_root_groups,
        );
        maps.push(spatial_map);
    }

    maps
}

const MAX_COLLISION_RADIUS: i32 = 6;

fn resolve_spatial_collisions(
    spatial_map: &mut HashMap<String, (i32, i32)>,
    insertion_order: &[String],
    placement_pairs: &HashMap<String, ArchitectureDirectionPair>,
    node_root_groups: &HashMap<String, Option<String>>,
) {
    let mut occupied_by_root: HashMap<Option<String>, HashSet<(i32, i32)>> = HashMap::new();
    for (id, pos) in spatial_map.iter() {
        let root = node_root_groups.get(id).cloned().unwrap_or(None);
        occupied_by_root.entry(root).or_default().insert(*pos);
    }
    let mut coord_nodes: HashMap<(i32, i32), Vec<String>> = HashMap::new();

    for id in insertion_order {
        if let Some(pos) = spatial_map.get(id) {
            coord_nodes.entry(*pos).or_default().push(id.clone());
        }
    }

    for (pos, nodes) in coord_nodes {
        let mut nodes_by_root: HashMap<Option<String>, Vec<String>> = HashMap::new();
        for node_id in nodes {
            let root = node_root_groups.get(&node_id).cloned().unwrap_or(None);
            nodes_by_root.entry(root).or_default().push(node_id);
        }

        for (root, nodes) in nodes_by_root {
            if nodes.len() < 2 {
                continue;
            }

            let Some((keep, relocate)) = nodes.split_last() else {
                continue;
            };
            let _ = keep;

            let occupied = occupied_by_root.entry(root).or_default();
            for node_id in relocate {
                let pair = placement_pairs.get(node_id);
                let new_pos = resolve_collision(occupied, pos, pair);
                if let Some(entry) = spatial_map.get_mut(node_id) {
                    *entry = new_pos;
                }
                occupied.insert(new_pos);
            }
        }
    }
}

fn resolve_collision(
    occupied: &HashSet<(i32, i32)>,
    start: (i32, i32),
    pair: Option<&ArchitectureDirectionPair>,
) -> (i32, i32) {
    let primary = pair
        .map(|p| opposite_dir(p.source))
        .unwrap_or(ArchitectureDirection::Left);
    let (primary_dx, primary_dy) = primary_axis_step(primary);

    for step in 1..=MAX_COLLISION_RADIUS {
        let candidate = (start.0 + primary_dx * step, start.1 + primary_dy * step);
        if !occupied.contains(&candidate) {
            return candidate;
        }
    }

    let dirs = preferred_search_dirs(primary);

    let mut queue: VecDeque<((i32, i32), i32)> = VecDeque::new();
    let mut seen: HashSet<(i32, i32)> = HashSet::new();

    queue.push_back((start, 0));
    seen.insert(start);

    while let Some(((x, y), dist)) = queue.pop_front() {
        if dist >= MAX_COLLISION_RADIUS {
            continue;
        }

        for (dx, dy) in dirs {
            let next = (x + dx, y + dy);
            if !seen.insert(next) {
                continue;
            }
            if !occupied.contains(&next) {
                return next;
            }
            queue.push_back((next, dist + 1));
        }
    }

    start
}

fn opposite_dir(dir: ArchitectureDirection) -> ArchitectureDirection {
    match dir {
        ArchitectureDirection::Left => ArchitectureDirection::Right,
        ArchitectureDirection::Right => ArchitectureDirection::Left,
        ArchitectureDirection::Top => ArchitectureDirection::Bottom,
        ArchitectureDirection::Bottom => ArchitectureDirection::Top,
    }
}

fn preferred_search_dirs(dir: ArchitectureDirection) -> [(i32, i32); 4] {
    match dir {
        ArchitectureDirection::Left => [(-1, 0), (0, -1), (0, 1), (1, 0)],
        ArchitectureDirection::Right => [(1, 0), (0, -1), (0, 1), (-1, 0)],
        ArchitectureDirection::Top => [(0, 1), (-1, 0), (1, 0), (0, -1)],
        ArchitectureDirection::Bottom => [(0, -1), (-1, 0), (1, 0), (0, 1)],
    }
}

fn primary_axis_step(dir: ArchitectureDirection) -> (i32, i32) {
    match dir {
        ArchitectureDirection::Left => (-1, 0),
        ArchitectureDirection::Right => (1, 0),
        ArchitectureDirection::Top => (0, 1),
        ArchitectureDirection::Bottom => (0, -1),
    }
}

#[derive(Debug, Clone, Copy)]
struct GroupBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn compute_group_bounds(db: &ArchitectureDb, graph: &LayoutGraph) -> HashMap<String, GroupBounds> {
    let mut group_children: HashMap<String, Vec<String>> = HashMap::new();
    for group in db.get_groups() {
        if let Some(parent) = group.parent.as_deref() {
            group_children
                .entry(parent.to_string())
                .or_default()
                .push(group.id.clone());
        }
    }

    let mut direct_nodes: HashMap<String, Vec<String>> = HashMap::new();
    for service in db.get_services() {
        if let Some(parent) = service.parent.as_deref() {
            direct_nodes
                .entry(parent.to_string())
                .or_default()
                .push(service.id.clone());
        }
    }
    for junction in db.get_junctions() {
        if let Some(parent) = junction.parent.as_deref() {
            direct_nodes
                .entry(parent.to_string())
                .or_default()
                .push(junction.id.clone());
        }
    }

    let label_heights: HashMap<String, f64> = db
        .get_services()
        .into_iter()
        .filter(|service| service.title.is_some())
        .map(|service| (service.id.clone(), ARCH_LABEL_HEIGHT))
        .collect();

    let mut bounds_map = HashMap::new();
    for group in db.get_groups() {
        if bounds_map.contains_key(&group.id) {
            continue;
        }

        compute_group_bounds_for(
            &group.id,
            graph,
            &group_children,
            &direct_nodes,
            &label_heights,
            &mut bounds_map,
        );
    }

    bounds_map
}

fn compute_group_bounds_for(
    group_id: &str,
    graph: &LayoutGraph,
    group_children: &HashMap<String, Vec<String>>,
    direct_nodes: &HashMap<String, Vec<String>>,
    label_heights: &HashMap<String, f64>,
    bounds_map: &mut HashMap<String, GroupBounds>,
) -> Option<GroupBounds> {
    if let Some(bounds) = bounds_map.get(group_id) {
        return Some(*bounds);
    }

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    if let Some(nodes) = direct_nodes.get(group_id) {
        for node_id in nodes {
            let Some(node) = graph.get_node(node_id) else {
                continue;
            };
            let (Some(x), Some(y)) = (node.x, node.y) else {
                continue;
            };
            let mut height = node.height;
            if let Some(label_height) = label_heights.get(node_id) {
                height += label_height;
            }
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + node.width);
            max_y = max_y.max(y + height);
        }
    }

    if let Some(children) = group_children.get(group_id) {
        for child_id in children {
            if let Some(child_bounds) = compute_group_bounds_for(
                child_id,
                graph,
                group_children,
                direct_nodes,
                label_heights,
                bounds_map,
            ) {
                min_x = min_x.min(child_bounds.x);
                min_y = min_y.min(child_bounds.y);
                max_x = max_x.max(child_bounds.x + child_bounds.width);
                max_y = max_y.max(child_bounds.y + child_bounds.height);
            }
        }
    }

    if min_x == f64::MAX {
        return None;
    }

    let rect_x = min_x - ARCH_GROUP_PADDING;
    let rect_y = min_y - ARCH_GROUP_PADDING;
    let rect_w = (max_x - min_x) + ARCH_GROUP_PADDING * 2.0;
    let rect_h = (max_y - min_y) + ARCH_GROUP_PADDING * 2.0;

    let bounds = GroupBounds {
        x: rect_x,
        y: rect_y,
        width: rect_w,
        height: rect_h,
    };
    bounds_map.insert(group_id.to_string(), bounds);
    Some(bounds)
}

fn collect_group_nodes(
    group_id: &str,
    group_children: &HashMap<String, Vec<String>>,
    direct_nodes: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut nodes = direct_nodes.get(group_id).cloned().unwrap_or_default();
    if let Some(children) = group_children.get(group_id) {
        for child_id in children {
            nodes.extend(collect_group_nodes(child_id, group_children, direct_nodes));
        }
    }
    nodes
}

fn build_node_group_map(db: &ArchitectureDb) -> HashMap<String, Option<String>> {
    let mut map = HashMap::new();
    for service in db.get_services() {
        map.insert(service.id.clone(), service.parent.clone());
    }
    for junction in db.get_junctions() {
        map.insert(junction.id.clone(), junction.parent.clone());
    }
    map
}

fn build_node_root_group_map(db: &ArchitectureDb) -> HashMap<String, Option<String>> {
    let mut parent_map: HashMap<String, Option<String>> = HashMap::new();
    for group in db.get_groups() {
        parent_map.insert(group.id.clone(), group.parent.clone());
    }

    let mut root_cache: HashMap<String, String> = HashMap::new();
    let mut resolve_root = |group_id: &str| -> String {
        if let Some(root) = root_cache.get(group_id) {
            return root.clone();
        }
        let mut current = group_id;
        let mut path: Vec<String> = Vec::new();
        while let Some(Some(parent)) = parent_map.get(current) {
            path.push(current.to_string());
            current = parent;
        }
        let root = current.to_string();
        for id in path {
            root_cache.insert(id, root.clone());
        }
        root
    };

    let mut map = HashMap::new();
    for service in db.get_services() {
        let root = service.parent.as_deref().map(&mut resolve_root);
        map.insert(service.id.clone(), root);
    }
    for junction in db.get_junctions() {
        let root = junction.parent.as_deref().map(&mut resolve_root);
        map.insert(junction.id.clone(), root);
    }
    map
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArchitectureDirectionPair {
    source: ArchitectureDirection,
    target: ArchitectureDirection,
}

impl ArchitectureDirectionPair {
    fn new(source: ArchitectureDirection, target: ArchitectureDirection) -> Option<Self> {
        if source == target {
            None
        } else {
            Some(Self { source, target })
        }
    }

    fn shift_position(&self, x: i32, y: i32, distance: i32) -> (i32, i32) {
        let source = self.source;
        let target = self.target;
        if source.is_x() {
            let dx = if source == ArchitectureDirection::Left {
                -distance
            } else {
                distance
            };
            if target.is_y() {
                let dy = if target == ArchitectureDirection::Top {
                    1
                } else {
                    -1
                };
                (x + dx, y + dy)
            } else {
                (x + dx, y)
            }
        } else {
            let dy = if source == ArchitectureDirection::Top {
                distance
            } else {
                -distance
            };
            if target.is_x() {
                let dx = if target == ArchitectureDirection::Left {
                    1
                } else {
                    -1
                };
                (x + dx, y + dy)
            } else {
                (x, y + dy)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagrams::architecture::{
        ArchitectureDirection, ArchitectureGroup, ArchitectureService,
    };
    use crate::layout::CharacterSizeEstimator;

    #[test]
    fn test_architecture_to_layout_graph() {
        let mut db = ArchitectureDb::new();
        db.add_group(ArchitectureGroup::new("api".to_string()).with_title("API"))
            .unwrap();
        db.add_service(
            ArchitectureService::new("db".to_string())
                .with_title("Database")
                .with_parent("api"),
        )
        .unwrap();
        db.add_service(ArchitectureService::new("server".to_string()).with_title("Server"))
            .unwrap();
        db.add_edge(ArchitectureEdge::new(
            "db".to_string(),
            ArchitectureDirection::Left,
            "server".to_string(),
            ArchitectureDirection::Right,
        ))
        .unwrap();

        let estimator = CharacterSizeEstimator::default();
        let graph = db.to_layout_graph(&estimator).unwrap();

        let db_node = graph.get_node("db").unwrap();
        assert_eq!(db_node.parent_id.as_deref(), Some("api"));
        assert_eq!(graph.edges.len(), 1);
        assert!(graph.get_node("api").is_some());
    }

    #[test]
    fn test_architecture_layout_positions() {
        let mut db = ArchitectureDb::new();
        db.add_group(ArchitectureGroup::new("api".to_string()).with_title("API"))
            .unwrap();
        db.add_service(
            ArchitectureService::new("db".to_string())
                .with_title("Database")
                .with_parent("api"),
        )
        .unwrap();
        db.add_service(ArchitectureService::new("server".to_string()).with_title("Server"))
            .unwrap();
        db.add_service(ArchitectureService::new("gateway".to_string()).with_title("Gateway"))
            .unwrap();
        db.add_edge(ArchitectureEdge::new(
            "db".to_string(),
            ArchitectureDirection::Left,
            "server".to_string(),
            ArchitectureDirection::Right,
        ))
        .unwrap();
        db.add_edge(
            ArchitectureEdge::new(
                "gateway".to_string(),
                ArchitectureDirection::Right,
                "server".to_string(),
                ArchitectureDirection::Left,
            )
            .with_rhs_into(),
        )
        .unwrap();

        let estimator = CharacterSizeEstimator::default();
        let graph = layout_architecture(&db, &estimator).unwrap();

        let db_node = graph.get_node("db").unwrap();
        let server_node = graph.get_node("server").unwrap();
        let gateway_node = graph.get_node("gateway").unwrap();

        let db_x = db_node.x.unwrap();
        let server_x = server_node.x.unwrap();
        let gateway_x = gateway_node.x.unwrap();

        assert!(db_x > server_x, "db should be to the right of server");
        assert!(
            gateway_x < server_x,
            "gateway should be to the left of server"
        );
    }

    #[test]
    fn test_overlapping_nodes_same_direction() {
        let mut db = ArchitectureDb::new();
        // A -> B (Right)
        // A -> C (Right)
        db.add_service(ArchitectureService::new("A".to_string()))
            .unwrap();
        db.add_service(ArchitectureService::new("B".to_string()))
            .unwrap();
        db.add_service(ArchitectureService::new("C".to_string()))
            .unwrap();

        db.add_edge(ArchitectureEdge::new(
            "A".to_string(),
            ArchitectureDirection::Right,
            "B".to_string(),
            ArchitectureDirection::Left,
        ))
        .unwrap();

        db.add_edge(ArchitectureEdge::new(
            "A".to_string(),
            ArchitectureDirection::Right,
            "C".to_string(),
            ArchitectureDirection::Left,
        ))
        .unwrap();

        let estimator = CharacterSizeEstimator::default();
        let graph = layout_architecture(&db, &estimator).unwrap();

        let node_b = graph.get_node("B").unwrap();
        let node_c = graph.get_node("C").unwrap();

        let b_pos = (node_b.x.unwrap(), node_b.y.unwrap());
        let c_pos = (node_c.x.unwrap(), node_c.y.unwrap());

        assert_ne!(b_pos, c_pos, "Nodes B and C should not overlap");
    }

    #[test]

    fn test_cross_group_separation() {
        let mut db = ArchitectureDb::new();

        // G1: A -> G2: B (Right)

        db.add_group(ArchitectureGroup::new("G1".to_string()))
            .unwrap();

        db.add_group(ArchitectureGroup::new("G2".to_string()))
            .unwrap();

        db.add_service(ArchitectureService::new("A".to_string()).with_parent("G1"))
            .unwrap();

        db.add_service(ArchitectureService::new("B".to_string()).with_parent("G2"))
            .unwrap();

        db.add_edge(ArchitectureEdge::new(
            "A".to_string(),
            ArchitectureDirection::Right,
            "B".to_string(),
            ArchitectureDirection::Left,
        ))
        .unwrap();

        let estimator = CharacterSizeEstimator::default();

        let graph = layout_architecture(&db, &estimator).unwrap();

        let node_a = graph.get_node("A").unwrap();

        let node_b = graph.get_node("B").unwrap();

        let ax = node_a.x.unwrap();

        let bx = node_b.x.unwrap();

        assert!(
            bx - ax >= ARCH_NODE_SPACING - 1.0,
            "Nodes in different groups should be separated. ax={}, bx={}",
            ax,
            bx
        );
    }
}
