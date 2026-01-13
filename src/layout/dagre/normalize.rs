//! Edge normalization for dagre layout
//!
//! This module breaks long edges (spanning multiple ranks) into short segments
//! that span exactly 1 rank each. Dummy nodes are created at intermediate ranks.
//!
//! After layout, `undo` collects the positions of dummy nodes into edge points
//! for proper edge routing.

use super::graph::{DagreGraph, EdgeLabel, NodeLabel};
use crate::layout::NodeShape;

/// Run normalization: break long edges into unit-length segments
pub fn run(graph: &mut DagreGraph) {
    // Collect edges that need normalization
    let edges_to_normalize: Vec<_> = graph
        .edges()
        .iter()
        .filter_map(|key| {
            let v = &key.v;
            let w = &key.w;
            let name = key.name.clone();
            let v_rank = graph.node(v).and_then(|n| n.rank)?;
            let w_rank = graph.node(w).and_then(|n| n.rank)?;

            // Only normalize edges that span more than 1 rank
            if w_rank != v_rank + 1 {
                Some((v.clone(), w.clone(), name, v_rank, w_rank))
            } else {
                None
            }
        })
        .collect();

    // Initialize dummy chains storage
    let mut dummy_chains: Vec<String> = Vec::new();

    for (v, w, name, v_rank, w_rank) in edges_to_normalize {
        // Get edge label before removing
        let edge_label = graph.edge(&v, &w).cloned().unwrap_or_default();
        let weight = edge_label.weight;
        let label_rank = edge_label.label_rank;

        // Remove the original long edge
        graph.remove_edge(&v, &w);

        let mut prev_node = v.clone();
        let mut first_dummy: Option<String> = None;

        // Create dummy nodes for each intermediate rank
        for rank in (v_rank + 1)..w_rank {
            let dummy_id = format!("_d{}_{}", rank, graph.node_count());

            let mut dummy_label = NodeLabel {
                width: 0.0,
                height: 0.0,
                rank: Some(rank),
                dummy: Some("edge".to_string()),
                edge_label: Some(Box::new(edge_label.clone())),
                edge_obj: Some((v.clone(), w.clone(), name.clone())),
                ..Default::default()
            };

            // If this is the label rank, give the dummy node the label's dimensions
            if Some(rank) == label_rank {
                dummy_label.width = edge_label.width;
                dummy_label.height = edge_label.height;
                dummy_label.dummy = Some("edge-label".to_string());
                dummy_label.labelpos = Some(edge_label.labelpos.clone());
            }

            graph.set_node(&dummy_id, dummy_label);

            // Connect previous node to this dummy
            graph.set_edge(
                &prev_node,
                &dummy_id,
                EdgeLabel {
                    weight,
                    ..Default::default()
                },
            );

            if first_dummy.is_none() {
                first_dummy = Some(dummy_id.clone());
            }

            prev_node = dummy_id;
        }

        // Connect last dummy to the target
        graph.set_edge(
            &prev_node,
            &w,
            EdgeLabel {
                weight,
                ..Default::default()
            },
        );

        // Track the first dummy in this chain
        if let Some(dummy) = first_dummy {
            dummy_chains.push(dummy);
        }
    }

    // Store dummy chains in graph
    graph.graph_mut().dummy_chains = dummy_chains;
}

