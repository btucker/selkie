//! Common text utilities shared across diagram renderers.
//!
//! Consolidates duplicated string operations: BR tag normalization,
//! proportional text width estimation, and word-wrap by pixel width.

/// Normalize HTML `<br>` tag variants to newline characters.
///
/// Handles `<br>`, `<br/>`, and `<br />` forms.
pub(crate) fn normalize_br_tags(text: &str) -> String {
    text.replace("<br />", "\n")
        .replace("<br/>", "\n")
        .replace("<br>", "\n")
}

/// Normalize Mermaid label markup into the visible text Selkie should emit.
///
/// Mermaid's HTML-label path treats `<br/>` as a line break, lets simple inline
/// formatting tags contribute only their inner text, and decodes common HTML
/// entities such as `&lt;` before the SVG/XML layer escapes the final text.
pub(crate) fn normalize_mermaid_label_markup(text: &str) -> String {
    let with_breaks = normalize_br_tags(text);
    let without_formatting = strip_inline_formatting_tags(&with_breaks);
    let decoded = decode_html_entities(&without_formatting);
    decode_mermaid_escapes(&decoded)
}

fn strip_inline_formatting_tags(text: &str) -> String {
    const TAGS: &[&str] = &[
        "<b>",
        "</b>",
        "<strong>",
        "</strong>",
        "<em>",
        "</em>",
        "<i>",
        "</i>",
    ];

    let mut result = text.to_string();
    for tag in TAGS {
        result = result.replace(tag, "");
    }
    result
}

/// Decode common HTML entities to their literal characters.
///
/// Handles the entities mermaid labels commonly contain (`&lt;`, `&gt;`,
/// `&quot;`, `&#39;`, `&apos;`, `&amp;`). `&amp;` is decoded last so that
/// doubly escaped sequences (e.g. `&amp;lt;`) are not over-decoded.
pub(crate) fn decode_html_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn decode_mermaid_escapes(text: &str) -> String {
    text.replace("\\\\", "\\")
}

/// A styled run of text produced by parsing a markdown label.
///
/// Mirrors the word/segment records mermaid's `markdownToLines`
/// (`rendering-util/createText.ts`) emits, reduced to the inline styles
/// Selkie renders: bold (`**x**` / `__x__`) and italic (`*x*` / `_x_`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MarkdownRun {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
}

/// Parse a minimal markdown subset into styled runs.
///
/// Transliterates mermaid's markdown handling for the inline emphasis it
/// renders on flowchart labels: `**x**`/`__x__` become bold runs and
/// `*x*`/`_x_` become italic runs. Everything else is emitted verbatim as
/// unstyled runs. Unbalanced markers are treated as literal text (never
/// panics). Bold is matched before italic so `**` is not mis-read as two
/// italic delimiters.
pub(crate) fn parse_markdown_runs(text: &str) -> Vec<MarkdownRun> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut runs: Vec<MarkdownRun> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;

    fn flush(buf: &mut String, runs: &mut Vec<MarkdownRun>) {
        if !buf.is_empty() {
            runs.push(MarkdownRun {
                text: std::mem::take(buf),
                bold: false,
                italic: false,
            });
        }
    }

    while i < n {
        let c = chars[i];

        // Bold: paired `**` or `__` delimiters with non-empty inner text.
        if (c == '*' || c == '_') && i + 1 < n && chars[i + 1] == c {
            if let Some(close) = find_markdown_close(&chars, i + 2, c, true) {
                flush(&mut buf, &mut runs);
                runs.push(MarkdownRun {
                    text: chars[i + 2..close].iter().collect(),
                    bold: true,
                    italic: false,
                });
                i = close + 2;
                continue;
            }
        }

        // Italic: paired single `*` or `_` delimiter with non-empty inner text.
        if c == '*' || c == '_' {
            if let Some(close) = find_markdown_close(&chars, i + 1, c, false) {
                flush(&mut buf, &mut runs);
                runs.push(MarkdownRun {
                    text: chars[i + 1..close].iter().collect(),
                    bold: false,
                    italic: true,
                });
                i = close + 1;
                continue;
            }
        }

        buf.push(c);
        i += 1;
    }

    flush(&mut buf, &mut runs);
    runs
}

