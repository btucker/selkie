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
