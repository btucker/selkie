//! Size estimation for layout

use super::adapter::{NodeSizeConfig, SizeEstimator};
use super::types::NodeShape;

/// Character-width based size estimator
///
/// This estimator uses average character widths to approximate text dimensions
/// without requiring a rendering context. It's suitable for layout purposes
/// where exact pixel-perfect sizing isn't critical.
#[derive(Debug, Clone)]
pub struct CharacterSizeEstimator {
    /// Average character width ratio (relative to font size)
    pub char_width_ratio: f64,
    /// Line height ratio (relative to font size)
    pub line_height_ratio: f64,
}

impl Default for CharacterSizeEstimator {
    fn default() -> Self {
        Self {
            // Approximate ratio for proportional fonts like trebuchet ms
            // Calibrated to match mermaid.js foreignObject text rendering
            // Mermaid.js uses actual browser getBBox which varies by font/platform
            char_width_ratio: 0.6,
            // HTML text in foreignObject has ~2.3x line-height due to
            // default line-height:1.5 plus <p> element margins
            line_height_ratio: 2.3,
        }
    }
}

impl CharacterSizeEstimator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an estimator optimized for monospace fonts
    pub fn monospace() -> Self {
        Self {
            char_width_ratio: 0.6,
            line_height_ratio: 1.2,
        }
    }
}

impl SizeEstimator for CharacterSizeEstimator {
    fn estimate_text_size(&self, text: &str, font_size: f64) -> (f64, f64) {
        if text.is_empty() {
            return (0.0, font_size * self.line_height_ratio);
        }

        // Normalize <br> variants to newlines for proper line counting
        let normalized = crate::render::text_utils::normalize_br_tags(text);

        let lines: Vec<&str> = normalized.lines().collect();
        let max_chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let num_lines = lines.len().max(1);

        let width = (max_chars as f64) * font_size * self.char_width_ratio;
        let height = (num_lines as f64) * font_size * self.line_height_ratio;

        (width, height)
    }

    fn estimate_node_size(
        &self,
        label: Option<&str>,
        shape: NodeShape,
        config: &NodeSizeConfig,
    ) -> (f64, f64) {
        // Calculate text dimensions
        let (text_width, text_height) = label
            .map(|l| self.estimate_text_size(l, config.font_size))
            .unwrap_or((0.0, 0.0));

        // Add padding
        let base_width = text_width + config.padding_horizontal * 2.0;
        let base_height = text_height + config.padding_vertical * 2.0;

        // Apply shape-specific adjustments
        let (width, height) = match shape {
            NodeShape::Circle | NodeShape::DoubleCircle => {
                // Circle sizing per mermaid.js circle.ts:
                // radius = bbox.width / 2 + halfPadding (where halfPadding = node.padding / 2 = 4)
                // diameter = bbox.width + padding (effectively text_width + 8)
                let half_padding = config.padding_vertical / 2.0; // 4, matches mermaid's halfPadding
                let diameter = text_width.max(text_height) + half_padding * 2.0;
                (diameter, diameter)
            }
            NodeShape::Diamond => {
                // Diamond is a square rotated 45 degrees, matching mermaid.js question.ts:
                // mermaid.js uses single padding (node.padding = 8) for diamonds, not double
                // w = bbox.width + padding, h = bbox.height + padding, s = w + h
                let single_padding = config.padding_vertical; // 8, matches mermaid's node.padding
                let w = text_width + single_padding;
                let h = text_height + single_padding;
                let s = w + h;
                (s, s)
            }
            NodeShape::Hexagon => {
                // Hexagon needs extra horizontal space for angled sides
                (base_width * 1.2, base_height)
            }
            NodeShape::Ellipse => {
                // Ellipse needs slightly more space
                (base_width * 1.1, base_height * 1.1)
            }
            NodeShape::Stadium => {
                // Stadium (pill shape) needs extra width for rounded ends
                (base_width + base_height, base_height)
            }
            NodeShape::Cylinder => {
                // Cylinder needs extra height for 3D cap
                (base_width, base_height * 1.3)
            }
            NodeShape::Trapezoid | NodeShape::InvTrapezoid => {
                // Trapezoid needs extra width for angled sides
                (base_width * 1.2, base_height)
            }
            NodeShape::LeanRight | NodeShape::LeanLeft => {
                // Parallelogram needs extra width
                (base_width * 1.2, base_height)
            }
            NodeShape::Subroutine => {
                // Subroutine has extra side bars
                (base_width + 20.0, base_height)
            }
            NodeShape::Odd => {
                // Odd shape (flag-like) - asymmetric
                (base_width * 1.1, base_height)
            }
            NodeShape::HorizontalBar => {
                // Fork/join bar: fixed dimensions, ignore text
                (70.0, 10.0)
            }
            NodeShape::Rectangle => {
                // Standard rectangle - no adjustment needed
                (base_width, base_height)
            }
            NodeShape::RoundedRect => {
                // roundedRect.ts uses half the horizontal padding of squareRect.ts
                (text_width + config.padding_horizontal, base_height)
            }
        };

        // Apply min/max constraints
        // For shapes that must be square (circle, diamond), use max of both constraints
        let (final_width, final_height) = match shape {
            NodeShape::Circle | NodeShape::DoubleCircle | NodeShape::Diamond => {
                let min_dim = config.min_width.max(config.min_height);
                let dim = width.max(min_dim);
                (dim, dim)
            }
            _ => {
                let w = width.max(config.min_width);
                let h = height.max(config.min_height);
                (w, h)
            }
        };
        let final_width = config
            .max_width
            .map(|max| final_width.min(max))
            .unwrap_or(final_width);

        (final_width, final_height)
    }
}

