//! TUI renderer for sequence diagrams.
//!
//! Produces character-art output with:
//! - Box-drawing participant headers (top and bottom)
//! - Vertical lifelines using `│`
//! - Horizontal message arrows using `─` with `>` or `>>` tips
//! - Message labels centered above arrows
//! - Fragment boxes (loop/alt/opt/par) using box-drawing characters

use crate::diagrams::sequence::{LineType, SequenceDb};
use crate::error::Result;

/// Layout constants for TUI sequence diagrams.
const ACTOR_COL_WIDTH: usize = 20;
const ACTOR_BOX_PADDING: usize = 2;
const MESSAGE_ROW_SPACING: usize = 2;

/// Render a sequence diagram as character art.
pub fn render_sequence_tui(db: &SequenceDb) -> Result<String> {
    let actors = db.get_actors_in_order();
    if actors.is_empty() {
        return Ok(String::new());
    }

    let messages = db.get_messages();

    // Calculate column widths: each actor gets a column wide enough for its label
    let actor_widths: Vec<usize> = actors
        .iter()
        .map(|a| {
            let label_len = a.description.chars().count();
            // Minimum width = label + padding on each side + border chars
            (label_len + ACTOR_BOX_PADDING * 2 + 2).max(ACTOR_COL_WIDTH)
        })
        .collect();

    // Calculate per-gap spacing based on message text widths
    let mut gap_widths: Vec<usize> = vec![4; actors.len().saturating_sub(1)];
    let actor_name_to_idx: std::collections::HashMap<&str, usize> = actors
        .iter()
        .enumerate()
        .map(|(i, a)| (a.name.as_str(), i))
        .collect();
    for msg in messages {
        if let (Some(from), Some(to)) = (&msg.from, &msg.to) {
            if let (Some(&fi), Some(&ti)) = (
                actor_name_to_idx.get(from.as_str()),
                actor_name_to_idx.get(to.as_str()),
            ) {
                if fi != ti {
                    let min_idx = fi.min(ti);
                    let max_idx = fi.max(ti);
                    let needed = msg.message.chars().count() + 4; // label + padding
                    let actor_half = actor_widths[fi] / 2 + actor_widths[ti] / 2;
                    let total_gap_needed = needed.saturating_sub(actor_half).max(4);
                    let num_gaps = max_idx - min_idx;
                    let per_gap = total_gap_needed.div_ceil(num_gaps);
                    for g in min_idx..max_idx {
                        if g < gap_widths.len() {
                            gap_widths[g] = gap_widths[g].max(per_gap);
                        }
                    }
                }
            }
        }
    }

    // Calculate center column for each actor
    let mut actor_centers: Vec<usize> = Vec::with_capacity(actors.len());
    let mut x = 1; // left margin
    for (i, width) in actor_widths.iter().enumerate() {
        actor_centers.push(x + width / 2);
        let gap = if i < gap_widths.len() {
            gap_widths[i]
        } else {
            4
        };
        x += width + gap;
    }

    let total_width = if let Some(last_center) = actor_centers.last() {
        last_center + actor_widths.last().unwrap_or(&ACTOR_COL_WIDTH) / 2 + 2
    } else {
        80
    };

    // Build actor name → index map
    let actor_index: std::collections::HashMap<&str, usize> = actors
        .iter()
        .enumerate()
        .map(|(i, a)| (a.name.as_str(), i))
        .collect();

    // Count message rows needed (skip control structures for row counting)
    let mut row_events: Vec<RowEvent> = Vec::new();
    for msg in messages {
        match msg.message_type {
            LineType::ActiveStart | LineType::ActiveEnd | LineType::Autonumber => continue,
            LineType::LoopStart
            | LineType::AltStart
            | LineType::OptStart
            | LineType::ParStart
            | LineType::CriticalStart
            | LineType::BreakStart
            | LineType::RectStart => {
                row_events.push(RowEvent::FragmentStart(
                    msg.message.clone(),
                    msg.message_type,
                ));
            }
            LineType::AltElse | LineType::ParAnd | LineType::CriticalOption => {
                row_events.push(RowEvent::FragmentDivider(
                    msg.message.clone(),
                    msg.message_type,
                ));
            }
            LineType::LoopEnd
            | LineType::AltEnd
            | LineType::OptEnd
            | LineType::ParEnd
            | LineType::CriticalEnd
            | LineType::BreakEnd
            | LineType::RectEnd => {
                row_events.push(RowEvent::FragmentEnd);
            }
            _ => {
                if msg.from.is_some() && msg.to.is_some() {
                    row_events.push(RowEvent::Message {
                        from: msg.from.as_deref().unwrap_or(""),
                        to: msg.to.as_deref().unwrap_or(""),
                        label: &msg.message,
                        msg_type: msg.message_type,
                    });
                }
            }
        }
    }

    // Canvas: allocate rows
    // Top actor box: 3 rows, then each event gets MESSAGE_ROW_SPACING rows,
    // then bottom actor box: 3 rows
    let content_rows = row_events.len() * MESSAGE_ROW_SPACING;
    let top_box_rows = 3;
    let bottom_box_rows = 3;
    let total_rows = top_box_rows + 1 + content_rows + 1 + bottom_box_rows;

    let mut canvas: Vec<Vec<char>> = vec![vec![' '; total_width]; total_rows];

    // Draw top actor boxes
    for (i, actor) in actors.iter().enumerate() {
        draw_actor_box(
            &mut canvas,
            0,
            actor_centers[i],
            actor_widths[i],
            &actor.description,
        );
    }

    // Draw lifelines (from below top box to above bottom box)
    let lifeline_start = top_box_rows;
    let lifeline_end = total_rows - bottom_box_rows - 1;
    for &center in &actor_centers {
        if center < total_width {
            for canvas_row in canvas
                .iter_mut()
                .take(lifeline_end + 1)
                .skip(lifeline_start)
            {
                if canvas_row[center] == ' ' {
                    canvas_row[center] = '│';
                }
            }
        }
    }

    // Draw messages and fragments
    let mut current_row = top_box_rows + 1;
    let mut fragment_stack: Vec<(usize, String)> = Vec::new(); // (start_row, label)

    for event in &row_events {
        match event {
            RowEvent::Message {
                from,
                to,
                label,
                msg_type,
            } => {
                let from_idx = actor_index.get(from).copied();
                let to_idx = actor_index.get(to).copied();

                if let (Some(fi), Some(ti)) = (from_idx, to_idx) {
                    let from_col = actor_centers[fi];
                    let to_col = actor_centers[ti];
                    let is_dotted = matches!(
                        msg_type,
                        LineType::Dotted
                            | LineType::DottedOpen
                            | LineType::DottedCross
                            | LineType::DottedPoint
                    );
                    let is_self = fi == ti;

                    if is_self {
                        draw_self_message(&mut canvas, current_row, from_col, label, total_width);
                    } else {
                        draw_message(
                            &mut canvas,
                            current_row,
                            from_col,
                            to_col,
                            label,
                            is_dotted,
                            total_width,
                        );
                    }
                }
                current_row += MESSAGE_ROW_SPACING;
            }
            RowEvent::FragmentStart(label, msg_type) => {
                fragment_stack.push((current_row, fragment_prefix(*msg_type).to_string()));
                // Draw fragment header
                let prefix = fragment_prefix(*msg_type);
                let header = if label.is_empty() {
                    format!("[{}]", prefix)
                } else {
                    format!("[{} {}]", prefix, label)
                };
                // Place header at left side
                let col = 0;
                for (j, ch) in header.chars().enumerate() {
                    if col + j < total_width && current_row < canvas.len() {
                        canvas[current_row][col + j] = ch;
                    }
                }
                current_row += MESSAGE_ROW_SPACING;
            }
            RowEvent::FragmentDivider(label, msg_type) => {
                // Draw dashed divider line
                if current_row < canvas.len() {
                    let prefix = fragment_prefix(*msg_type);
                    let divider_label = if label.is_empty() {
                        format!("- - [{}] - -", prefix)
                    } else {
                        format!("- - [{}] - -", label)
                    };
                    for (j, ch) in divider_label.chars().enumerate() {
                        if j < total_width {
                            canvas[current_row][j] = ch;
                        }
                    }
                }
                current_row += MESSAGE_ROW_SPACING;
            }
            RowEvent::FragmentEnd => {
                if let Some((_start_row, _label)) = fragment_stack.pop() {
                    // Draw fragment end marker
                    if current_row < canvas.len() {
                        let end_marker = "[end]";
                        for (j, ch) in end_marker.chars().enumerate() {
                            if j < total_width {
                                canvas[current_row][j] = ch;
                            }
                        }
                    }
                }
                current_row += MESSAGE_ROW_SPACING;
            }
        }
    }

    // Draw bottom actor boxes
    let bottom_row = total_rows - bottom_box_rows;
    for (i, actor) in actors.iter().enumerate() {
        draw_actor_box(
            &mut canvas,
            bottom_row,
            actor_centers[i],
            actor_widths[i],
            &actor.description,
        );
    }

    // Convert canvas to string, trimming trailing empty lines
    let mut result = String::new();
    let mut last_non_empty = 0;
    for (i, row) in canvas.iter().enumerate() {
        if row.iter().any(|&c| c != ' ') {
            last_non_empty = i;
        }
    }

    for row in &canvas[..=last_non_empty] {
        let line: String = row.iter().collect();
        result.push_str(line.trim_end());
        result.push('\n');
    }

    Ok(result)
}

