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
pub const ARCH_CROSS_GROUP_OFFSET: f64 = ARCH_PADDING * ARCH_GROUP_ICON_SCALE;
pub const ARCH_EDGE_GROUP_LABEL_SHIFT: f64 = 18.0;

impl ToLayoutGraph for ArchitectureDb {
    fn to_layout_graph(&self, _size_estimator: &dyn SizeEstimator) -> Result<LayoutGraph> {
        let mut graph = LayoutGraph::new("architecture");

        graph.options = LayoutOptions {
            direction: self.preferred_direction(),
            node_spacing: ARCH_NODE_SPACING,
            layer_spacing: ARCH_NODE_SPACING,
            padding: Padding::uniform(ARCH_PADDING),
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
    let spatial_maps = build_spatial_maps(&adj, &node_ids);

    let mut positions: HashMap<String, (f64, f64)> = HashMap::new();
    let mut offset_x = 0.0;

    for spatial_map in spatial_maps {
        let mut component_positions: Vec<(String, f64, f64)> = Vec::new();
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;

        for (id, (grid_x, grid_y)) in spatial_map {
            let cx = (grid_x as f64) * ARCH_NODE_SPACING;
            let cy = (-grid_y as f64) * ARCH_NODE_SPACING;
            min_x = min_x.min(cx);
            max_x = max_x.max(cx);
            component_positions.push((id, cx, cy));
        }

        let shift_x = if min_x == f64::MAX {
            0.0
        } else {
            offset_x - min_x
        };
        for (id, cx, cy) in component_positions {
            positions.insert(id, (cx + shift_x, cy));
        }

        if min_x != f64::MAX {
            offset_x = max_x + shift_x + ARCH_NODE_SPACING;
        }
    }

    apply_cross_group_offsets(db, &mut positions);

    let half_icon = ARCH_ICON_SIZE / 2.0;
    for (id, (cx, cy)) in positions {
        if let Some(node) = graph.get_node_mut(&id) {
            node.x = Some(cx - half_icon);
            node.y = Some(cy - half_icon);
        }
    }

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

fn build_adjacency(
    db: &ArchitectureDb,
    node_ids: &[String],
) -> HashMap<String, Vec<(ArchitectureDirectionPair, String)>> {
    let mut adj: HashMap<String, Vec<(ArchitectureDirectionPair, String)>> = HashMap::new();
    for id in node_ids {
        adj.insert(id.clone(), Vec::new());
    }

    for edge in db.get_edges() {
        if let Some(pair) = ArchitectureDirectionPair::new(edge.lhs_dir, edge.rhs_dir) {
            adj.entry(edge.lhs_id.clone())
                .or_default()
                .push((pair, edge.rhs_id.clone()));
        }
        if let Some(pair) = ArchitectureDirectionPair::new(edge.rhs_dir, edge.lhs_dir) {
            adj.entry(edge.rhs_id.clone())
                .or_default()
                .push((pair, edge.lhs_id.clone()));
        }
    }

    adj
}

fn build_spatial_maps(
    adj: &HashMap<String, Vec<(ArchitectureDirectionPair, String)>>,
    node_ids: &[String],
) -> Vec<HashMap<String, (i32, i32)>> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut maps = Vec::new();

    for id in node_ids {
        if visited.contains(id) {
            continue;
        }

        let mut spatial_map: HashMap<String, (i32, i32)> = HashMap::new();
        let mut occupied: HashSet<(i32, i32)> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        spatial_map.insert(id.clone(), (0, 0));
        occupied.insert((0, 0));
        queue.push_back(id.clone());

        while let Some(curr) = queue.pop_front() {
            if visited.contains(&curr) {
                continue;
            }
            visited.insert(curr.clone());
            let (x, y) = spatial_map.get(&curr).copied().unwrap_or((0, 0));

            if let Some(neighbors) = adj.get(&curr) {
                for (pair, neighbor) in neighbors {
                    if visited.contains(neighbor) || spatial_map.contains_key(neighbor) {
                        continue;
                    }

                    let (tx, ty) = pair.shift_position(x, y);
                    let (nx, ny) = if occupied.contains(&(tx, ty)) {
                        find_alternative_position((tx, ty), pair.source, &occupied)
                    } else {
                        (tx, ty)
                    };

                    spatial_map.insert(neighbor.clone(), (nx, ny));
                    occupied.insert((nx, ny));
                    queue.push_back(neighbor.clone());
                }
            }
        }

        maps.push(spatial_map);
    }

    maps
}

fn find_alternative_position(
    target: (i32, i32),
    direction: ArchitectureDirection,
    occupied: &HashSet<(i32, i32)>,
) -> (i32, i32) {
    let (tx, ty) = target;
    let mut offset = 1;
    let is_horizontal = direction.is_x();

    loop {
        // Try +offset (perpendicular to direction)
        let p1 = if is_horizontal {
            (tx, ty + offset)
        } else {
            (tx + offset, ty)
        };
        if !occupied.contains(&p1) {
            return p1;
        }

        // Try -offset
        let p2 = if is_horizontal {
            (tx, ty - offset)
        } else {
            (tx - offset, ty)
        };
        if !occupied.contains(&p2) {
            return p2;
        }

        offset += 1;
        if offset > 100 {
            // Safety break, give up and overlap if too crowded
            return target;
        }
    }
}

fn apply_cross_group_offsets(db: &ArchitectureDb, positions: &mut HashMap<String, (f64, f64)>) {
    let node_groups = build_node_group_map(db);
    let mut offsets: HashMap<String, (f64, f64)> = HashMap::new();

    for edge in db.get_edges() {
        let lhs_group = node_groups.get(&edge.lhs_id).and_then(|g| g.as_deref());
        let rhs_group = node_groups.get(&edge.rhs_id).and_then(|g| g.as_deref());

        if lhs_group == rhs_group {
            continue;
        }

        let (target_id, dir) = if lhs_group.is_none() && rhs_group.is_some() {
            (edge.lhs_id.as_str(), edge.lhs_dir)
        } else if rhs_group.is_none() && lhs_group.is_some() {
            (edge.rhs_id.as_str(), edge.rhs_dir)
        } else {
            continue;
        };

        let (dx, dy) = architecture_direction_vector(dir);
        let entry = offsets.entry(target_id.to_string()).or_insert((0.0, 0.0));
        entry.0 += -dx * ARCH_CROSS_GROUP_OFFSET;
        entry.1 += -dy * ARCH_CROSS_GROUP_OFFSET;
    }

    for (id, (dx, dy)) in offsets {
        if let Some((x, y)) = positions.get_mut(&id) {
            *x += dx;
            *y += dy;
        }
    }
}

fn architecture_direction_vector(dir: ArchitectureDirection) -> (f64, f64) {
    match dir {
        ArchitectureDirection::Left => (-1.0, 0.0),
        ArchitectureDirection::Right => (1.0, 0.0),
        ArchitectureDirection::Top => (0.0, -1.0),
        ArchitectureDirection::Bottom => (0.0, 1.0),
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
        let nodes = collect_group_nodes(&group.id, &group_children, &direct_nodes);
        if nodes.is_empty() {
            continue;
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for node_id in nodes {
            let Some(node) = graph.get_node(&node_id) else {
                continue;
            };
            let (Some(x), Some(y)) = (node.x, node.y) else {
                continue;
            };
            let mut height = node.height;
            if let Some(label_height) = label_heights.get(&node_id) {
                height += label_height;
            }
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + node.width);
            max_y = max_y.max(y + height);
        }

        if min_x == f64::MAX {
            continue;
        }

        let rect_x = min_x - ARCH_GROUP_PADDING;
        let rect_y = min_y - ARCH_GROUP_PADDING;
        let rect_w = (max_x - min_x) + ARCH_GROUP_PADDING * 2.0;
        let rect_h = (max_y - min_y) + ARCH_GROUP_PADDING * 2.0;

        bounds_map.insert(
            group.id.clone(),
            GroupBounds {
                x: rect_x,
                y: rect_y,
                width: rect_w,
                height: rect_h,
            },
        );
    }

    bounds_map
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

#[derive(Debug, Clone, Copy)]
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

    fn shift_position(&self, x: i32, y: i32) -> (i32, i32) {
        let source = self.source;
        let target = self.target;
        if source.is_x() {
            let dx = if source == ArchitectureDirection::Left {
                -1
            } else {
                1
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
                1
            } else {
                -1
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
}
