//! Diagram type detection

use regex::Regex;
use std::sync::LazyLock;

use crate::error::{MermaidError, Result};

/// Supported diagram types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramType {
    Architecture,
    Block,
    C4,
    Class,
    Er,
    Flowchart,
    Gantt,
    Git,
    Info,
    Journey,
    Kanban,
    Mindmap,
    Packet,
    Pie,
    Quadrant,
    Radar,
    Requirement,
    Sankey,
    Sequence,
    State,
    Timeline,
    Treemap,
    XyChart,
}

// Regex patterns for detecting diagram types
static FLOWCHART_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*(flowchart|graph)\s*(TB|BT|RL|LR|TD)?").unwrap());

static PIE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*pie").unwrap());
static MINDMAP_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*mindmap").unwrap());
static INFO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*info").unwrap());
static SEQUENCE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*sequenceDiagram").unwrap());
static CLASS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*classDiagram").unwrap());
static STATE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*stateDiagram").unwrap());
static ER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*erDiagram").unwrap());
static GANTT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*gantt").unwrap());
static JOURNEY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*(journey|user-journey)").unwrap());
static GIT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*gitGraph").unwrap());
static REQUIREMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*requirementDiagram").unwrap());
static QUADRANT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*quadrantChart").unwrap());
static C4_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*C4(Context|Container|Component|Dynamic|Deployment)").unwrap()
});
static TIMELINE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*timeline").unwrap());
static SANKEY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*sankey(-beta)?").unwrap());
static XYCHART_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*xychart(-beta)?").unwrap());
static BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*block(-beta)?").unwrap());
static PACKET_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*packet(-beta)?").unwrap());
static ARCHITECTURE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*architecture(-beta)?").unwrap());
static KANBAN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*kanban").unwrap());
static RADAR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*radar(-beta)?").unwrap());
static TREEMAP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*treemap(-beta)?").unwrap());

/// Detect the type of diagram from input text
pub fn detect_type(input: &str) -> Result<DiagramType> {
    // Remove comments and frontmatter
    let cleaned = remove_frontmatter(input);
    let cleaned = remove_comments(&cleaned);

    // Check in order of specificity (more specific patterns first)
    if SEQUENCE_RE.is_match(&cleaned) {
        return Ok(DiagramType::Sequence);
    }
    if CLASS_RE.is_match(&cleaned) {
        return Ok(DiagramType::Class);
    }
    if STATE_RE.is_match(&cleaned) {
        return Ok(DiagramType::State);
    }
    if ER_RE.is_match(&cleaned) {
        return Ok(DiagramType::Er);
    }
    if GANTT_RE.is_match(&cleaned) {
        return Ok(DiagramType::Gantt);
    }
    if JOURNEY_RE.is_match(&cleaned) {
        return Ok(DiagramType::Journey);
    }
    if GIT_RE.is_match(&cleaned) {
        return Ok(DiagramType::Git);
    }
    if REQUIREMENT_RE.is_match(&cleaned) {
        return Ok(DiagramType::Requirement);
    }
    if QUADRANT_RE.is_match(&cleaned) {
        return Ok(DiagramType::Quadrant);
    }
    if C4_RE.is_match(&cleaned) {
        return Ok(DiagramType::C4);
    }
    if PIE_RE.is_match(&cleaned) {
        return Ok(DiagramType::Pie);
    }
    if MINDMAP_RE.is_match(&cleaned) {
        return Ok(DiagramType::Mindmap);
    }
    if INFO_RE.is_match(&cleaned) {
        return Ok(DiagramType::Info);
    }
    if TIMELINE_RE.is_match(&cleaned) {
        return Ok(DiagramType::Timeline);
    }
    if SANKEY_RE.is_match(&cleaned) {
        return Ok(DiagramType::Sankey);
    }
    if XYCHART_RE.is_match(&cleaned) {
        return Ok(DiagramType::XyChart);
    }
    if BLOCK_RE.is_match(&cleaned) {
        return Ok(DiagramType::Block);
    }
    if PACKET_RE.is_match(&cleaned) {
        return Ok(DiagramType::Packet);
    }
    if ARCHITECTURE_RE.is_match(&cleaned) {
        return Ok(DiagramType::Architecture);
    }
    if KANBAN_RE.is_match(&cleaned) {
        return Ok(DiagramType::Kanban);
    }
    if RADAR_RE.is_match(&cleaned) {
        return Ok(DiagramType::Radar);
    }
    if TREEMAP_RE.is_match(&cleaned) {
        return Ok(DiagramType::Treemap);
    }
    if FLOWCHART_RE.is_match(&cleaned) {
        return Ok(DiagramType::Flowchart);
    }

    Err(MermaidError::UnknownDiagramType(
        "Could not detect diagram type".to_string(),
    ))
}

/// Remove YAML frontmatter from input
fn remove_frontmatter(input: &str) -> String {
    let trimmed = input.trim_start();
    if let Some(after_start) = trimmed.strip_prefix("---") {
        if let Some(end_pos) = after_start.find("---") {
            return after_start[end_pos + 3..].to_string();
        }
    }
    input.to_string()
}

/// Remove comments from input
fn remove_comments(input: &str) -> String {
    input
        .lines()
        .filter(|line| !line.trim().starts_with("%%"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_flowchart() {
        assert_eq!(
            detect_type("flowchart LR\n  A --> B").unwrap(),
            DiagramType::Flowchart
        );
        assert_eq!(
            detect_type("graph TD\n  A --> B").unwrap(),
            DiagramType::Flowchart
        );
        assert_eq!(
            detect_type("  flowchart TB\n  A --> B").unwrap(),
            DiagramType::Flowchart
        );
    }

    #[test]
    fn detect_pie() {
        assert_eq!(detect_type("pie\n  \"A\": 50").unwrap(), DiagramType::Pie);
        assert_eq!(detect_type("  pie\n  \"A\": 50").unwrap(), DiagramType::Pie);
        assert_eq!(
            detect_type("pie showData\n  \"A\": 50").unwrap(),
            DiagramType::Pie
        );
    }

    #[test]
    fn detect_mindmap() {
        assert_eq!(
            detect_type("mindmap\n  root").unwrap(),
            DiagramType::Mindmap
        );
        assert_eq!(
            detect_type("  mindmap\n  root").unwrap(),
            DiagramType::Mindmap
        );
    }

    #[test]
    fn detect_info() {
        assert_eq!(detect_type("info").unwrap(), DiagramType::Info);
        assert_eq!(detect_type("info showInfo").unwrap(), DiagramType::Info);
        assert_eq!(detect_type("  info").unwrap(), DiagramType::Info);
    }

    #[test]
    fn detect_with_comments() {
        assert_eq!(
            detect_type("%% this is a comment\nflowchart LR\n  A --> B").unwrap(),
            DiagramType::Flowchart
        );
    }

    #[test]
    fn unknown_type() {
        assert!(detect_type("unknown diagram").is_err());
    }
}
