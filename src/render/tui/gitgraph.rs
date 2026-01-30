//! TUI renderer for git graph diagrams.
//!
//! Renders git history as a text-based commit graph with branch lines,
//! commit markers, and labels. Uses box-drawing and bullet characters
//! to represent the branching structure.

use crate::diagrams::git::GitGraphDb;
use crate::error::Result;

/// Branch lane markers for visual distinction.
const BRANCH_CHARS: &[char] = &['●', '◆', '■', '▲', '◉', '★'];

/// Render a git graph as character art.
pub fn render_gitgraph_tui(db: &GitGraphDb) -> Result<String> {
    let commits = db.get_commits();
    if commits.is_empty() {
        return Ok("(empty git graph)\n".to_string());
    }

    // Title
    let mut lines: Vec<String> = Vec::new();
    if !db.diagram_title.is_empty() {
        lines.push(db.diagram_title.clone());
        lines.push("─".repeat(db.diagram_title.chars().count().max(40)));
        lines.push(String::new());
    }

    // Sort commits by sequence number
    let mut sorted_commits: Vec<_> = commits.values().collect();
    sorted_commits.sort_by_key(|c| c.seq);

    // Build branch order (assign lane indices)
    let branch_configs = db.get_branches_as_obj_array();
    let branch_order: Vec<String> = branch_configs.iter().map(|b| b.name.clone()).collect();

    let get_lane =
        |branch: &str| -> usize { branch_order.iter().position(|b| b == branch).unwrap_or(0) };

    let max_lanes = branch_order.len().max(1);

    // Render each commit
    for commit in &sorted_commits {
        let lane = get_lane(&commit.branch);
        let marker = BRANCH_CHARS.get(lane % BRANCH_CHARS.len()).unwrap_or(&'●');

        // Build the lane prefix
        let mut prefix = String::new();
        for l in 0..max_lanes {
            if l == lane {
                prefix.push(*marker);
            } else if l < branch_order.len() {
                // Show vertical line for active branches
                prefix.push('│');
            } else {
                prefix.push(' ');
            }
            prefix.push(' ');
        }

        // Commit info
        let mut info = String::new();

        // Short commit id
        let short_id = if commit.id.len() > 7 {
            &commit.id[..7]
        } else {
            &commit.id
        };
        info.push_str(short_id);

        // Message
        if !commit.message.is_empty() {
            info.push_str(&format!(" - {}", commit.message));
        }

        // Tags
        if !commit.tags.is_empty() {
            info.push_str(&format!(" [{}]", commit.tags.join(", ")));
        }

        // Branch label (show on first commit of each branch)
        let is_branch_head = db
            .get_branches()
            .get(&commit.branch)
            .and_then(|h| h.as_ref())
            .is_some_and(|h| h == &commit.id);

        if is_branch_head {
            info.push_str(&format!(" ({})", commit.branch));
        }

        lines.push(format!("  {}{}", prefix.trim_end(), info));

        // Show merge lines if this is a merge commit
        if commit.parents.len() > 1 {
            let mut merge_line = String::new();
            for l in 0..max_lanes {
                if l == lane {
                    merge_line.push('├');
                } else {
                    merge_line.push('│');
                }
                merge_line.push(' ');
            }
            // Don't push redundant merge lines for simple cases
        }
    }

    // Legend
    if branch_order.len() > 1 {
        lines.push(String::new());
        lines.push("  Branches:".to_string());
        for (i, branch) in branch_order.iter().enumerate() {
            let marker = BRANCH_CHARS.get(i % BRANCH_CHARS.len()).unwrap_or(&'●');
            lines.push(format!("    {} {}", marker, branch));
        }
    }

    lines.push(String::new());
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_gitgraph() {
        let db = GitGraphDb::new();
        let output = render_gitgraph_tui(&db).unwrap();
        assert!(output.contains("empty git graph"));
    }

    #[test]
    fn gallery_gitgraph_renders() {
        let input = std::fs::read_to_string("docs/sources/git_graph.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Git(db) => db,
            _ => panic!("Expected git graph"),
        };
        let output = render_gitgraph_tui(&db).unwrap();
        // Should have commit markers
        assert!(
            output.contains('●'),
            "Should have commit markers\nOutput:\n{}",
            output
        );
        // Should reference branches
        assert!(
            output.contains("main") || output.contains("feature"),
            "Output:\n{}",
            output
        );
    }

    #[test]
    fn commit_ids_appear() {
        let input = std::fs::read_to_string("docs/sources/git_graph.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Git(db) => db,
            _ => panic!("Expected git graph"),
        };
        let output = render_gitgraph_tui(&db).unwrap();
        // Custom IDs from the sample: A, B, C, D
        for id in &["A", "B", "C", "D"] {
            assert!(
                output.contains(id),
                "Should contain commit ID '{}'\nOutput:\n{}",
                id,
                output
            );
        }
    }

    #[test]
    fn branch_legend_appears() {
        let input = std::fs::read_to_string("docs/sources/git_graph.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Git(db) => db,
            _ => panic!("Expected git graph"),
        };
        let output = render_gitgraph_tui(&db).unwrap();
        assert!(
            output.contains("Branches:"),
            "Should have legend\nOutput:\n{}",
            output
        );
        assert!(
            output.contains("feature"),
            "Should list feature branch\nOutput:\n{}",
            output
        );
    }
}