/// Font-based size estimator using fontdue for accurate text measurement
///
/// This estimator uses actual font metrics to calculate text dimensions,
/// matching browser getBBox() behavior for better visual parity with mermaid.js.
#[derive(Debug)]
pub struct FontdueSizeEstimator {
    /// The loaded font for text measurement
    font: fontdue::Font,
    /// Line height ratio (relative to font size)
    line_height_ratio: f64,
}

impl FontdueSizeEstimator {
    /// Create a new estimator from font data
    pub fn from_bytes(font_data: &[u8]) -> Result<Self, &'static str> {
        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())?;
        Ok(Self {
            font,
            line_height_ratio: 1.5, // Standard HTML line-height
        })
    }

    /// Try to create an estimator by loading a system font
    ///
    /// Attempts to find and load fonts in this order:
    /// 1. DejaVu Sans (common on Linux)
    /// 2. Arial (common on Windows/Mac)
    /// 3. Helvetica (common on Mac)
    pub fn try_system_font() -> Option<Self> {
        // Common font paths by platform
        let font_paths = [
            // Linux
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
            // Mac
            "/System/Library/Fonts/Helvetica.ttc",
            "/Library/Fonts/Arial.ttf",
            // Windows
            "C:\\Windows\\Fonts\\arial.ttf",
            "C:\\Windows\\Fonts\\verdana.ttf",
        ];

        for path in font_paths {
            if let Ok(data) = std::fs::read(path) {
                if let Ok(estimator) = Self::from_bytes(&data) {
                    return Some(estimator);
                }
            }
        }
        None
    }

    /// Measure text width using font metrics
    fn measure_text_width(&self, text: &str, font_size: f64) -> f64 {
        let px = font_size as f32;
        text.chars()
            .map(|c| {
                let metrics = self.font.metrics(c, px);
                metrics.advance_width as f64
            })
            .sum()
    }
}

impl SizeEstimator for FontdueSizeEstimator {
    fn estimate_text_size(&self, text: &str, font_size: f64) -> (f64, f64) {
        if text.is_empty() {
            return (0.0, font_size * self.line_height_ratio);
        }

        // Normalize <br> variants to newlines for proper line counting
        let normalized = crate::render::text_utils::normalize_br_tags(text);

        let lines: Vec<&str> = normalized.lines().collect();
        let num_lines = lines.len().max(1);

        // Measure actual width of each line using font metrics
        let width = lines
            .iter()
            .map(|line| self.measure_text_width(line, font_size))
            .fold(0.0_f64, |max, w| max.max(w));

        let height = (num_lines as f64) * font_size * self.line_height_ratio;

        (width, height)
    }

    fn estimate_node_size(
        &self,
        label: Option<&str>,
        shape: NodeShape,
        config: &NodeSizeConfig,
    ) -> (f64, f64) {
        // Calculate text dimensions using font metrics
        let (text_width, text_height) = label
            .map(|l| self.estimate_text_size(l, config.font_size))
            .unwrap_or((0.0, 0.0));

        // Add padding
        let base_width = text_width + config.padding_horizontal * 2.0;
        let base_height = text_height + config.padding_vertical * 2.0;

        // Apply shape-specific adjustments (same as CharacterSizeEstimator)
        let (width, height) = match shape {
            NodeShape::Circle | NodeShape::DoubleCircle => {
                let half_padding = config.padding_vertical / 2.0;
                let diameter = text_width.max(text_height) + half_padding * 2.0;
                (diameter, diameter)
            }
            NodeShape::Diamond => {
                let single_padding = config.padding_vertical;
                let w = text_width + single_padding;
                let h = text_height + single_padding;
                let s = w + h;
                (s, s)
            }
            NodeShape::Hexagon => (base_width * 1.2, base_height),
            NodeShape::Ellipse => (base_width * 1.1, base_height * 1.1),
            NodeShape::Stadium => (base_width + base_height, base_height),
            NodeShape::Cylinder => (base_width, base_height * 1.3),
            NodeShape::Trapezoid | NodeShape::InvTrapezoid => (base_width * 1.2, base_height),
            NodeShape::LeanRight | NodeShape::LeanLeft => (base_width * 1.2, base_height),
            NodeShape::Subroutine => (base_width + 20.0, base_height),
            NodeShape::Odd => (base_width * 1.1, base_height),
            NodeShape::HorizontalBar => (70.0, 10.0),
            NodeShape::Rectangle => (base_width, base_height),
            // roundedRect.ts uses half the horizontal padding of squareRect.ts
            NodeShape::RoundedRect => (text_width + config.padding_horizontal, base_height),
        };

        // Apply min/max constraints
        let (final_width, final_height) = match shape {
            NodeShape::Circle | NodeShape::DoubleCircle | NodeShape::Diamond => {
                let min_dim = config.min_width.max(config.min_height);
                let dim = width.max(min_dim);
                (dim, dim)
            }
            _ => {
                let w = width.max(config.min_width);
                let h = height.max(config.min_height);
                (w, h)
            }
        };
        let final_width = config
            .max_width
            .map(|max| final_width.min(max))
            .unwrap_or(final_width);

        (final_width, final_height)
    }
}

