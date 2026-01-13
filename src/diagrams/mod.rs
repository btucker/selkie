//! Diagram types and parsing

pub mod architecture;
pub mod block;
pub mod c4;
pub mod class;
pub mod er;
pub mod flowchart;
pub mod gantt;
pub mod git;
pub mod info;
pub mod journey;
pub mod kanban;
pub mod mindmap;
pub mod packet;
pub mod pie;
pub mod quadrant;
pub mod radar;
pub mod requirement;
pub mod sankey;
pub mod sequence;
pub mod state;
pub mod timeline;
pub mod treemap;
pub mod xychart;

mod detect;
pub mod directive;

pub use detect::{detect_type, DiagramType};
pub use directive::{detect_init, remove_directives, DiagramConfig};

use crate::error::Result;

/// A parsed mermaid diagram
#[derive(Debug, Clone)]
pub enum Diagram {
    Architecture(architecture::ArchitectureDb),
    Block(block::BlockDb),
    C4(c4::C4Db),
    Class(class::ClassDb),
    Er(er::ErDb),
    Flowchart(flowchart::FlowchartDb),
    Gantt(gantt::GanttDb),
    Git(git::GitGraphDb),
    Info(info::InfoDb),
    Journey(journey::JourneyDb),
    Kanban(kanban::KanbanDb),
    Mindmap(mindmap::MindmapDb),
    Packet(packet::PacketDb),
    Pie(pie::PieDb),
    Quadrant(quadrant::QuadrantDb),
    Radar(radar::RadarDb),
    Requirement(requirement::RequirementDb),
    Sankey(sankey::SankeyDb),
    Sequence(sequence::SequenceDb),
    State(state::StateDb),
    Timeline(timeline::TimelineDb),
    Treemap(treemap::TreemapDb),
    XyChart(xychart::XYChartDb),
}

/// Parse a diagram of a specific type
pub fn parse(diagram_type: DiagramType, input: &str) -> Result<Diagram> {
    match diagram_type {
        DiagramType::Architecture => Ok(Diagram::Architecture(architecture::parse(input)?)),
        DiagramType::Block => Ok(Diagram::Block(block::parse(input)?)),
        DiagramType::C4 => Ok(Diagram::C4(c4::parse(input)?)),
        DiagramType::Class => Ok(Diagram::Class(class::parse(input)?)),
        DiagramType::Er => Ok(Diagram::Er(er::parse(input)?)),
        DiagramType::Flowchart => Ok(Diagram::Flowchart(flowchart::parse(input)?)),
        DiagramType::Gantt => Ok(Diagram::Gantt(gantt::parse(input)?)),
        DiagramType::Git => Ok(Diagram::Git(git::parse(input)?)),
        DiagramType::Info => Ok(Diagram::Info(info::parse(input)?)),
        DiagramType::Journey => Ok(Diagram::Journey(journey::parse(input)?)),
        DiagramType::Kanban => Ok(Diagram::Kanban(kanban::parse(input)?)),
        DiagramType::Mindmap => Ok(Diagram::Mindmap(mindmap::parse(input)?)),
        DiagramType::Packet => Ok(Diagram::Packet(packet::parse(input)?)),
        DiagramType::Pie => Ok(Diagram::Pie(pie::parse(input)?)),
        DiagramType::Quadrant => Ok(Diagram::Quadrant(quadrant::parse(input)?)),
        DiagramType::Radar => Ok(Diagram::Radar(radar::parse(input)?)),
        DiagramType::Requirement => Ok(Diagram::Requirement(requirement::parse(input)?)),
        DiagramType::Sankey => Ok(Diagram::Sankey(sankey::parse(input)?)),
        DiagramType::Sequence => Ok(Diagram::Sequence(sequence::parse(input)?)),
        DiagramType::State => Ok(Diagram::State(state::parse(input)?)),
        DiagramType::Timeline => Ok(Diagram::Timeline(timeline::parse(input)?)),
        DiagramType::Treemap => Ok(Diagram::Treemap(treemap::parse(input)?)),
        DiagramType::XyChart => Ok(Diagram::XyChart(xychart::parse(input)?)),
    }
}
