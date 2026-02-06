//! Rendering engine for mermaid diagrams
//!
//! This module provides SVG rendering for positioned diagram elements.

mod architecture;
pub mod ascii;
mod block;
mod c4;
mod class;
mod er;
mod flowchart;
mod gantt;
mod git;
mod journey;
mod kanban;
mod mindmap;
mod packet;
mod pie;
mod quadrant;
mod radar;
mod requirement;
mod sankey;
mod sequence;
mod state;
pub mod svg;
pub(crate) mod text_utils;
mod timeline;
mod treemap;
mod xychart;

use crate::diagrams::{detect_init, detect_type, parse, remove_directives, Diagram};
use crate::error::{MermaidError, Result};
use crate::layout::{self, CharacterSizeEstimator, ToLayoutGraph};

pub use svg::{RenderConfig, SvgRenderer, Theme};

/// Render a diagram to SVG
pub fn render(diagram: &Diagram) -> Result<String> {
    render_with_config(diagram, &RenderConfig::default())
}

/// Render diagram text to SVG with automatic directive processing
///
/// This function:
/// 1. Detects and parses `%%{init: ...}%%` directives
/// 2. Extracts theme configuration from directives
/// 3. Detects the diagram type
/// 4. Parses the diagram
/// 5. Renders with directive-derived theme configuration
///
/// # Example
///
/// ```
/// use selkie::render::render_text;
///
/// let svg = render_text(r#"%%{init: {"theme": "dark"}}%%
/// flowchart TD
///     A[Start] --> B[End]
/// "#).unwrap();
/// assert!(svg.contains("<svg"));
/// ```
pub fn render_text(text: &str) -> Result<String> {
    // Extract directive configuration
    let directive_config = detect_init(text);

    // Build render config with directive theme and themeCSS
    let config = if let Some(ref dc) = directive_config {
        RenderConfig {
            theme: Theme::from_directive(dc),
            theme_css: dc.theme_css.clone(),
            ..RenderConfig::default()
        }
    } else {
        RenderConfig::default()
    };

    // Remove directives from text before parsing
    let clean_text = remove_directives(text);

    // Detect diagram type and parse
    let diagram_type = detect_type(&clean_text)?;
    let diagram = parse(diagram_type, &clean_text)?;

    // Render with config
    render_with_config(&diagram, &config)
}

