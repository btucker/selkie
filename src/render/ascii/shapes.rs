//! Node shape rendering using box-drawing characters.
//!
//! Each shape is rendered into a rectangular grid of characters.
//! Shapes supported:
//! - Rectangle: sharp corners `┌─┐│ │└─┘`
//! - Round: rounded corners `╭─╮│ │╰─╯`
//! - Diamond: `/` and `\` characters
//! - Stadium: same as round (rounded corners)
//! - Others: fallback to rectangle

use crate::layout::NodeShape;

/// A rendered shape as a grid of character rows.
#[derive(Debug, Clone)]
pub struct RenderedShape {
    /// Lines of the rendered shape (each line is the same width, padded with spaces).
    pub lines: Vec<String>,
    /// Width in cells.
    pub width: usize,
    /// Height in cells.
    pub height: usize,
}

/// Render a node shape with the given label text and cell dimensions.
///
/// `cell_w` and `cell_h` are the allocated cell dimensions (from layout scaling).
/// The shape is rendered to fill those dimensions, with the label centered.
pub fn render_shape(shape: &NodeShape, label: &str, cell_w: usize, cell_h: usize) -> RenderedShape {
    // Use character count (not byte length) for proper Unicode label sizing
    let label_chars = label.chars().count();
    // Ensure minimum dimensions for the shape
    let w = cell_w.max(label_chars + 4).max(5);
    let h = cell_h.max(3);

    match shape {
        NodeShape::Rectangle | NodeShape::Subroutine => render_rect(label, w, h, false),
        NodeShape::RoundedRect | NodeShape::Stadium | NodeShape::Ellipse => {
            render_rect(label, w, h, true)
        }
        NodeShape::Diamond => render_diamond(label, w, h),
        NodeShape::Circle => render_circle(label, w, h, false),
        NodeShape::DoubleCircle => render_circle(label, w, h, true),
        NodeShape::HorizontalBar => render_horizontal_bar(cell_w),
        _ => render_rect(label, w, h, false),
    }
}

/// Render a rectangle (sharp or rounded corners) with centered label.
fn render_rect(label: &str, w: usize, h: usize, rounded: bool) -> RenderedShape {
    let (tl, tr, bl, br) = if rounded {
        ('╭', '╮', '╰', '╯')
    } else {
        ('┌', '┐', '└', '┘')
    };

    let inner_w = w.saturating_sub(2);
    let mut lines = Vec::with_capacity(h);

    // Top border
    let top = format!("{}{}{}", tl, "─".repeat(inner_w), tr);
    lines.push(top);

    // Middle rows
    for row in 1..h.saturating_sub(1) {
        let mid_row = h / 2;
        if row == mid_row {
            // Label row — center the label (use char count for Unicode safety)
            let label_chars: String = label.chars().take(inner_w).collect();
            let label_char_count = label_chars.chars().count();
            let pad_total = inner_w.saturating_sub(label_char_count);
            let pad_left = pad_total / 2;
            let pad_right = pad_total - pad_left;
            let line = format!(
                "│{}{}{}│",
                " ".repeat(pad_left),
                label_chars,
                " ".repeat(pad_right)
            );
            lines.push(line);
        } else {
            lines.push(format!("│{}│", " ".repeat(inner_w)));
        }
    }

    // Bottom border
    let bot = format!("{}{}{}", bl, "─".repeat(inner_w), br);
    lines.push(bot);

    RenderedShape {
        width: w,
        height: lines.len(),
        lines,
    }
}

