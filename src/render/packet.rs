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
            // Mermaid defaults: paddingY: 5 (base)
            padding_y: 5.0,
            // Mermaid defaults: paddingX: 5
            padding_x: 5.0,
            bit_width: 32.0,
            bits_per_row: 32,
            show_bits: true,
        }
    }
}

impl PacketConfig {
    /// Returns the effective padding_y, which increases by 10 when show_bits is true
    /// to make room for the bit number labels above the blocks.
    /// This matches mermaid's behavior in db.ts: `if (config.showBits) { config.paddingY += 10; }`
    pub fn effective_padding_y(&self) -> f64 {
        if self.show_bits {
            self.padding_y + 10.0
        } else {
            self.padding_y
        }
    }
}

/// Render a packet diagram to SVG
pub fn render_packet(db: &PacketDb, config: &RenderConfig) -> Result<String> {
    let mut doc = SvgDocument::new();

    let packet_config = PacketConfig::default();
    let PacketConfig {
        row_height,
        bit_width,
        bits_per_row,
        ..
    } = packet_config;
    // Use effective_padding_y which adds 10 when show_bits is true (like mermaid)
    let padding_y = packet_config.effective_padding_y();

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

    // Collect all shapes and text elements from all words
    // This ensures correct z-order across all rows (all shapes before all text)
    let mut all_rect_elements: Vec<SvgElement> = Vec::new();
    let mut all_text_elements: Vec<SvgElement> = Vec::new();

    for (word_index, word) in words.iter().enumerate() {
        collect_word_elements(
            word,
            word_index,
            &packet_config,
            &mut all_rect_elements,
            &mut all_text_elements,
        );
    }

    // Emit all rect elements first (shapes behind text)
    for elem in all_rect_elements {
        doc.add_element(elem);
    }

    // Emit all text elements second (text on top of shapes)
    for elem in all_text_elements {
        doc.add_element(elem);
    }

    // Render title at the bottom (after all other content)
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

/// Collect elements for a word (row) of packet blocks into shape and text vectors.
/// This allows the caller to batch all shapes before all text for correct z-order.
fn collect_word_elements(
    word: &[crate::diagrams::packet::PacketBlock],
    row_number: usize,
    config: &PacketConfig,
    rect_elements: &mut Vec<SvgElement>,
    text_elements: &mut Vec<SvgElement>,
) {
    let PacketConfig {
        row_height,
        padding_x,
        bit_width,
        bits_per_row,
        show_bits,
        ..
    } = *config;
    // Use effective_padding_y which adds 10 when show_bits is true
    let padding_y = config.effective_padding_y();

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
        rect_elements.push(rect);

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
        text_elements.push(label);

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
            text_elements.push(start_bit);

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
                text_elements.push(end_bit);
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