/// Row event types for sequence diagram layout
enum RowEvent<'a> {
    Message {
        from: &'a str,
        to: &'a str,
        label: &'a str,
        msg_type: LineType,
    },
    FragmentStart(String, LineType),
    FragmentDivider(String, LineType),
    FragmentEnd,
}

/// Draw an actor box at the given position
fn draw_actor_box(
    canvas: &mut [Vec<char>],
    start_row: usize,
    center_col: usize,
    width: usize,
    label: &str,
) {
    let half_w = width / 2;
    let left = center_col.saturating_sub(half_w);
    let right = left + width - 1;
    let cols = canvas[0].len();

    if start_row + 2 >= canvas.len() {
        return;
    }

    // Top border: ┌───┐
    if left < cols {
        canvas[start_row][left] = '┌';
    }
    for cell in canvas[start_row]
        .iter_mut()
        .take(right.min(cols))
        .skip(left + 1)
    {
        *cell = '─';
    }
    if right < cols {
        canvas[start_row][right] = '┐';
    }

    // Middle: │ label │
    if left < cols {
        canvas[start_row + 1][left] = '│';
    }
    if right < cols {
        canvas[start_row + 1][right] = '│';
    }
    // Center label in the box
    let label_chars: Vec<char> = label.chars().collect();
    let label_len = label_chars.len();
    let inner_width = right.saturating_sub(left + 1);
    let label_start = left + 1 + inner_width.saturating_sub(label_len) / 2;
    for (j, &ch) in label_chars.iter().enumerate() {
        let col = label_start + j;
        if col < right && col < cols {
            canvas[start_row + 1][col] = ch;
        }
    }

    // Bottom border: └───┘
    if left < cols {
        canvas[start_row + 2][left] = '└';
    }
    for cell in canvas[start_row + 2]
        .iter_mut()
        .take(right.min(cols))
        .skip(left + 1)
    {
        *cell = '─';
    }
    if right < cols {
        canvas[start_row + 2][right] = '┘';
    }
}