/// Per-character advance widths (in em, at 1em font size) for 'trebuchet ms',
/// the default mermaid.js flowchart font. Covers ASCII 0x20..=0x7E.
///
/// Derived from the Trebuchet MS font metrics (unitsPerEm = 2048) and
/// validated against label bboxes measured from mermaid.js reference SVGs
/// (e.g. 'Square Rect' at 16px -> 85.1875px).
#[rustfmt::skip]
const TREBUCHET_ADVANCE_WIDTHS: [f64; 95] = [
    0.301270, // ' '
    0.367188, // '!'
    0.324707, // '"'
    0.524414, // '#'
    0.524414, // '$'
    0.600098, // '%'
    0.706055, // '&'
    0.159668, // '\''
    0.367188, // '('
    0.367188, // ')'
    0.367188, // '*'
    0.524414, // '+'
    0.367188, // ','
    0.367188, // '-'
    0.367188, // '.'
    0.524414, // '/'
    0.524414, // '0'
    0.524414, // '1'
    0.524414, // '2'
    0.524414, // '3'
    0.524414, // '4'
    0.524414, // '5'
    0.524414, // '6'
    0.524414, // '7'
    0.524414, // '8'
    0.524414, // '9'
    0.367188, // ':'
    0.367188, // ';'
    0.524414, // '<'
    0.524414, // '='
    0.524414, // '>'
    0.367188, // '?'
    0.770508, // '@'
    0.589844, // 'A'
    0.565918, // 'B'
    0.598145, // 'C'
    0.613281, // 'D'
    0.535645, // 'E'
    0.524902, // 'F'
    0.676270, // 'G'
    0.654297, // 'H'
    0.278320, // 'I'
    0.476562, // 'J'
    0.575684, // 'K'
    0.506348, // 'L'
    0.709473, // 'M'
    0.638184, // 'N'
    0.673828, // 'O'
    0.557617, // 'P'
    0.675781, // 'Q'
    0.582031, // 'R'
    0.480957, // 'S'
    0.580566, // 'T'
    0.648438, // 'U'
    0.587402, // 'V'
    0.852051, // 'W'
    0.556641, // 'X'
    0.570312, // 'Y'
    0.550293, // 'Z'
    0.367188, // '['
    0.355469, // '\\'
    0.367188, // ']'
    0.524414, // '^'
    0.524414, // '_'
    0.524414, // '`'
    0.525391, // 'a'
    0.557129, // 'b'
    0.495117, // 'c'
    0.557129, // 'd'
    0.545410, // 'e'
    0.369629, // 'f'
    0.501953, // 'g'
    0.546387, // 'h'
    0.285156, // 'i'
    0.366699, // 'j'
    0.504395, // 'k'
    0.294922, // 'l'
    0.830078, // 'm'
    0.546387, // 'n'
    0.536621, // 'o'
    0.557129, // 'p'
    0.557129, // 'q'
    0.388672, // 'r'
    0.404785, // 's'
    0.396484, // 't'
    0.546387, // 'u'
    0.489746, // 'v'
    0.744141, // 'w'
    0.500977, // 'x'
    0.493164, // 'y'
    0.474609, // 'z'
    0.367188, // '{'
    0.524414, // '|'
    0.367188, // '}'
    0.524414, // '~'
];