/// Undo normalization: collect dummy node positions into edge points
pub fn undo(graph: &mut DagreGraph) {
    let dummy_chains = std::mem::take(&mut graph.graph_mut().dummy_chains);

    for start_dummy in dummy_chains {
        let mut current = start_dummy;

        // Get the original edge info from the first dummy
        let (orig_v, orig_w, orig_name, mut orig_label) = {
            let node = match graph.node(&current) {
                Some(n) => n,
                None => continue,
            };

            let edge_obj = match &node.edge_obj {
                Some(obj) => obj.clone(),
                None => continue,
            };

            let label = node
                .edge_label
                .as_ref()
                .map(|b| (**b).clone())
                .unwrap_or_default();
            (edge_obj.0, edge_obj.1, edge_obj.2, label)
        };

        // Collect points from dummy nodes
        let mut points = Vec::new();

        while let Some(n) = graph.node(&current) {
            let node = n.clone();

            // Check if this is still a dummy node
            if node.dummy.is_none() {
                break;
            }

            // Get successor before removing this node
            let successors: Vec<_> = graph.successors(&current).into_iter().cloned().collect();
            let next = successors.into_iter().next();

            // Add this dummy's position to points
            if let (Some(x), Some(y)) = (node.x, node.y) {
                points.push(super::graph::Point { x, y });
            }

            // Handle edge-label dummy specially
            if node.dummy.as_deref() == Some("edge-label") {
                orig_label.x = node.x;
                orig_label.y = node.y;
                orig_label.width = node.width;
                orig_label.height = node.height;
            }

            // Remove this dummy node
            graph.remove_node(&current);

            // Move to next
            current = match next {
                Some(n) => n,
                None => break,
            };
        }

        // Store points in the original edge label
        orig_label.points = points;

        // Restore the original edge with collected points
        if let Some(name) = &orig_name {
            graph.set_edge_with_name(&orig_v, &orig_w, orig_label, name);
        } else {
            graph.set_edge(&orig_v, &orig_w, orig_label);
        }
    }
}

/// Compute intersection point of a line from point p to node center with the node's boundary
/// Dispatches to the appropriate shape-specific intersection function
pub fn intersect_node(node: &NodeLabel, p: &super::graph::Point) -> super::graph::Point {
    match node.shape {
        NodeShape::Diamond => intersect_diamond(node, p),
        NodeShape::Circle | NodeShape::DoubleCircle => intersect_circle(node, p),
        NodeShape::Ellipse => intersect_ellipse(node, p),
        _ => intersect_rect(node, p),
    }
}

/// Compute intersection point of a line from point p to node center with a rectangular boundary
pub fn intersect_rect(node: &NodeLabel, p: &super::graph::Point) -> super::graph::Point {
    let (cx, cy) = match (node.x, node.y) {
        (Some(x), Some(y)) => (x, y),
        _ => return *p,
    };

    let w = node.width / 2.0;
    let h = node.height / 2.0;

    let dx = p.x - cx;
    let dy = p.y - cy;

    if dx == 0.0 && dy == 0.0 {
        return super::graph::Point { x: cx, y: cy };
    }

    // Compute intersection with rectangle boundary
    let (sx, sy) = if dx.abs() * h > dy.abs() * w {
        // Intersects left or right edge
        let sx = if dx > 0.0 { w } else { -w };
        let sy = sy_for_sx(dx, dy, sx);
        (sx, sy)
    } else {
        // Intersects top or bottom edge
        let sy = if dy > 0.0 { h } else { -h };
        let sx = sx_for_sy(dx, dy, sy);
        (sx, sy)
    };

    super::graph::Point {
        x: cx + sx,
        y: cy + sy,
    }
}

/// Compute intersection point for a diamond (rhombus) shape
/// Diamond vertices are at (cx, cy-h), (cx+w, cy), (cx, cy+h), (cx-w, cy)
pub fn intersect_diamond(node: &NodeLabel, p: &super::graph::Point) -> super::graph::Point {
    let (cx, cy) = match (node.x, node.y) {
        (Some(x), Some(y)) => (x, y),
        _ => return *p,
    };

    let w = node.width / 2.0;
    let h = node.height / 2.0;

    let dx = p.x - cx;
    let dy = p.y - cy;

    if dx == 0.0 && dy == 0.0 {
        return super::graph::Point { x: cx, y: cy };
    }

    // For a diamond, the boundary satisfies: |dx/w| + |dy/h| = 1
    // We need to find the intersection point along the line from center to p
    // Parametrize the line as (t*dx, t*dy) where t goes from 0 to 1
    // At the boundary: |t*dx/w| + |t*dy/h| = 1
    // So: t * (|dx|/w + |dy|/h) = 1
    // t = 1 / (|dx|/w + |dy|/h)

    let t = 1.0 / (dx.abs() / w + dy.abs() / h);

    super::graph::Point {
        x: cx + t * dx,
        y: cy + t * dy,
    }
}