/// Draw a message arrow between two actor lifelines
fn draw_message(
    canvas: &mut [Vec<char>],
    row: usize,
    from_col: usize,
    to_col: usize,
    label: &str,
    is_dotted: bool,
    total_width: usize,
) {
    if row >= canvas.len() {
        return;
    }

    let (left, right, going_right) = if from_col < to_col {
        (from_col, to_col, true)
    } else {
        (to_col, from_col, false)
    };

    // Draw arrow line
    let line_char = if is_dotted { '·' } else { '─' };
    for cell in canvas[row]
        .iter_mut()
        .take(right.min(total_width))
        .skip(left + 1)
    {
        *cell = line_char;
    }

    // Arrow tip
    if going_right {
        if right < total_width {
            canvas[row][right] = '>';
        }
    } else if left < total_width {
        canvas[row][left] = '<';
    }

    // Place label above the arrow (centered between from/to, avoiding lifeline columns)
    if !label.is_empty() {
        let label_row = if row > 0 { row - 1 } else { row };
        let mid = (left + right) / 2;
        let label_chars: Vec<char> = label.chars().collect();
        let label_len = label_chars.len();
        // Clamp label to fit between lifelines (left+1 .. right-1)
        let available = right.saturating_sub(left + 2);
        let display_chars = if label_len > available && available > 0 {
            &label_chars[..available]
        } else {
            &label_chars
        };
        let label_start = mid.saturating_sub(display_chars.len() / 2);
        for (j, &ch) in display_chars.iter().enumerate() {
            let col = label_start + j;
            if col < total_width && label_row < canvas.len() {
                canvas[label_row][col] = ch;
            }
        }
    }
}

