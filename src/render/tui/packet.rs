//! TUI renderer for packet/bit-field diagrams.
//!
//! Renders packet words as rows of labeled bit fields with bit-position
//! markers, using box-drawing characters for field boundaries.

use crate::diagrams::packet::PacketDb;
use crate::error::Result;

/// Width in characters per bit (including borders).
const CHARS_PER_BIT: usize = 3;

/// Render a packet diagram as character art.
pub fn render_packet_tui(db: &PacketDb) -> Result<String> {
    let packet = db.get_packet();
    if packet.is_empty() {
        let title = db.get_title();
        if !title.is_empty() {
            return Ok(format!("{}\n\n(empty packet)\n", title));
        }
        return Ok("(empty packet)\n".to_string());
    }

    let mut lines: Vec<String> = Vec::new();

    // Title
    let title = db.get_title();
    if !title.is_empty() {
        lines.push(title.to_string());
        lines.push(String::new());
    }

    for word in packet {
        if word.is_empty() {
            continue;
        }

        // Bit position header
        let mut bit_header = String::new();
        for block in word.iter() {
            let bits = block.bits;
            let field_width = bits * CHARS_PER_BIT;
            let pos_str = format!("{}", block.start);
            bit_header.push_str(&format!("{:<width$}", pos_str, width = field_width));
        }
        lines.push(format!("  {}", bit_header.trim_end()));

        // Top border
        let mut top = String::from("  ");
        for block in word.iter() {
            let field_width = block.bits * CHARS_PER_BIT;
            top.push('┌');
            top.push_str(&"─".repeat(field_width.saturating_sub(1)));
        }
        // Replace internal ┌ with ┬ (all except first)
        let top = replace_internal_corners(&top, '┌', '┬');
        let top = format!("{}┐", top);
        lines.push(top);

        // Label row
        let mut label_row = String::from("  ");
        for block in word.iter() {
            let field_width = block.bits * CHARS_PER_BIT;
            let inner_w = field_width.saturating_sub(1);
            let label = &block.label;
            let label_len = label.chars().count();
            if label_len >= inner_w {
                let truncated: String = label.chars().take(inner_w.saturating_sub(1)).collect();
                label_row.push('│');
                label_row.push_str(&truncated);
                label_row.push('…');
                let remaining = inner_w.saturating_sub(truncated.chars().count() + 1);
                label_row.push_str(&" ".repeat(remaining));
            } else {
                let pad_total = inner_w.saturating_sub(label_len);
                let pad_left = pad_total / 2;
                let pad_right = pad_total - pad_left;
                label_row.push('│');
                label_row.push_str(&" ".repeat(pad_left));
                label_row.push_str(label);
                label_row.push_str(&" ".repeat(pad_right));
            }
        }
        label_row.push('│');
        lines.push(label_row);

        // Bottom border
        let mut bottom = String::from("  ");
        for block in word.iter() {
            let field_width = block.bits * CHARS_PER_BIT;
            bottom.push('└');
            bottom.push_str(&"─".repeat(field_width.saturating_sub(1)));
        }
        let bottom = replace_internal_corners(&bottom, '└', '┴');
        let bottom = format!("{}┘", bottom);
        lines.push(bottom);

        lines.push(String::new());
    }

    Ok(lines.join("\n"))
}

/// Replace all occurrences of `from` with `to` except the first one after whitespace.
fn replace_internal_corners(s: &str, from: char, to: char) -> String {
    let mut result = String::new();
    let mut found_first = false;
    for ch in s.chars() {
        if ch == from {
            if !found_first {
                found_first = true;
                result.push(ch);
            } else {
                result.push(to);
            }
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_packet() {
        let db = PacketDb::new();
        let output = render_packet_tui(&db).unwrap();
        assert!(output.contains("empty packet"));
    }

    #[test]
    fn gallery_packet_renders() {
        let input = std::fs::read_to_string("docs/sources/packet.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Packet(db) => db,
            _ => panic!("Expected packet"),
        };
        let output = render_packet_tui(&db).unwrap();
        assert!(output.contains("Header"), "Output:\n{}", output);
        assert!(output.contains("Length"), "Output:\n{}", output);
        assert!(output.contains("Data"), "Output:\n{}", output);
    }

    #[test]
    fn packet_has_box_drawing() {
        let input = std::fs::read_to_string("docs/sources/packet.mmd").unwrap();
        let diagram = crate::parse(&input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Packet(db) => db,
            _ => panic!("Expected packet"),
        };
        let output = render_packet_tui(&db).unwrap();
        assert!(output.contains('┌'), "Output:\n{}", output);
        assert!(output.contains('┘'), "Output:\n{}", output);
    }

    #[test]
    fn title_appears() {
        let input = "packet\n    title Test Packet\n    0-7: \"Byte\"";
        let diagram = crate::parse(input).unwrap();
        let db = match diagram {
            crate::diagrams::Diagram::Packet(db) => db,
            _ => panic!("Expected packet"),
        };
        let output = render_packet_tui(&db).unwrap();
        assert!(output.contains("Test Packet"), "Output:\n{}", output);
    }
}