/// Compute intersection point for a circle shape
pub fn intersect_circle(node: &NodeLabel, p: &super::graph::Point) -> super::graph::Point {
    let (cx, cy) = match (node.x, node.y) {
        (Some(x), Some(y)) => (x, y),
        _ => return *p,
    };

    // For circles, use the smaller of width/height as diameter
    let r = node.width.min(node.height) / 2.0;

    let dx = p.x - cx;
    let dy = p.y - cy;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist == 0.0 {
        return super::graph::Point { x: cx, y: cy };
    }

    // Point on circle at radius r in direction of p
    super::graph::Point {
        x: cx + r * dx / dist,
        y: cy + r * dy / dist,
    }
}

/// Compute intersection point for an ellipse shape
pub fn intersect_ellipse(node: &NodeLabel, p: &super::graph::Point) -> super::graph::Point {
    let (cx, cy) = match (node.x, node.y) {
        (Some(x), Some(y)) => (x, y),
        _ => return *p,
    };

    let rx = node.width / 2.0;
    let ry = node.height / 2.0;

    let dx = p.x - cx;
    let dy = p.y - cy;

    if dx == 0.0 && dy == 0.0 {
        return super::graph::Point { x: cx, y: cy };
    }

    // For an ellipse: (x/rx)^2 + (y/ry)^2 = 1
    // Line from center: (t*dx, t*dy)
    // (t*dx/rx)^2 + (t*dy/ry)^2 = 1
    // t^2 * ((dx/rx)^2 + (dy/ry)^2) = 1
    let t = 1.0 / ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt();

    super::graph::Point {
        x: cx + t * dx,
        y: cy + t * dy,
    }
}

fn sy_for_sx(dx: f64, dy: f64, sx: f64) -> f64 {
    if dx == 0.0 {
        0.0
    } else {
        dy * sx / dx
    }
}

fn sx_for_sy(dx: f64, dy: f64, sy: f64) -> f64 {
    if dy == 0.0 {
        0.0
    } else {
        dx * sy / dy
    }
}