/// Find the index of the closing delimiter for an emphasis span, or `None`
/// when the span is unbalanced or empty. `double` requires two consecutive
/// `delim` characters (bold); otherwise a single `delim` closes (italic).
fn find_markdown_close(chars: &[char], from: usize, delim: char, double: bool) -> Option<usize> {
    let n = chars.len();
    let mut j = from;
    while j < n {
        if chars[j] == delim {
            if double {
                if j + 1 < n && chars[j + 1] == delim {
                    return if j > from { Some(j) } else { None };
                }
                // A lone delimiter inside a bold span is literal content.
                j += 1;
                continue;
            }
            return if j > from { Some(j) } else { None };
        }
        j += 1;
    }
    None
}

/// The visible text of a markdown label with all emphasis markers removed.
///
/// Mermaid measures the *rendered* label (bold/italic glyphs, no `*`/`_`
/// source markers), so layout must size markdown nodes from this stripped
/// text rather than the raw source. `<br/>` tags and HTML entities are left
/// intact for the downstream measurement/normalization step to handle.
pub(crate) fn strip_markdown_markers(text: &str) -> String {
    parse_markdown_runs(text)
        .iter()
        .map(|run| run.text.as_str())
        .collect()
}

/// Wrap flowchart label text exactly like the layout measurement does
/// (mermaid `flowchart.wrappingWidth` = 200px greedy word-wrap), so the
/// drawn tspans match the node/edge boxes sized by the layout.
///
/// Lines are joined with `\n`; trailing break spaces are trimmed for
/// display (they only affect measurement).
pub(crate) fn wrap_label_text_mermaid(text: &str, font_size: f64) -> String {
    let (lines, _, _) = crate::layout::TrebuchetSizeEstimator::measure_label(
        text,
        font_size,
        crate::layout::size::MERMAID_WRAPPING_WIDTH,
    );
    lines
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Estimate text width in pixels using per-character weight classes.
///
/// Approximates browser rendering of proportional fonts (e.g. Trebuchet MS)
/// by bucketing characters into narrow, regular, semi-wide, and wide classes.
pub(crate) fn estimate_text_width(text: &str, font_size: f64) -> f64 {
    let mut total_width = 0.0;

    for c in text.chars() {
        let char_width = match c {
            // Narrow characters
            'i' | 'l' | 'I' | '!' | '|' | '\'' | '.' | ',' | ':' | ';' | 'j' | 'f' | 't' | 'r' => {
                font_size * 0.35
            }
            // Wide characters
            'M' | 'W' | 'm' | 'w' | '@' => font_size * 0.9,
            // Semi-wide uppercase
            'N' | 'O' | 'Q' | 'G' | 'D' | 'H' | 'U' | 'A' | 'V' | 'X' | 'Y' | 'Z' | 'K' | 'R'
            | 'B' | 'P' => font_size * 0.65,
            // Space
            ' ' => font_size * 0.35,
            // Regular lowercase
            'a'..='z' => font_size * 0.5,
            // Regular uppercase (fallback for any not matched above)
            'A'..='Z' => font_size * 0.6,
            // Numbers
            '0'..='9' => font_size * 0.55,
            // Default
            _ => font_size * 0.5,
        };
        total_width += char_width;
    }

    total_width
}

/// Wrap text into lines that fit within `max_width` pixels.
///
/// Uses [`estimate_text_width`] to measure each candidate line.
/// Words are never broken — a single word wider than `max_width` gets its own line.
pub(crate) fn wrap_text_by_width(text: &str, max_width: f64, font_size: f64) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in words {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else {
            let potential = format!("{} {}", current_line, word);
            if estimate_text_width(&potential, font_size) <= max_width {
                current_line = potential;
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── normalize_br_tags ────────────────────────────────────────────

    #[test]
    fn normalize_br_tags_handles_all_variants() {
        assert_eq!(normalize_br_tags("a<br>b"), "a\nb");
        assert_eq!(normalize_br_tags("a<br/>b"), "a\nb");
        assert_eq!(normalize_br_tags("a<br />b"), "a\nb");
    }

    #[test]
    fn normalize_br_tags_handles_mixed_variants() {
        assert_eq!(
            normalize_br_tags("line1<br>line2<br/>line3<br />line4"),
            "line1\nline2\nline3\nline4"
        );
    }

    #[test]
    fn normalize_br_tags_preserves_plain_text() {
        assert_eq!(normalize_br_tags("no breaks here"), "no breaks here");
    }

    #[test]
    fn normalize_br_tags_empty_string() {
        assert_eq!(normalize_br_tags(""), "");
    }

    #[test]
    fn normalize_mermaid_label_markup_decodes_entities_after_removing_tags() {
        assert_eq!(
            normalize_mermaid_label_markup("Vec&lt;Effect&gt;"),
            "Vec<Effect>"
        );
        assert_eq!(normalize_mermaid_label_markup("Some<b>2</b>"), "Some2");
    }

    #[test]
    fn normalize_mermaid_label_markup_converts_br_before_removing_tags() {
        assert_eq!(
            normalize_mermaid_label_markup("Line 1<br/>Line <b>2</b>"),
            "Line 1\nLine 2"
        );
    }

    // ── markdown runs ────────────────────────────────────────────────

    #[test]
    fn parse_markdown_runs_splits_bold_and_italic() {
        let runs = parse_markdown_runs("**bold** and _em_");
        assert_eq!(
            runs,
            vec![
                MarkdownRun {
                    text: "bold".to_string(),
                    bold: true,
                    italic: false
                },
                MarkdownRun {
                    text: " and ".to_string(),
                    bold: false,
                    italic: false
                },
                MarkdownRun {
                    text: "em".to_string(),
                    bold: false,
                    italic: true
                },
            ]
        );
    }

    #[test]
    fn parse_markdown_runs_handles_underscore_bold_and_asterisk_italic() {
        assert_eq!(
            parse_markdown_runs("__b__"),
            vec![MarkdownRun {
                text: "b".to_string(),
                bold: true,
                italic: false
            }]
        );
        assert_eq!(
            parse_markdown_runs("*i*"),
            vec![MarkdownRun {
                text: "i".to_string(),
                bold: false,
                italic: true
            }]
        );
    }

    #[test]
    fn parse_markdown_runs_treats_unbalanced_markers_as_literal() {
        assert_eq!(
            parse_markdown_runs("2 * 3 = 6"),
            vec![MarkdownRun {
                text: "2 * 3 = 6".to_string(),
                bold: false,
                italic: false
            }]
        );
    }

    #[test]
    fn strip_markdown_markers_removes_all_emphasis() {
        assert_eq!(strip_markdown_markers("**bold** and _em_"), "bold and em");
        assert_eq!(strip_markdown_markers("plain"), "plain");
    }

    #[test]
    fn normalize_mermaid_label_markup_decodes_literal_backslash_escape() {
        assert_eq!(
            normalize_mermaid_label_markup(r"join with \\n"),
            r"join with \n"
        );
    }

    // ── estimate_text_width ──────────────────────────────────────────

    #[test]
    fn estimate_width_empty_string() {
        assert_eq!(estimate_text_width("", 16.0), 0.0);
    }

    #[test]
    fn estimate_width_narrow_chars_smaller_than_wide() {
        let narrow = estimate_text_width("iii", 16.0);
        let wide = estimate_text_width("MMM", 16.0);
        assert!(narrow < wide, "narrow={narrow} should be < wide={wide}");
    }

    #[test]
    fn estimate_width_scales_with_font_size() {
        let small = estimate_text_width("hello", 10.0);
        let large = estimate_text_width("hello", 20.0);
        assert!(
            (large - small * 2.0).abs() < 0.001,
            "doubling font_size should double width"
        );
    }

    #[test]
    fn estimate_width_space_counted() {
        let no_space = estimate_text_width("ab", 16.0);
        let with_space = estimate_text_width("a b", 16.0);
        assert!(with_space > no_space);
    }

    // ── wrap_text_by_width ───────────────────────────────────────────

    #[test]
    fn wrap_empty_text() {
        assert_eq!(wrap_text_by_width("", 100.0, 16.0), vec![""]);
    }

    #[test]
    fn wrap_single_word_fits() {
        assert_eq!(wrap_text_by_width("hello", 200.0, 16.0), vec!["hello"]);
    }

    #[test]
    fn wrap_forces_long_word_onto_own_line() {
        // A very narrow max_width but single word — should still appear
        let result = wrap_text_by_width("supercalifragilistic", 1.0, 16.0);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "supercalifragilistic");
    }

    #[test]
    fn wrap_splits_when_exceeding_width() {
        // Use a width that fits ~5 lowercase chars at font_size=16
        // 5 chars * 16 * 0.5 = 40
        let result = wrap_text_by_width("aaa bbb ccc", 45.0, 16.0);
        assert!(result.len() >= 2, "should wrap: {:?}", result);
    }

    #[test]
    fn wrap_preserves_word_order() {
        let result = wrap_text_by_width("one two three four", 200.0, 16.0);
        let joined = result.join(" ");
        assert_eq!(joined, "one two three four");
    }
}