/// Kerning adjustments (in em) for ASCII pairs in Trebuchet MS.
/// Browsers apply kerning when measuring HTML labels, so these are needed
/// to match mermaid's getBoundingClientRect-based label bboxes.
/// Sorted by (left, right) byte pair for binary search.
#[rustfmt::skip]
const TREBUCHET_KERN_PAIRS: [(u8, u8, f64); 110] = [
    (b' ', b'A', -0.055176), (b' ', b'T', -0.018066), (b' ', b'Y', -0.018066),
    (b'A', b' ', -0.055176), (b'A', b'T', -0.097168), (b'A', b'V', -0.087891),
    (b'A', b'W', -0.087891), (b'A', b'Y', -0.106445), (b'A', b'v', -0.055176),
    (b'A', b'w', -0.045898), (b'A', b'y', -0.041016), (b'F', b',', -0.180176),
    (b'F', b'.', -0.180176), (b'F', b'A', -0.105957), (b'K', b'e', -0.031250),
    (b'K', b'i', -0.031250), (b'K', b'n', -0.031250), (b'K', b'o', -0.031250),
    (b'K', b'u', -0.031250), (b'K', b'w', -0.031250), (b'L', b' ', -0.037109),
    (b'L', b'T', -0.102051), (b'L', b'V', -0.138672), (b'L', b'W', -0.125000),
    (b'L', b'Y', -0.129395), (b'L', b'y', -0.083008), (b'P', b' ', -0.018066),
    (b'P', b',', -0.195312), (b'P', b'.', -0.195312), (b'P', b'A', -0.111328),
    (b'P', b'a', -0.046875), (b'P', b'e', -0.046875), (b'P', b'h', -0.046875),
    (b'P', b'i', -0.046875), (b'P', b'o', -0.046875), (b'P', b'r', -0.046875),
    (b'R', b'T', -0.041016), (b'R', b'V', -0.045898), (b'R', b'W', -0.063965),
    (b'R', b'Y', -0.063965), (b'R', b'e', -0.040527), (b'R', b'o', -0.040527),
    (b'R', b'u', -0.028809), (b'T', b' ', -0.018066), (b'T', b',', -0.166016),
    (b'T', b'-', -0.096680), (b'T', b'.', -0.166016), (b'T', b':', -0.110840),
    (b'T', b';', -0.110840), (b'T', b'A', -0.097168), (b'T', b'O', -0.055176),
    (b'T', b'a', -0.124512), (b'T', b'c', -0.124512), (b'T', b'e', -0.124512),
    (b'T', b'i', -0.041504), (b'T', b'o', -0.124512), (b'T', b'r', -0.109863),
    (b'T', b's', -0.120117), (b'T', b'u', -0.129395), (b'T', b'w', -0.138184),
    (b'T', b'y', -0.115234), (b'V', b',', -0.146484), (b'V', b'-', -0.073730),
    (b'V', b'.', -0.146484), (b'V', b':', -0.060059), (b'V', b';', -0.060059),
    (b'V', b'A', -0.102051), (b'V', b'a', -0.078613), (b'V', b'e', -0.064453),
    (b'V', b'i', -0.018066), (b'V', b'o', -0.064453), (b'V', b'r', -0.060059),
    (b'V', b'u', -0.064941), (b'V', b'y', -0.037109), (b'W', b',', -0.092285),
    (b'W', b'-', -0.068848), (b'W', b'.', -0.092285), (b'W', b':', -0.018066),
    (b'W', b';', -0.018066), (b'W', b'A', -0.087891), (b'W', b'a', -0.055664),
    (b'W', b'e', -0.045898), (b'W', b'i', -0.013672), (b'W', b'o', -0.045898),
    (b'W', b'r', -0.050293), (b'W', b'u', -0.041016), (b'W', b'y', -0.018066),
    (b'Y', b' ', -0.018066), (b'Y', b',', -0.161133), (b'Y', b'-', -0.122070),
    (b'Y', b'.', -0.161133), (b'Y', b':', -0.087402), (b'Y', b';', -0.087402),
    (b'Y', b'A', -0.106445), (b'Y', b'a', -0.092773), (b'Y', b'e', -0.104980),
    (b'Y', b'i', -0.055664), (b'Y', b'o', -0.114746), (b'Y', b'p', -0.092773),
    (b'Y', b'q', -0.119629), (b'Y', b'u', -0.073730), (b'Y', b'v', -0.059570),
    (b'r', b',', -0.142578), (b'r', b'.', -0.133301), (b'v', b',', -0.134277),
    (b'v', b'.', -0.134277), (b'w', b',', -0.105957), (b'w', b'.', -0.105957),
    (b'y', b',', -0.122070), (b'y', b'.', -0.122070),
];

/// Fallback advance width (in em) for characters outside the metrics table.
const TREBUCHET_FALLBACK_WIDTH: f64 = 0.6;

/// Line height ratio used by mermaid HTML labels (line-height: 1.5).
const TREBUCHET_LINE_HEIGHT: f64 = 1.5;

/// Mermaid `flowchart.wrappingWidth` default (config.schema.yaml): HTML
/// labels wider than this are re-rendered with `display: table`,
/// `white-space: break-spaces` and `width: 200px`, greedily word-wrapping
/// (rendering-util/createText.ts `addHtmlSpan`).
pub(crate) const MERMAID_WRAPPING_WIDTH: f64 = 200.0;

/// Advance widths (in em at 16px) for non-ASCII symbols that appear in
/// labels, derived from mermaid reference SVG label bboxes (browser
/// font-fallback rendering of the trebuchet ms stack).
#[rustfmt::skip]
const TREBUCHET_SYMBOL_WIDTHS: [(char, f64); 6] = [
    ('\u{2192}', 1.000486), // → (from 'SVG → PNG conversion' 164.734375)
    ('\u{2705}', 1.250488), // ✅ (from '✅ PASS' 56.796875)
    ('\u{2713}', 0.764648), // ✓ (bare label 12.234375)
    ('\u{2717}', 0.571289), // ✗ (bare label 9.140625)
    ('\u{25BC}', 0.897949), // ▼ (from 'World → Wor▼d' 119.4375)
    ('\u{274C}', 1.250488), // ❌ (from '❌ FLAKY FAIL' 102.328125)
];

/// Size estimator using pre-computed 'trebuchet ms' font metrics.
///
/// Mermaid.js measures flowchart labels in the browser with the default
/// `"trebuchet ms", verdana, arial` font stack at 16px and line-height 1.5.
/// This estimator reproduces those measurements from a per-character
/// advance-width table plus kerning pairs, giving label bboxes within
/// fractions of a pixel of mermaid's reference output.
///
/// Node sizing follows the mermaid shape formulas
/// (`rendering-util/rendering-elements/shapes/*.ts`) with the default
/// flowchart `node.padding` of 15 and no min/max clamps (mermaid has none).
#[derive(Debug, Clone, Default)]
pub struct TrebuchetSizeEstimator;