/// Assign node intersection points to edges
/// This adds the start and end points where edges meet node boundaries
pub fn assign_node_intersects(graph: &mut DagreGraph) {
    // Calculate graph center from actual node positions (more reliable than graph.width/height)
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for node_id in graph.nodes() {
        if let Some(node) = graph.node(node_id) {
            let x = node.x.unwrap_or(0.0);
            let y = node.y.unwrap_or(0.0);
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
    }
    let _graph_center_x = (min_x + max_x) / 2.0;
    let _graph_center_y = (min_y + max_y) / 2.0;

    let rankdir = graph.graph().rankdir.as_str();
    let _is_horizontal = rankdir == "LR" || rankdir == "RL";

    // Collect edge data (v, w, points) upfront to avoid borrow issues
    let edge_data: Vec<_> = graph
        .edges()
        .iter()
        .filter_map(|key| {
            let v = key.v.clone();
            let w = key.w.clone();
            let node_v = graph.node(&v)?.clone();
            let node_w = graph.node(&w)?.clone();
            let points = graph
                .edge(&v, &w)
                .map(|e| e.points.clone())
                .unwrap_or_default();
            Some((v, w, node_v, node_w, points))
        })
        .collect();

    for (v, w, node_v, node_w, mut points) in edge_data {
        // Determine start and end reference points
        let (p1, p2) = if points.is_empty() {
            // No intermediate points - use node centers
            let p1 = super::graph::Point {
                x: node_w.x.unwrap_or(0.0),
                y: node_w.y.unwrap_or(0.0),
            };
            let p2 = super::graph::Point {
                x: node_v.x.unwrap_or(0.0),
                y: node_v.y.unwrap_or(0.0),
            };
            (p1, p2)
        } else {
            // Use first and last intermediate points
            let p1 = points.first().cloned().unwrap();
            let p2 = points.last().cloned().unwrap();
            (p1, p2)
        };

        // Compute intersections with node boundaries (shape-aware)
        let start_point = intersect_node(&node_v, &p1);
        let end_point = intersect_node(&node_w, &p2);

        // Add start and end points
        points.insert(0, start_point);
        points.push(end_point);

        // For edges with only 2 points (no intermediate dummy nodes from normalization),
        // we do NOT add a midpoint. This matches dagre's behavior where short edges
        // (between adjacent ranks) are straight lines from source boundary to target
        // boundary. The intersection calculation already ensures edges connect at the
        // correct points (e.g., bottom of source and top of target in TB layout).
        //
        // Mermaid.js renders these as straight lines with `curveBasis`, which for 2 points
        // is just a straight line. Adding artificial midpoints caused edges to curve
        // excessively and enter nodes from the wrong side (sides instead of top in TB).

        // Update edge with new points
        if let Some(edge) = graph.edge_mut(&v, &w) {
            edge.points = points;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intersect_rect_from_below() {
        let node = NodeLabel {
            x: Some(100.0),
            y: Some(100.0),
            width: 50.0,
            height: 50.0,
            ..Default::default()
        };

        // Point below the node
        let p = super::super::graph::Point { x: 100.0, y: 200.0 };
        let intersection = intersect_rect(&node, &p);

        // Should intersect bottom edge
        assert!((intersection.x - 100.0).abs() < 0.01);
        assert!((intersection.y - 125.0).abs() < 0.01);
    }

    #[test]
    fn test_intersect_rect_from_right() {
        let node = NodeLabel {
            x: Some(100.0),
            y: Some(100.0),
            width: 50.0,
            height: 50.0,
            ..Default::default()
        };

        // Point to the right of the node
        let p = super::super::graph::Point { x: 200.0, y: 100.0 };
        let intersection = intersect_rect(&node, &p);

        // Should intersect right edge
        assert!((intersection.x - 125.0).abs() < 0.01);
        assert!((intersection.y - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_intersect_diamond_from_right() {
        // Diamond centered at (100, 100) with width 50, height 50
        // Vertices at (100, 75), (125, 100), (100, 125), (75, 100)
        let node = NodeLabel {
            x: Some(100.0),
            y: Some(100.0),
            width: 50.0,
            height: 50.0,
            shape: NodeShape::Diamond,
            ..Default::default()
        };

        // Point directly to the right - should hit right vertex
        let p = super::super::graph::Point { x: 200.0, y: 100.0 };
        let intersection = intersect_diamond(&node, &p);

        // Should intersect at right vertex (125, 100)
        assert!(
            (intersection.x - 125.0).abs() < 0.01,
            "x={}",
            intersection.x
        );
        assert!(
            (intersection.y - 100.0).abs() < 0.01,
            "y={}",
            intersection.y
        );
    }

    #[test]
    fn test_intersect_diamond_from_below() {
        let node = NodeLabel {
            x: Some(100.0),
            y: Some(100.0),
            width: 50.0,
            height: 50.0,
            shape: NodeShape::Diamond,
            ..Default::default()
        };

        // Point directly below - should hit bottom vertex
        let p = super::super::graph::Point { x: 100.0, y: 200.0 };
        let intersection = intersect_diamond(&node, &p);

        // Should intersect at bottom vertex (100, 125)
        assert!(
            (intersection.x - 100.0).abs() < 0.01,
            "x={}",
            intersection.x
        );
        assert!(
            (intersection.y - 125.0).abs() < 0.01,
            "y={}",
            intersection.y
        );
    }

    #[test]
    fn test_intersect_diamond_from_diagonal() {
        // Diamond centered at (100, 100) with width 50, height 50
        // For a point at 45 degrees, the intersection should be on the edge
        let node = NodeLabel {
            x: Some(100.0),
            y: Some(100.0),
            width: 50.0,
            height: 50.0,
            shape: NodeShape::Diamond,
            ..Default::default()
        };

        // Point at 45 degrees (lower-right)
        let p = super::super::graph::Point { x: 200.0, y: 200.0 };
        let intersection = intersect_diamond(&node, &p);

        // For |dx|=|dy| and w=h, the intersection should be at t = 0.5
        // (100 + 0.5*100, 100 + 0.5*100) = (150, 150)? No...
        // Actually: t = 1/(|dx|/w + |dy|/h) = 1/(100/25 + 100/25) = 1/8
        // Wait, w = width/2 = 25, h = height/2 = 25
        // t = 1/(100/25 + 100/25) = 1/(4+4) = 1/8
        // point = (100 + 100/8, 100 + 100/8) = (112.5, 112.5)
        assert!(
            (intersection.x - 112.5).abs() < 0.01,
            "x={}",
            intersection.x
        );
        assert!(
            (intersection.y - 112.5).abs() < 0.01,
            "y={}",
            intersection.y
        );
    }

    #[test]
    fn test_intersect_circle_from_right() {
        let node = NodeLabel {
            x: Some(100.0),
            y: Some(100.0),
            width: 50.0,
            height: 50.0,
            shape: NodeShape::Circle,
            ..Default::default()
        };

        // Point directly to the right
        let p = super::super::graph::Point { x: 200.0, y: 100.0 };
        let intersection = intersect_circle(&node, &p);

        // Should intersect at (125, 100) - radius 25 from center
        assert!(
            (intersection.x - 125.0).abs() < 0.01,
            "x={}",
            intersection.x
        );
        assert!(
            (intersection.y - 100.0).abs() < 0.01,
            "y={}",
            intersection.y
        );
    }

    #[test]
    fn test_intersect_node_dispatches_correctly() {
        let rect_node = NodeLabel {
            x: Some(100.0),
            y: Some(100.0),
            width: 50.0,
            height: 50.0,
            shape: NodeShape::Rectangle,
            ..Default::default()
        };

        let diamond_node = NodeLabel {
            x: Some(100.0),
            y: Some(100.0),
            width: 50.0,
            height: 50.0,
            shape: NodeShape::Diamond,
            ..Default::default()
        };

        // Point to the right at same y level
        let p = super::super::graph::Point { x: 200.0, y: 100.0 };

        // Rectangle should intersect at right edge (125, 100)
        let rect_intersection = intersect_node(&rect_node, &p);
        assert!((rect_intersection.x - 125.0).abs() < 0.01);

        // Diamond should also intersect at right vertex (125, 100)
        let diamond_intersection = intersect_node(&diamond_node, &p);
        assert!((diamond_intersection.x - 125.0).abs() < 0.01);

        // But for diagonal points, they should differ
        // Point at (200, 150) - below and to the right
        let diag_p = super::super::graph::Point { x: 200.0, y: 150.0 };
        let rect_diag = intersect_node(&rect_node, &diag_p);
        let diamond_diag = intersect_node(&diamond_node, &diag_p);

        // Rectangle: dx=100, dy=50, |dx|*h (2500) > |dy|*w (1250)
        // So it intersects right edge: sx=25, sy=50*25/100=12.5
        // intersection = (125, 112.5)
        assert!((rect_diag.x - 125.0).abs() < 0.01, "rect x={}", rect_diag.x);
        assert!((rect_diag.y - 112.5).abs() < 0.01, "rect y={}", rect_diag.y);

        // Diamond should intersect differently
        // t = 1/(|dx|/w + |dy|/h) = 1/(100/25 + 50/25) = 1/(4+2) = 1/6
        // intersection = (100 + 100/6, 100 + 50/6) = (116.67, 108.33)
        assert!(
            (diamond_diag.x - 116.67).abs() < 0.1,
            "diamond x={}",
            diamond_diag.x
        );
        assert!(
            (diamond_diag.y - 108.33).abs() < 0.1,
            "diamond y={}",
            diamond_diag.y
        );
    }
}