/// Draw a self-message (loop back to the same actor)
fn draw_self_message(
    canvas: &mut [Vec<char>],
    row: usize,
    col: usize,
    label: &str,
    total_width: usize,
) {
    if row >= canvas.len() {
        return;
    }

    // Draw a small loop: ──┐
    //                      │
    //                   <──┘
    let loop_width = 6;
    let right = (col + loop_width).min(total_width - 1);

    // Top line
    for cell in canvas[row]
        .iter_mut()
        .take((right + 1).min(total_width))
        .skip(col + 1)
    {
        *cell = '─';
    }
    if right < total_width {
        canvas[row][right] = '┐';
    }

    // Vertical
    if row + 1 < canvas.len() && right < total_width {
        canvas[row + 1][right] = '│';
    }

    // Bottom return line: <──┘
    if row + 2 < canvas.len() {
        if col < total_width {
            canvas[row + 2][col] = '<';
        }
        for cell in canvas[row + 2]
            .iter_mut()
            .take(right.min(total_width))
            .skip(col + 1)
        {
            *cell = '─';
        }
        if right < total_width {
            canvas[row + 2][right] = '┘';
        }
    }

    // Place label to the right of the top line
    if !label.is_empty() {
        let label_start = right + 2;
        for (j, ch) in label.chars().enumerate() {
            let c = label_start + j;
            if c < total_width {
                canvas[row][c] = ch;
            }
        }
    }
}