/// Render a diagram to SVG with custom configuration
pub fn render_with_config(diagram: &Diagram, config: &RenderConfig) -> Result<String> {
    match diagram {
        Diagram::Architecture(db) => render_architecture(db, config),
        Diagram::Block(db) => block::render_block(db, config),
        Diagram::C4(db) => c4::render_c4(db, config),
        Diagram::Flowchart(db) => render_flowchart(db, config),
        Diagram::Git(db) => git::render_git(db, config),
        Diagram::Pie(db) => pie::render_pie(db, config),
        Diagram::Sequence(db) => sequence::render_sequence(db, config),
        Diagram::Class(db) => class::render_class(db, config),
        Diagram::State(db) => state::render_state(db, config),
        Diagram::Er(db) => er::render_er(db, config),
        Diagram::Gantt(db) => {
            let mut db_clone = db.clone();
            gantt::render_gantt(&mut db_clone, config)
        }
        Diagram::Mindmap(db) => mindmap::render_mindmap(db, config),
        Diagram::Timeline(db) => timeline::render_timeline(db, config),
        Diagram::Requirement(db) => requirement::render_requirement(db, config),
        Diagram::Sankey(db) => sankey::render_sankey(db, config),
        Diagram::Radar(db) => radar::render_radar(db, config),
        Diagram::Packet(db) => packet::render_packet(db, config),
        Diagram::XyChart(db) => xychart::render_xychart(db, config),
        Diagram::Quadrant(db) => quadrant::render_quadrant(db, config),
        Diagram::Treemap(db) => treemap::render_treemap(db, config),
        Diagram::Journey(db) => journey::render_journey(db, config),
        Diagram::Kanban(db) => kanban::render_kanban(db, config),
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

/// Render a diagram to ASCII character art.
///
/// This is the primary entry point for ASCII rendering. It accepts any parsed
/// `Diagram` and dispatches to the appropriate type-specific ASCII renderer.
///
/// # Example
///
/// ```
/// let diagram = selkie::parse("flowchart TD\n    A[Start] --> B[End]").unwrap();
/// let ascii = selkie::render::render_ascii(&diagram).unwrap();
/// assert!(ascii.contains("Start"));
/// ```
pub fn render_ascii(diagram: &Diagram) -> Result<String> {
    use crate::layout::{self, CharacterSizeEstimator, ToLayoutGraph};

    let estimator = CharacterSizeEstimator::default();

    match diagram {
        Diagram::Flowchart(db) => {
            let graph = db.to_layout_graph(&estimator)?;
            let graph = layout::layout(graph)?;
            Ok(ascii::render_flowchart_ascii(db, &graph)?)
        }
        Diagram::Sequence(db) => Ok(ascii::render_sequence_ascii(db)?),
        Diagram::Class(db) => {
            let graph = db.to_layout_graph(&estimator)?;
            let graph = layout::layout(graph)?;
            Ok(ascii::render_class_ascii(db, &graph)?)
        }
        Diagram::State(db) => {
            let graph = db.to_layout_graph(&estimator)?;
            let graph = layout::layout(graph)?;
            Ok(ascii::render_graph_ascii(&graph)?)
        }
        Diagram::Er(db) => {
            let graph = db.to_layout_graph(&estimator)?;
            let graph = layout::layout(graph)?;
            Ok(ascii::render_er_ascii(db, &graph)?)
        }
        Diagram::Architecture(db) => {
            let graph = db.to_layout_graph(&estimator)?;
            let graph = layout::layout(graph)?;
            Ok(ascii::render_graph_ascii(&graph)?)
        }
        Diagram::Requirement(db) => {
            let graph = db.to_layout_graph(&estimator)?;
            let graph = layout::layout(graph)?;
            Ok(ascii::render_graph_ascii(&graph)?)
        }
        Diagram::Pie(db) => Ok(ascii::pie::render_pie_ascii(db)?),
        Diagram::Gantt(db) => {
            let mut db_clone = db.clone();
            Ok(ascii::gantt::render_gantt_ascii(&mut db_clone)?)
        }
        Diagram::Mindmap(db) => Ok(ascii::mindmap::render_mindmap_ascii(db)?),
        Diagram::Journey(db) => Ok(ascii::journey::render_journey_ascii(db)?),
        Diagram::Timeline(db) => Ok(ascii::timeline::render_timeline_ascii(db)?),
        Diagram::Kanban(db) => Ok(ascii::kanban::render_kanban_ascii(db)?),
        Diagram::Packet(db) => Ok(ascii::packet::render_packet_ascii(db)?),
        Diagram::XyChart(db) => Ok(ascii::xychart::render_xychart_ascii(db)?),
        Diagram::Quadrant(db) => Ok(ascii::quadrant::render_quadrant_ascii(db)?),
        Diagram::Radar(db) => Ok(ascii::radar::render_radar_ascii(db)?),
        Diagram::Git(db) => Ok(ascii::gitgraph::render_gitgraph_ascii(db)?),
        Diagram::Sankey(db) => Ok(ascii::sankey::render_sankey_ascii(db)?),
        Diagram::Block(db) => Ok(ascii::block::render_block_ascii(db)?),
        Diagram::C4(db) => Ok(ascii::c4::render_c4_ascii(db)?),
        Diagram::Treemap(db) => Ok(ascii::treemap::render_treemap_ascii(db)?),
        _ => Err(MermaidError::RenderError(
            "ASCII format not yet supported for this diagram type".to_string(),
        )),
    }
}

/// Render mermaid text directly to ASCII character art.
///
/// This is a convenience function that parses the input text and renders it
/// to ASCII in one step, similar to how [`render_text`] works for SVG.
///
/// # Example
///
/// ```
/// let ascii = selkie::render::render_text_ascii("flowchart TD\n    A[Start] --> B[End]").unwrap();
/// assert!(ascii.contains("Start"));
/// ```
pub fn render_text_ascii(text: &str) -> Result<String> {
    let clean_text = remove_directives(text);
    let diagram_type = detect_type(&clean_text)?;
    let diagram = parse(diagram_type, &clean_text)?;
    render_ascii(&diagram)
}

/// Render an architecture diagram
fn render_architecture(
    db: &crate::diagrams::architecture::ArchitectureDb,
    config: &RenderConfig,
) -> Result<String> {
    let size_estimator = CharacterSizeEstimator::default();

    let graph = architecture::layout_architecture(db, &size_estimator)?;

    let renderer = SvgRenderer::new(config.clone());
    renderer.render_architecture(db, &graph)
}
