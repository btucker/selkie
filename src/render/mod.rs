//! Rendering engine for mermaid diagrams
//!
//! This module provides SVG rendering for positioned diagram elements.

mod architecture;
mod class;
mod er;
mod flowchart;
mod gantt;
mod git;
mod mindmap;
mod packet;
mod pie;
mod sequence;
mod state;
pub mod svg;
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
        Diagram::Packet(db) => packet::render_packet(db, config),
        Diagram::XyChart(db) => xychart::render_xychart(db, config),
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
