//! Rendering engine for mermaid diagrams
//!
//! This module provides SVG rendering for positioned diagram elements.

mod class;
mod er;
mod flowchart;
mod gantt;
mod pie;
mod sequence;
mod state;
pub mod svg;

use crate::diagrams::Diagram;
use crate::error::{MermaidError, Result};
use crate::layout::{self, CharacterSizeEstimator, ToLayoutGraph};

pub use svg::{RenderConfig, SvgRenderer, Theme};

/// Render a diagram to SVG
pub fn render(diagram: &Diagram) -> Result<String> {
    render_with_config(diagram, &RenderConfig::default())
}

/// Render a diagram to SVG with custom configuration
pub fn render_with_config(diagram: &Diagram, config: &RenderConfig) -> Result<String> {
    match diagram {
        Diagram::Flowchart(db) => render_flowchart(db, config),
        Diagram::Pie(db) => pie::render_pie(db, config),
        Diagram::Sequence(db) => sequence::render_sequence(db, config),
        Diagram::Class(db) => class::render_class(db, config),
        Diagram::State(db) => state::render_state(db, config),
        Diagram::Er(db) => er::render_er(db, config),
        Diagram::Gantt(db) => {
            let mut db_clone = db.clone();
            gantt::render_gantt(&mut db_clone, config)
        }
        _ => Err(MermaidError::RenderError(format!(
            "Diagram type {:?} not yet supported for rendering",
            diagram_type_name(diagram)
        ))),
    }
}

/// Get the name of the diagram type for error messages
fn diagram_type_name(diagram: &Diagram) -> &'static str {
    match diagram {
        Diagram::Architecture(_) => "Architecture",
        Diagram::Block(_) => "Block",
        Diagram::C4(_) => "C4",
        Diagram::Class(_) => "Class",
        Diagram::Er(_) => "ER",
        Diagram::Flowchart(_) => "Flowchart",
        Diagram::Gantt(_) => "Gantt",
        Diagram::Git(_) => "Git",
        Diagram::Info(_) => "Info",
        Diagram::Journey(_) => "Journey",
        Diagram::Kanban(_) => "Kanban",
        Diagram::Mindmap(_) => "Mindmap",
        Diagram::Packet(_) => "Packet",
        Diagram::Pie(_) => "Pie",
        Diagram::Quadrant(_) => "Quadrant",
        Diagram::Radar(_) => "Radar",
        Diagram::Requirement(_) => "Requirement",
        Diagram::Sankey(_) => "Sankey",
        Diagram::Sequence(_) => "Sequence",
        Diagram::State(_) => "State",
        Diagram::Timeline(_) => "Timeline",
        Diagram::Treemap(_) => "Treemap",
        Diagram::XyChart(_) => "XyChart",
    }
}

/// Render a flowchart diagram
fn render_flowchart(
    db: &crate::diagrams::flowchart::FlowchartDb,
    config: &RenderConfig,
) -> Result<String> {
    let size_estimator = CharacterSizeEstimator::default();

    // Convert to layout graph
    let graph = db.to_layout_graph(&size_estimator)?;

    // Run layout algorithm
    let graph = layout::layout(graph)?;

    // Render to SVG
    let renderer = SvgRenderer::new(config.clone());
    renderer.render_flowchart(db, &graph)
}