impl TrebuchetSizeEstimator {
    pub fn new() -> Self {
        Self
    }

    /// Measure the width of a single line of text at the given font size,
    /// applying advance widths and kerning like a browser would.
    fn measure_line(line: &str, font_size: f64) -> f64 {
        let mut width = 0.0;
        let mut prev: Option<char> = None;
        for c in line.chars() {
            let advance = match u32::from(c) {
                0x20..=0x7E => TREBUCHET_ADVANCE_WIDTHS[(u32::from(c) - 0x20) as usize],
                _ => TREBUCHET_SYMBOL_WIDTHS
                    .iter()
                    .find(|(sym, _)| *sym == c)
                    .map(|(_, w)| *w)
                    .unwrap_or(TREBUCHET_FALLBACK_WIDTH),
            };
            width += advance;
            if let (Some(p), true) = (prev, c.is_ascii()) {
                if p.is_ascii() {
                    let key = (p as u8, c as u8);
                    if let Ok(idx) =
                        TREBUCHET_KERN_PAIRS.binary_search_by(|(l, r, _)| (*l, *r).cmp(&key))
                    {
                        width += TREBUCHET_KERN_PAIRS[idx].2;
                    }
                }
            }
            prev = Some(c);
        }
        width * font_size
    }

    /// Measure a label the way mermaid's `addHtmlSpan` does
    /// (rendering-util/createText.ts):
    ///
    /// 1. Split on `<br/>` and collapse HTML whitespace runs per line.
    /// 2. Render nowrap with `max-width: wrapping_width`; if any line reaches
    ///    the limit, re-render with `display: table; width: wrapping_width;
    ///    white-space: break-spaces`, greedily word-wrapping every line.
    ///    A break keeps its space at the end of the broken line
    ///    (break-spaces preserves spaces), and words longer than the limit
    ///    stay unbroken, expanding the table beyond `wrapping_width`.
    ///
    /// Returns the final visual lines and the measured (width, height):
    /// width is the longest line, clamped up to `wrapping_width` when
    /// wrapping occurred (the forced table width); height is
    /// `lines * font_size * 1.5`.
    pub(crate) fn measure_label(
        text: &str,
        font_size: f64,
        wrapping_width: f64,
    ) -> (Vec<String>, f64, f64) {
        // Normalize <br> variants to newlines and decode HTML entities
        // before measuring, matching what the browser renders.
        let normalized = crate::render::text_utils::normalize_br_tags(text);
        let decoded = crate::render::text_utils::decode_html_entities(&normalized);

        // HTML collapses whitespace runs (and strips leading/trailing
        // whitespace around line breaks) in both nowrap and initial layout.
        let segments: Vec<String> = decoded
            .lines()
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect();

        let needs_wrap = segments
            .iter()
            .any(|seg| Self::measure_line(seg, font_size) >= wrapping_width);

        let lines: Vec<String> = if needs_wrap {
            segments
                .iter()
                .flat_map(|seg| Self::wrap_segment(seg, font_size, wrapping_width))
                .collect()
        } else {
            segments
        };

        let max_line_width = lines
            .iter()
            .map(|line| Self::measure_line(line, font_size))
            .fold(0.0_f64, f64::max);
        let width = if needs_wrap {
            // The re-rendered div gets width: wrapping_width, so the bbox is
            // at least that wide; unbreakable words can expand it further.
            max_line_width.max(wrapping_width)
        } else {
            max_line_width
        };
        let height = (lines.len().max(1) as f64) * font_size * TREBUCHET_LINE_HEIGHT;

        (lines, width, height)
    }

    /// Greedily wrap one whitespace-collapsed segment at `wrapping_width`,
    /// mirroring `white-space: break-spaces` line breaking: breaks happen
    /// after a space, the space stays on the broken line, and a word wider
    /// than the limit occupies its own line unbroken.
    fn wrap_segment(segment: &str, font_size: f64, wrapping_width: f64) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current = String::new();
        for word in segment.split(' ') {
            if current.is_empty() {
                current = word.to_string();
                continue;
            }
            let candidate = format!("{current} {word}");
            if Self::measure_line(&candidate, font_size) > wrapping_width {
                // break-spaces keeps the breaking space on the ended line
                current.push(' ');
                lines.push(current);
                current = word.to_string();
            } else {
                current = candidate;
            }
        }
        lines.push(current);
        lines
    }
}

impl SizeEstimator for TrebuchetSizeEstimator {
    fn estimate_text_size(&self, text: &str, font_size: f64) -> (f64, f64) {
        if text.is_empty() {
            return (0.0, 0.0);
        }

        let (_, width, height) = Self::measure_label(text, font_size, MERMAID_WRAPPING_WIDTH);
        (width, height)
    }