/// Render a diamond shape with centered label.
fn render_diamond(label: &str, _w: usize, _h: usize) -> RenderedShape {
    // Diamond needs to be wide enough for the label at its widest point.
    // The widest row (middle) has the form: /  label  \
    // So the total width = label.len() + 4 (for / \ and spacing), rounded up to odd.
    let inner_w = label.chars().count() + 2; // space on each side of label
    let half = inner_w.div_ceil(2) + 1; // half-height
    let full_w = half * 2; // total width (even is fine)
    let full_h = half * 2 + 1; // total height (odd for symmetry)
    let mid = full_h / 2;

    let mut lines = Vec::with_capacity(full_h);

    for row in 0..full_h {
        let dist = mid.abs_diff(row);

        if row == 0 {
            // Top point: /\
            lines.push(format!("{}/\\", " ".repeat(mid)));
        } else if row == full_h - 1 {
            // Bottom point: \/
            lines.push(format!("{}\\/", " ".repeat(mid)));
        } else if row == mid {
            // Widest row with label
            let content_w = full_w.saturating_sub(2);
            let label_display: String = label.chars().take(content_w).collect();
            let label_char_count = label_display.chars().count();
            let pad_total = content_w.saturating_sub(label_char_count);
            let pl = pad_total / 2;
            let pr = pad_total - pl;
            lines.push(format!(
                "/{}{}{}\\",
                " ".repeat(pl),
                label_display,
                " ".repeat(pr)
            ));
        } else if row < mid {
            // Upper half: expanding
            let outer = dist;
            let inner = full_w.saturating_sub(2 * outer).saturating_sub(2);
            lines.push(format!("{}/{}\\", " ".repeat(outer), " ".repeat(inner)));
        } else {
            // Lower half: contracting
            let outer = dist;
            let inner = full_w.saturating_sub(2 * outer).saturating_sub(2);
            lines.push(format!("{}\\{}/", " ".repeat(outer), " ".repeat(inner)));
        }
    }

    // Pad all lines to the same width (use char count for Unicode safety)
    let max_w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    for line in &mut lines {
        let cur = line.chars().count();
        if cur < max_w {
            line.push_str(&" ".repeat(max_w - cur));
        }
    }

    RenderedShape {
        width: max_w,
        height: lines.len(),
        lines,
    }
}

/// Render a horizontal bar for fork/join states.
///
/// Fork and join nodes in state diagrams are rendered as solid horizontal bars,
/// matching the UML convention. Uses block characters for a filled appearance.
fn render_horizontal_bar(cell_w: usize) -> RenderedShape {
    let w = cell_w.max(8);
    let bar = "█".repeat(w);
    RenderedShape {
        lines: vec![bar],
        width: w,
        height: 1,
    }
}

/// Render a circle (start) or double-circle (end) node.
///
/// When no label is provided (state diagram start/end markers), renders as a
/// single `●` or `◉` symbol. When a label is provided (e.g., flowchart
/// `((...))` nodes), renders as a rounded rectangle — matching the visual
/// convention of elliptical shapes in ASCII.
fn render_circle(label: &str, w: usize, h: usize, double: bool) -> RenderedShape {
    if label.is_empty() {
        let symbol = if double { '◉' } else { '●' };
        RenderedShape {
            lines: vec![format!(" {} ", symbol)],
            width: 3,
            height: 1,
        }
    } else {
        // Labelled circles use rounded rectangle (same as Ellipse/Stadium)
        render_rect(label, w, h, true)
    }
}

/// Render a class diagram box with sections: annotations, name, members, methods.
///
/// The box uses ├─┤ dividers between sections. Empty sections (no members or
/// no methods) are omitted, but the divider between members and methods is
/// always drawn when both exist.
pub fn render_class_box(
    name: &str,
    annotations: &[&str],
    members: &[&str],
    methods: &[&str],
    cell_w: usize,
    cell_h: usize,
) -> RenderedShape {
    // Compute minimum width from content (use char count for UTF-8 safety)
    let mut max_content = name.chars().count();
    for a in annotations {
        let text = format!("«{}»", a);
        max_content = max_content.max(text.chars().count());
    }
    for m in members {
        max_content = max_content.max(m.chars().count());
    }
    for m in methods {
        max_content = max_content.max(m.chars().count());
    }

    let w = cell_w.max(max_content + 4).max(5);
    let inner_w = w.saturating_sub(2);

    let mut lines: Vec<String> = Vec::new();

    // Top border
    lines.push(format!("┌{}┐", "─".repeat(inner_w)));

    // Annotations (e.g., «interface»)
    for a in annotations {
        let text = format!("«{}»", a);
        lines.push(center_in_box(&text, inner_w));
    }

    // Class name
    lines.push(center_in_box(name, inner_w));

    // Members section
    let has_members = !members.is_empty();
    let has_methods = !methods.is_empty();

    if has_members || has_methods {
        // Divider before members
        lines.push(format!("├{}┤", "─".repeat(inner_w)));
    }

    for m in members {
        lines.push(left_align_in_box(m, inner_w));
    }

    if has_members && has_methods {
        // Divider between members and methods
        lines.push(format!("├{}┤", "─".repeat(inner_w)));
    }

    for m in methods {
        lines.push(left_align_in_box(m, inner_w));
    }

    // Bottom border
    lines.push(format!("└{}┘", "─".repeat(inner_w)));

    // Pad to desired height if needed
    let _ = cell_h; // class boxes are sized by content, not forced height

    RenderedShape {
        width: w,
        height: lines.len(),
        lines,
    }
}

