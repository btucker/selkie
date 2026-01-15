//! Architecture diagram adapter for layout

use crate::diagrams::architecture::{
    ArchitectureAlignment, ArchitectureDb, ArchitectureEdge, ArchitectureService,
};
use crate::error::Result;
use crate::layout::{
    LayoutDirection, LayoutEdge, LayoutGraph, LayoutNode, LayoutOptions, NodeShape, NodeSizeConfig,
    Padding, SizeEstimator, ToLayoutGraph,
};

impl ToLayoutGraph for ArchitectureDb {
    fn to_layout_graph(&self, size_estimator: &dyn SizeEstimator) -> Result<LayoutGraph> {
        let mut graph = LayoutGraph::new("architecture");
        let size_config = NodeSizeConfig {
            font_size: 14.0,
            padding_horizontal: 18.0,
            padding_vertical: 10.0,
            min_width: 80.0,
            min_height: 40.0,
            max_width: Some(260.0),
        };

        graph.options = LayoutOptions {
            direction: self.preferred_direction(),
            node_spacing: 80.0,
            layer_spacing: 80.0,
            padding: Padding::uniform(20.0),
        };

        let mut groups = self.get_groups();
        groups.sort_by_key(|g| g.id.as_str());
        for group in groups {
            let mut node = LayoutNode::new(&group.id, 0.0, 0.0)
                .with_shape(NodeShape::Rectangle)
                .with_padding(Padding::uniform(20.0));
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
            let label = service
                .title
                .as_deref()
                .or(service.icon_text.as_deref())
                .unwrap_or(service.id.as_str());
            let shape = icon_to_shape(service);
            let (width, height) =
                size_estimator.estimate_node_size(Some(label), shape, &size_config);

            let mut node = LayoutNode::new(&service.id, width, height).with_shape(shape);
            node = node.with_label(label);
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
            let mut node = LayoutNode::new(&junction.id, 14.0, 14.0).with_shape(NodeShape::Circle);
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

fn icon_to_shape(service: &ArchitectureService) -> NodeShape {
    let Some(icon) = service.icon.as_deref() else {
        return NodeShape::RoundedRect;
    };
    match icon.to_ascii_lowercase().as_str() {
        "database" | "disk" => NodeShape::Cylinder,
        "internet" => NodeShape::Circle,
        _ => NodeShape::RoundedRect,
    }
}

fn architecture_alignment(edge: &ArchitectureEdge) -> ArchitectureAlignment {
    crate::diagrams::architecture::get_direction_alignment(edge.lhs_dir, edge.rhs_dir)
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
}