    /// Node sizing per mermaid.js shape handlers with `node.padding` (15).
    ///
    /// References (mermaid `rendering-util/rendering-elements/shapes/`):
    /// - squareRect.ts + drawRect.ts: w = bbox.w + 4p, h = bbox.h + 2p
    /// - circle.ts: diameter = bbox.w + p (radius = bbox.w/2 + p/2)
    /// - doubleCircle.ts: diameter = bbox.w + p + 2*gap (gap = 5)
    /// - question.ts: s = (bbox.w + p) + (bbox.h + p)
    /// - hexagon.ts: h = bbox.h + p; w = bbox.w + 2.5p; drawn width = 7w/6
    /// - stadium.ts: h = bbox.h + p; w = bbox.w + h/4 + p
    /// - leanRight.ts / leanLeft.ts / trapezoid.ts: w = bbox.w + p,
    ///   h = bbox.h + p; polygon spans (w + h) x h
    /// - invertedTrapezoid.ts: w = bbox.w + 2p, h = bbox.h + 2p; spans (w+h) x h
    /// - cylinder.ts: w = bbox.w + p; rx = w/2; ry = rx/(2.5 + w/50);
    ///   h = bbox.h + ry + p; drawn height = h + 2*ry
    /// - subroutine.ts: w = bbox.w + p, h = bbox.h + p; side bars add 8 each
    /// - rectLeftInvArrow.ts (odd): w = bbox.w + p, h = bbox.h + p;
    ///   notch extends w by h/4
    ///
    /// No min/max clamps are applied; mermaid has none.
    fn estimate_node_size(
        &self,
        label: Option<&str>,
        shape: NodeShape,
        config: &NodeSizeConfig,
    ) -> (f64, f64) {
        let (bw, bh) = label
            .filter(|l| !l.is_empty())
            .map(|l| {
                let (_, w, h) = Self::measure_label(l, config.font_size, config.wrapping_width);
                (w, h)
            })
            .unwrap_or((0.0, 0.0));
        let p = config.padding;

        match shape {
            // squareRect.ts: labelPaddingX = padding*2 -> bbox.w + 4*p
            NodeShape::Rectangle => (bw + 4.0 * p, bh + 2.0 * p),
            // roundedRect.ts: labelPaddingX = padding -> bbox.w + 2*p
            NodeShape::RoundedRect => (bw + 2.0 * p, bh + 2.0 * p),
            NodeShape::Circle => {
                let d = bw + p;
                (d, d)
            }
            NodeShape::DoubleCircle => {
                let gap = 5.0;
                let d = bw + p + 2.0 * gap;
                (d, d)
            }
            NodeShape::Diamond => {
                let s = (bw + p) + (bh + p);
                (s, s)
            }
            NodeShape::Hexagon => {
                let h = bh + p;
                let w = bw + 2.5 * p;
                // Polygon half-width is w/2 + w/12, so drawn width is 7w/6
                (w * 7.0 / 6.0, h)
            }
            NodeShape::Stadium => {
                let h = bh + p;
                let w = bw + h / 4.0 + p;
                (w, h)
            }
            NodeShape::LeanRight | NodeShape::LeanLeft | NodeShape::Trapezoid => {
                let w = bw + p;
                let h = bh + p;
                (w + h, h)
            }
            NodeShape::InvTrapezoid => {
                let w = bw + 2.0 * p;
                let h = bh + 2.0 * p;
                (w + h, h)
            }
            NodeShape::Cylinder => {
                let w = bw + p;
                let rx = w / 2.0;
                let ry = rx / (2.5 + w / 50.0);
                let h = bh + ry + p;
                (w, h + 2.0 * ry)
            }
            NodeShape::Subroutine => {
                let w = bw + p;
                let h = bh + p;
                (w + 16.0, h)
            }
            NodeShape::Odd => {
                let w = bw + p;
                let h = bh + p;
                (w + h / 4.0, h)
            }
            NodeShape::Ellipse => {
                // No dedicated flowchart ellipse in mermaid v11 shapes;
                // approximate with rect-like padding widened for the curve.
                (bw + 4.0 * p, bh + 2.0 * p)
            }
            NodeShape::HorizontalBar => (70.0, 10.0),
        }
    }
}