fn center_in_box(text: &str, inner_w: usize) -> String {
    let char_count = text.chars().count();
    let display: String = if char_count > inner_w {
        text.chars().take(inner_w).collect()
    } else {
        text.to_string()
    };
    let display_len = display.chars().count();
    let pad_total = inner_w.saturating_sub(display_len);
    let pad_left = pad_total / 2;
    let pad_right = pad_total - pad_left;
    format!(
        "│{}{}{}│",
        " ".repeat(pad_left),
        display,
        " ".repeat(pad_right)
    )
}

fn left_align_in_box(text: &str, inner_w: usize) -> String {
    let char_count = text.chars().count();
    let display: String = if char_count > inner_w.saturating_sub(1) {
        text.chars().take(inner_w.saturating_sub(1)).collect()
    } else {
        text.to_string()
    };
    let display_len = display.chars().count();
    let pad = inner_w.saturating_sub(display_len + 1);
    format!("│ {}{}│", display, " ".repeat(pad))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_basic_shape() {
        let shape = render_shape(&NodeShape::Rectangle, "A", 7, 3);
        assert_eq!(shape.lines.len(), 3);
        assert_eq!(shape.lines[0], "┌─────┐");
        assert!(shape.lines[1].contains("A"));
        assert_eq!(shape.lines[2], "└─────┘");
    }

    #[test]
    fn rect_label_centered() {
        let shape = render_shape(&NodeShape::Rectangle, "Hi", 8, 3);
        // inner width = 6, "Hi" is 2 chars → 2 left pad, 2 right pad
        assert_eq!(shape.lines[1], "│  Hi  │");
    }

    #[test]
    fn rounded_rect_corners() {
        let shape = render_shape(&NodeShape::RoundedRect, "X", 5, 3);
        assert!(shape.lines[0].starts_with('╭'));
        assert!(shape.lines[0].ends_with('╮'));
        assert!(shape.lines[2].starts_with('╰'));
        assert!(shape.lines[2].ends_with('╯'));
    }

    #[test]
    fn stadium_uses_rounded() {
        let shape = render_shape(&NodeShape::Stadium, "ok", 6, 3);
        assert!(shape.lines[0].starts_with('╭'));
    }

    #[test]
    fn diamond_contains_label() {
        let shape = render_shape(&NodeShape::Diamond, "yes", 10, 5);
        let has_label = shape.lines.iter().any(|l| l.contains("yes"));
        assert!(has_label, "Diamond should contain label 'yes'");
    }

    #[test]
    fn diamond_top_and_bottom() {
        let shape = render_shape(&NodeShape::Diamond, "D", 10, 5);
        // Top should have /\ and bottom should have \/
        assert!(shape.lines[0].contains("/\\"), "Top should have /\\");
        let last = shape.lines.len() - 1;
        assert!(shape.lines[last].contains("\\/"), "Bottom should have \\/");
    }

    #[test]
    fn minimum_dimensions_enforced() {
        let shape = render_shape(&NodeShape::Rectangle, "Hello", 1, 1);
        // Should be at least 5 wide and 3 tall
        assert!(shape.width >= 5);
        assert!(shape.height >= 3);
        assert!(shape.lines[1].contains("Hello"));
    }

    #[test]
    fn fallback_uses_rectangle() {
        // Hexagon falls back to rectangle
        let shape = render_shape(&NodeShape::Hexagon, "hex", 7, 3);
        assert_eq!(shape.lines[0], "┌─────┐");
    }

    #[test]
    fn circle_renders_filled_dot() {
        let shape = render_shape(&NodeShape::Circle, "", 3, 3);
        let all_text: String = shape.lines.iter().flat_map(|l| l.chars()).collect();
        assert!(
            all_text.contains('●'),
            "Circle should render as ● (filled circle)\nLines: {:?}",
            shape.lines
        );
    }

    #[test]
    fn double_circle_renders_bullseye() {
        let shape = render_shape(&NodeShape::DoubleCircle, "", 3, 3);
        let all_text: String = shape.lines.iter().flat_map(|l| l.chars()).collect();
        assert!(
            all_text.contains('◉'),
            "DoubleCircle should render as ◉ (bullseye)\nLines: {:?}",
            shape.lines
        );
    }

    #[test]
    fn taller_rect_has_empty_rows() {
        let shape = render_shape(&NodeShape::Rectangle, "T", 5, 5);
        assert_eq!(shape.height, 5);
        // Middle row (row 2) should have the label
        assert!(shape.lines[2].contains("T"));
        // Rows 1 and 3 should be blank interior
        assert_eq!(shape.lines[1], "│   │");
        assert_eq!(shape.lines[3], "│   │");
    }

    #[test]
    fn class_box_name_only() {
        let shape = render_class_box("Animal", &[], &[], &[], 10, 5);
        assert!(shape.lines[0].starts_with('┌'));
        assert!(shape.lines.iter().any(|l| l.contains("Animal")));
        assert!(shape.lines.last().unwrap().starts_with('└'));
    }

    #[test]
    fn class_box_with_members_and_methods() {
        let shape = render_class_box(
            "Duck",
            &[],
            &["+String beakColor"],
            &["+swim()", "+quack()"],
            20,
            10,
        );
        let text = shape.lines.join("\n");
        assert!(text.contains("Duck"), "Should contain class name");
        assert!(text.contains("+String beakColor"), "Should contain member");
        assert!(text.contains("+swim()"), "Should contain method");
        // Should have dividers (├─┤)
        let dividers = shape.lines.iter().filter(|l| l.contains('├')).count();
        assert!(
            dividers >= 2,
            "Should have dividers between sections, got {}",
            dividers
        );
    }

    #[test]
    fn class_box_with_annotation() {
        let shape = render_class_box("Flyable", &["interface"], &[], &["+fly()"], 15, 5);
        let text = shape.lines.join("\n");
        assert!(text.contains("«interface»"), "Should contain annotation");
        assert!(text.contains("Flyable"), "Should contain name");
        assert!(text.contains("+fly()"), "Should contain method");
    }

    #[test]
    fn horizontal_bar_renders_solid() {
        let shape = render_shape(&NodeShape::HorizontalBar, "", 10, 1);
        assert_eq!(shape.height, 1, "Bar should be 1 row tall");
        assert!(shape.width >= 8, "Bar should be at least 8 chars wide");
        assert!(
            shape.lines[0].chars().all(|c| c == '█'),
            "Bar should be solid block characters, got: {:?}",
            shape.lines[0]
        );
    }

    #[test]
    fn horizontal_bar_no_label() {
        let shape = render_shape(&NodeShape::HorizontalBar, "fork_state", 10, 1);
        assert!(
            !shape.lines[0].contains("fork"),
            "Bar should not contain label text"
        );
    }

    #[test]
    fn class_box_members_only() {
        let shape = render_class_box("Config", &[], &["+host", "+port"], &[], 12, 5);
        let text = shape.lines.join("\n");
        assert!(text.contains("Config"));
        assert!(text.contains("+host"));
        assert!(text.contains("+port"));
        // Only one divider (before members)
        let dividers = shape.lines.iter().filter(|l| l.contains('├')).count();
        assert_eq!(dividers, 1, "Should have one divider before members");
    }
}