fn fragment_prefix(msg_type: LineType) -> &'static str {
    match msg_type {
        LineType::LoopStart | LineType::LoopEnd => "loop",
        LineType::AltStart | LineType::AltEnd | LineType::AltElse => "alt",
        LineType::OptStart | LineType::OptEnd => "opt",
        LineType::ParStart | LineType::ParEnd | LineType::ParAnd => "par",
        LineType::CriticalStart | LineType::CriticalEnd | LineType::CriticalOption => "critical",
        LineType::BreakStart | LineType::BreakEnd => "break",
        LineType::RectStart | LineType::RectEnd => "rect",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_sequence(input: &str) -> SequenceDb {
        let diagram = crate::parse(input).unwrap();
        match diagram {
            crate::diagrams::Diagram::Sequence(db) => db,
            _ => panic!("Expected sequence diagram"),
        }
    }

    #[test]
    fn renders_two_participants() {
        let db = parse_sequence("sequenceDiagram\n    participant A as Alice\n    participant B as Bob\n    A->>B: Hello");
        let output = render_sequence_tui(&db).unwrap();
        assert!(
            output.contains("Alice"),
            "Should contain Alice, got:\n{}",
            output
        );
        assert!(
            output.contains("Bob"),
            "Should contain Bob, got:\n{}",
            output
        );
    }

    #[test]
    fn renders_message_arrow() {
        let db = parse_sequence("sequenceDiagram\n    A->>B: Hello");
        let output = render_sequence_tui(&db).unwrap();
        // Should have an arrow character
        assert!(
            output.contains('>') || output.contains('─'),
            "Should contain arrow chars, got:\n{}",
            output
        );
    }

    #[test]
    fn renders_message_label() {
        let db = parse_sequence("sequenceDiagram\n    A->>B: Hello Bob");
        let output = render_sequence_tui(&db).unwrap();
        assert!(
            output.contains("Hello Bob"),
            "Should contain message label, got:\n{}",
            output
        );
    }

    #[test]
    fn renders_dotted_message() {
        let db = parse_sequence("sequenceDiagram\n    A-->>B: Response");
        let output = render_sequence_tui(&db).unwrap();
        assert!(
            output.contains("Response"),
            "Should contain dotted message label, got:\n{}",
            output
        );
        assert!(
            output.contains('·'),
            "Should contain dotted line char, got:\n{}",
            output
        );
    }

    #[test]
    fn renders_lifelines() {
        let db = parse_sequence("sequenceDiagram\n    A->>B: Hello\n    B->>A: Hi");
        let output = render_sequence_tui(&db).unwrap();
        // Lifelines use │
        let pipe_count = output.chars().filter(|&c| c == '│').count();
        assert!(
            pipe_count > 4,
            "Should have multiple lifeline chars, got {} in:\n{}",
            pipe_count,
            output
        );
    }

    #[test]
    fn renders_box_drawing_headers() {
        let db = parse_sequence("sequenceDiagram\n    participant A as Alice\n    A->>A: Think");
        let output = render_sequence_tui(&db).unwrap();
        assert!(output.contains('┌'), "Should have box top-left corner");
        assert!(output.contains('┘'), "Should have box bottom-right corner");
    }

    #[test]
    fn empty_diagram() {
        let db = SequenceDb::new();
        let output = render_sequence_tui(&db).unwrap();
        assert!(
            output.is_empty(),
            "Empty diagram should produce empty output"
        );
    }

    #[test]
    fn multiple_messages() {
        let db = parse_sequence(
            "sequenceDiagram\n    A->>B: First\n    B->>A: Second\n    A->>B: Third",
        );
        let output = render_sequence_tui(&db).unwrap();
        assert!(output.contains("First"), "Should contain First");
        assert!(output.contains("Second"), "Should contain Second");
        assert!(output.contains("Third"), "Should contain Third");
    }

    #[test]
    fn three_participants() {
        let db = parse_sequence(
            "sequenceDiagram\n    participant A\n    participant B\n    participant C\n    A->>B: msg1\n    B->>C: msg2",
        );
        let output = render_sequence_tui(&db).unwrap();
        // All three participant lifelines should appear
        assert!(output.contains("A"), "Should contain participant A");
        assert!(output.contains("B"), "Should contain participant B");
        assert!(output.contains("C"), "Should contain participant C");
        assert!(output.contains("msg1"));
        assert!(output.contains("msg2"));
    }

    #[test]
    fn self_message_has_return_arrow() {
        let db = parse_sequence("sequenceDiagram\n    A->>A: Think");
        let output = render_sequence_tui(&db).unwrap();
        assert!(
            output.contains('┐'),
            "Self-message should have top-right corner ┐\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('┘'),
            "Self-message should have bottom-right corner ┘\nOutput:\n{}",
            output
        );
        assert!(
            output.contains('<'),
            "Self-message should have return arrow <\nOutput:\n{}",
            output
        );
    }

    #[test]
    fn participant_ordering_preserved() {
        let db = parse_sequence(
            "sequenceDiagram\n    participant A as Alice\n    participant B as Bob\n    A->>B: Hello",
        );
        let output = render_sequence_tui(&db).unwrap();
        let alice_pos = output.find("Alice").expect("Alice should appear");
        let bob_pos = output.find("Bob").expect("Bob should appear");
        // Alice should appear before Bob (left-to-right) in the first line where they appear
        // They're on the same row in the header, so Alice col < Bob col
        assert!(
            alice_pos < bob_pos,
            "Alice should be left of Bob in first occurrence"
        );
    }
}