/// Create the best available size estimator
///
/// Tries to load a system font for accurate measurements.
/// Falls back to character-based estimation if no font is available.
pub fn create_size_estimator() -> Box<dyn SizeEstimator> {
    if let Some(font_estimator) = FontdueSizeEstimator::try_system_font() {
        Box::new(font_estimator)
    } else {
        Box::new(CharacterSizeEstimator::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_size_estimation() {
        let estimator = CharacterSizeEstimator::default();

        let (w, h) = estimator.estimate_text_size("Hello", 14.0);
        assert!(w > 0.0);
        assert!(h > 0.0);

        // Longer text should be wider
        let (w2, _) = estimator.estimate_text_size("Hello World", 14.0);
        assert!(w2 > w);

        // Multiline text should be taller
        let (_, h2) = estimator.estimate_text_size("Line1\nLine2", 14.0);
        assert!(h2 > h);
    }

    #[test]
    fn test_node_size_with_shapes() {
        let estimator = CharacterSizeEstimator::default();
        let config = NodeSizeConfig::default();

        let (rect_w, rect_h) =
            estimator.estimate_node_size(Some("Test"), NodeShape::Rectangle, &config);

        // Diamond should be larger than rectangle for same text
        let (diamond_w, diamond_h) =
            estimator.estimate_node_size(Some("Test"), NodeShape::Diamond, &config);
        assert!(diamond_w > rect_w);
        assert!(diamond_h > rect_h);

        // Circle should have equal width and height
        let (circle_w, circle_h) =
            estimator.estimate_node_size(Some("Test"), NodeShape::Circle, &config);
        assert!((circle_w - circle_h).abs() < 0.001);
    }

    #[test]
    fn test_min_size_constraints() {
        let estimator = CharacterSizeEstimator::default();
        let config = NodeSizeConfig {
            min_width: 100.0,
            min_height: 50.0,
            ..Default::default()
        };

        // Even with no label, should meet minimum size
        let (w, h) = estimator.estimate_node_size(None, NodeShape::Rectangle, &config);
        assert!(w >= 100.0);
        assert!(h >= 50.0);
    }

    // ── TrebuchetSizeEstimator ───────────────────────────────────────
    //
    // Reference bboxes extracted from mermaid.js SVG output
    // (eval-report flowchart example_flowchart_basic_reference.svg):
    //   'Square Rect' -> 85.1875 x 24
    //   'Circle'      -> 41.71875 x 24
    //   'Rhombus'     -> 64.0625 x 24
    //   'Link text'   -> 63.734375 x 24 (edge label)

    fn assert_close(actual: f64, expected: f64, tol: f64, what: &str) {
        assert!(
            (actual - expected).abs() <= tol,
            "{what}: expected {expected}, got {actual}"
        );
    }

    #[test]
    fn trebuchet_matches_reference_text_bboxes() {
        let estimator = TrebuchetSizeEstimator::new();

        for (text, expected_w) in [
            ("Square Rect", 85.1875),
            ("Circle", 41.71875),
            ("Rhombus", 64.0625),
            ("Link text", 63.734375),
        ] {
            let (w, h) = estimator.estimate_text_size(text, 16.0);
            assert_close(w, expected_w, 0.5, text);
            // Mermaid line height: fontSize * 1.5
            assert_close(h, 24.0, 0.001, text);
        }
    }

    #[test]
    fn trebuchet_handles_br_tags_and_entities() {
        let estimator = TrebuchetSizeEstimator::new();

        // <br> variants split lines; height = 2 lines * 24
        let (w1, _) = estimator.estimate_text_size("Circle", 16.0);
        let (w2, h2) = estimator.estimate_text_size("Circle<br/>Circle", 16.0);
        assert_close(w2, w1, 0.001, "widest line of multiline");
        assert_close(h2, 48.0, 0.001, "two-line height");

        // HTML entities are decoded before measuring
        let (w_entity, _) = estimator.estimate_text_size("&lt;x&gt;", 16.0);
        let (w_plain, _) = estimator.estimate_text_size("<x>", 16.0);
        assert_close(w_entity, w_plain, 0.001, "entity decoding");
    }

    #[test]
    fn trebuchet_non_ascii_uses_fallback_width() {
        let estimator = TrebuchetSizeEstimator::new();
        let (w, _) = estimator.estimate_text_size("日本", 16.0);
        // Fallback width: 0.6em per char
        assert_close(w, 2.0 * 16.0 * 0.6, 0.001, "non-ascii fallback");
    }

    #[test]
    fn trebuchet_rect_matches_mermaid_sizing() {
        // squareRect.ts + drawRect.ts: w = bbox.w + 4*padding, h = bbox.h + 2*padding
        // Reference: 'Square Rect' node A renders as 145.1875 x 54
        let estimator = TrebuchetSizeEstimator::new();
        let config = NodeSizeConfig::default();
        let (w, h) =
            estimator.estimate_node_size(Some("Square Rect"), NodeShape::Rectangle, &config);
        assert_close(w, 145.1875, 2.0, "rect width");
        assert_close(h, 54.0, 2.0, "rect height");
    }

    #[test]
    fn trebuchet_rounded_rect_uses_half_horizontal_padding() {
        // roundedRect.ts: labelPaddingX = node.padding (p), so w = bbox.w + 2*p
        // (squareRect.ts uses labelPaddingX = padding*2 -> bbox.w + 4*p).
        // Reference 'Round' bezier bbox width is 73.66 = 43.66 text + 2*15.
        let estimator = TrebuchetSizeEstimator::new();
        let config = NodeSizeConfig::default();
        let p = config.padding;
        let (text_w, _) = estimator.estimate_text_size("Round", 16.0);
        let (w, h) = estimator.estimate_node_size(Some("Round"), NodeShape::RoundedRect, &config);
        assert_close(w, text_w + 2.0 * p, 0.001, "rounded rect width = text + 2p");
        assert_close(h, 54.0, 2.0, "rounded rect height unchanged = bbox + 2p");
        // Must be strictly narrower than a plain rectangle of the same label.
        let (rect_w, _) =
            estimator.estimate_node_size(Some("Round"), NodeShape::Rectangle, &config);
        assert_close(
            rect_w - w,
            2.0 * p,
            0.001,
            "rounded is 2p narrower than rect",
        );
    }

    #[test]
    fn trebuchet_circle_matches_mermaid_sizing() {
        // circle.ts: radius = bbox.width / 2 + halfPadding
        // Reference: 'Circle' renders with r = 28.359375 (diameter 56.71875)
        let estimator = TrebuchetSizeEstimator::new();
        let config = NodeSizeConfig::default();
        let (w, h) = estimator.estimate_node_size(Some("Circle"), NodeShape::Circle, &config);
        assert_close(w, 56.71875, 2.0, "circle diameter");
        assert_close(h, 56.71875, 2.0, "circle diameter (height)");
    }

    #[test]
    fn trebuchet_diamond_matches_mermaid_sizing() {
        // question.ts: s = (bbox.w + padding) + (bbox.h + padding)
        // Reference: 'Rhombus' polygon spans 118.0625 x 118.0625
        let estimator = TrebuchetSizeEstimator::new();
        let config = NodeSizeConfig::default();
        let (w, h) = estimator.estimate_node_size(Some("Rhombus"), NodeShape::Diamond, &config);
        assert_close(w, 118.0625, 2.0, "diamond size");
        assert_close(h, 118.0625, 2.0, "diamond size (height)");
    }

    // ── wrappingWidth (mermaid flowchart.wrappingWidth = 200) ────────
    //
    // Mermaid's addHtmlSpan (rendering-util/createText.ts) renders HTML
    // labels with max-width: 200px; when the measured bbox hits that limit
    // it switches to display:table / white-space:break-spaces / width:200px
    // and re-measures, greedily word-wrapping the label. Reference bboxes
    // below are taken from mermaid reference SVGs in docs/images/reference.

    #[test]
    fn trebuchet_wraps_labels_at_wrapping_width() {
        // task_completion node J: label bbox is exactly 200 x 72 (3 lines);
        // "Dependent tasks stuck forever" wraps after "stuck".
        let estimator = TrebuchetSizeEstimator::new();
        let (w, h) = estimator.estimate_text_size(
            "Without ClearBlockedBy:<br/>Dependent tasks stuck forever",
            16.0,
        );
        assert_close(w, 200.0, 0.001, "wrapped label width clamps to 200");
        assert_close(h, 72.0, 0.001, "wrapped label height (3 lines)");
    }

    #[test]
    fn trebuchet_wrapped_width_expands_to_longest_unbreakable_word() {
        // message_indent ActionIndent: "(TIMESTAMP_GUTTER_WIDTH" cannot be
        // broken; with its trailing break space the reference bbox is
        // 211.59375 x 72 (min-content wider than the 200px table width).
        let estimator = TrebuchetSizeEstimator::new();
        let (w, h) = estimator.estimate_text_size(
            "indent_width = 7 + 2 = 9<br/>(TIMESTAMP_GUTTER_WIDTH + extra_indent)",
            16.0,
        );
        assert_close(w, 211.59375, 0.5, "unbreakable word width");
        assert_close(h, 72.0, 0.001, "3 visual lines");
    }

    #[test]
    fn trebuchet_collapses_html_whitespace_runs() {
        // message_indent Mermaid placeholder node: HTML collapses the run of
        // 9 spaces to one, so no line reaches 200px; reference bbox is
        // 197.21875 x 72 with no wrapping.
        let estimator = TrebuchetSizeEstimator::new();
        let (w, h) = estimator.estimate_text_size(
            "Mermaid placeholder:<br/>'         [1] Diagram: ...'<br/>(9 spaces via indent_width)",
            16.0,
        );
        assert_close(w, 197.21875, 0.5, "whitespace-collapsed width");
        assert_close(h, 72.0, 0.001, "3 explicit lines");
    }

    #[test]
    fn trebuchet_wrapped_node_matches_mermaid_sizing() {
        // task_completion node J renders as 260 x 102 (200 + 4*15, 72 + 2*15).
        let estimator = TrebuchetSizeEstimator::new();
        let config = NodeSizeConfig::default();
        let (w, h) = estimator.estimate_node_size(
            Some("Without ClearBlockedBy:<br/>Dependent tasks stuck forever"),
            NodeShape::Rectangle,
            &config,
        );
        assert_close(w, 260.0, 0.001, "wrapped rect width");
        assert_close(h, 102.0, 0.001, "wrapped rect height");
    }

    #[test]
    fn trebuchet_symbol_widths_match_reference() {
        // Symbol advance widths derived from mermaid reference SVG label
        // bboxes (modal_click_paths, test_parallel, refactoring samples).
        let estimator = TrebuchetSizeEstimator::new();
        for (text, expected_w) in [
            ("\u{2713}", 12.234375),                     // ✓
            ("\u{2717}", 9.140625),                      // ✗
            ("\u{2705} PASS", 56.796875),                // ✅ PASS
            ("\u{274C} FLAKY FAIL", 102.328125),         // ❌ FLAKY FAIL
            ("list() \u{2192} all statuses", 140.15625), // →
        ] {
            let (w, _) = estimator.estimate_text_size(text, 16.0);
            assert_close(w, expected_w, 0.5, text);
        }
    }

    #[test]
    fn trebuchet_no_min_max_clamps() {
        // Mermaid has no min/max node size clamps: a one-char label produces a
        // node sized purely from the formula.
        let estimator = TrebuchetSizeEstimator::new();
        let config = NodeSizeConfig::default();
        let (w, h) = estimator.estimate_node_size(Some("i"), NodeShape::Rectangle, &config);
        let (bw, bh) = estimator.estimate_text_size("i", 16.0);
        assert_close(w, bw + 60.0, 0.001, "no min width clamp");
        assert_close(h, bh + 30.0, 0.001, "no min height clamp");
    }
}
