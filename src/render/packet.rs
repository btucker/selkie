//! Packet diagram renderer
//!
//! Renders packet diagrams showing bit-level packet/protocol structure with labeled fields.
//! Based on the mermaid.js packet diagram implementation.

use crate::diagrams::packet::PacketDb;
use crate::error::Result;
use crate::render::svg::{Attrs, RenderConfig, SvgDocument, SvgElement};

/// Configuration for packet diagram rendering
#[derive(Debug, Clone)]
pub struct PacketConfig {
    /// Height of each row in pixels
    pub row_height: f64,
    /// Vertical padding between rows
    pub padding_y: f64,
    /// Horizontal padding within blocks
    pub padding_x: f64,
    /// Width of each bit in pixels
    pub bit_width: f64,
    /// Number of bits per row (default 32)
    pub bits_per_row: usize,
    /// Whether to show bit numbers
    pub show_bits: bool,
}

impl Default for PacketConfig {
    fn default() -> Self {
        Self {
            row_height: 32.0,
            padding_y: 8.0,
            padding_x: 2.0,
            bit_width: 32.0,
            bits_per_row: 32,
            show_bits: true,
        }
    }
}

/// Render a packet diagram to SVG
pub fn render_packet(db: &PacketDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    let packet_config = PacketConfig::default();
    let PacketConfig {
        row_height,
        padding_y,
        bit_width,
        bits_per_row,
        ..
    } = packet_config;

    let words = db.get_packet();
    let title = db.get_title();
    let total_row_height = row_height + padding_y;

    // Calculate SVG dimensions
    let num_rows = words.len();
    let has_title = !title.is_empty();

    // Height calculation: rows + title space
    let svg_height = if num_rows == 0 {
        // Empty diagram - just space for potential title
        if has_title {
            total_row_height * 2.0
        } else {
            total_row_height
        }
    } else {
        total_row_height * (num_rows as f64 + 1.0) - if has_title { 0.0 } else { row_height }
    };

    let svg_width = bit_width * bits_per_row as f64 + 2.0;

    doc.set_size(svg_width, svg_height);

    // Add theme styles
    if config.embed_css {
        doc.add_style(&config.theme.generate_css());
        doc.add_style(&generate_packet_css());
    }

    // Render each word (row)
    for (word_index, word) in words.iter().enumerate() {
        draw_word(&mut doc, word, word_index, &packet_config);
    }

    // Render title at the bottom
    if has_title {
        let title_y = svg_height - total_row_height / 2.0;
        let title_elem = SvgElement::Text {
            x: svg_width / 2.0,
            y: title_y,
            content: title.to_string(),
            attrs: Attrs::new()
                .with_attr("dominant-baseline", "middle")
                .with_attr("text-anchor", "middle")
                .with_class("packetTitle"),
        };
        doc.add_element(title_elem);
    }

    Ok(doc.to_string())
}

/// Draw a word (row) of packet blocks
fn draw_word(
    doc: &mut SvgDocument,
    word: &[crate::diagrams::packet::PacketBlock],
    row_number: usize,
    config: &PacketConfig,
) {
    let PacketConfig {
        row_height,
        padding_y,
        padding_x,
        bit_width,
        bits_per_row,
        show_bits,
    } = *config;

    let word_y = row_number as f64 * (row_height + padding_y) + padding_y;

    for block in word {
        // Calculate block position within the row
        let position_in_row = block.start % bits_per_row;
        let block_x = position_in_row as f64 * bit_width + 1.0;
        let width = block.bits as f64 * bit_width - padding_x;

        // Block rectangle
        let rect = SvgElement::Rect {
            x: block_x,
            y: word_y,
            width,
            height: row_height,
            rx: None,
            ry: None,
            attrs: Attrs::new().with_class("packetBlock"),
        };
        doc.add_element(rect);

        // Block label (centered in the block)
        let label = SvgElement::Text {
            x: block_x + width / 2.0,
            y: word_y + row_height / 2.0,
            content: block.label.clone(),
            attrs: Attrs::new()
                .with_class("packetLabel")
                .with_attr("dominant-baseline", "middle")
                .with_attr("text-anchor", "middle"),
        };
        doc.add_element(label);

        if show_bits {
            let is_single_block = block.end == block.start;
            let bit_number_y = word_y - 2.0;

            // Start bit number
            let start_bit = SvgElement::Text {
                x: block_x + if is_single_block { width / 2.0 } else { 0.0 },
                y: bit_number_y,
                content: block.start.to_string(),
                attrs: Attrs::new()
                    .with_class("packetByte")
                    .with_class("start")
                    .with_attr("dominant-baseline", "auto")
                    .with_attr(
                        "text-anchor",
                        if is_single_block { "middle" } else { "start" },
                    ),
            };
            doc.add_element(start_bit);

            // End bit number (if different from start)
            if !is_single_block {
                let end_bit = SvgElement::Text {
                    x: block_x + width,
                    y: bit_number_y,
                    content: block.end.to_string(),
                    attrs: Attrs::new()
                        .with_class("packetByte")
                        .with_class("end")
                        .with_attr("dominant-baseline", "auto")
                        .with_attr("text-anchor", "end"),
                };
                doc.add_element(end_bit);
            }
        }
    }
}

/// Generate CSS styles for packet diagrams
fn generate_packet_css() -> String {
    r#"
.packetByte {
    font-size: 10px;
}
.packetByte.start {
    fill: black;
}
.packetByte.end {
    fill: black;
}
.packetLabel {
    fill: black;
    font-size: 12px;
}
.packetTitle {
    fill: black;
    font-size: 14px;
}
.packetBlock {
    stroke: black;
    stroke-width: 1;
    fill: #efefef;
}
"#
    .to_string()
}
