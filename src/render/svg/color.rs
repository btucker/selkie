//! Color manipulation utilities for theme derivation
//!
//! This module provides color manipulation functions similar to the khroma
//! library used by mermaid.js for computing derived theme colors.

/// A color with RGB and optional alpha components
#[derive(Debug, Clone, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f64, // Alpha 0.0 - 1.0
}

impl Color {
    /// Create a new color from RGB values
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create a new color from RGBA values
    pub fn rgba(r: u8, g: u8, b: u8, a: f64) -> Self {
        Self {
            r,
            g,
            b,
            a: a.clamp(0.0, 1.0),
        }
    }

    /// Parse a color from a hex string (supports #RGB, #RRGGBB, #RRGGBBAA)
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');

        match hex.len() {
            3 => {
                // #RGB -> #RRGGBB
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::rgba(r, g, b, a as f64 / 255.0))
            }
            _ => None,
        }
    }

    /// Parse a color from various formats (hex, rgb(), rgba(), named colors)
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();

        // Hex format
        if s.starts_with('#') {
            return Self::from_hex(s);
        }

        // rgba() format
        if s.starts_with("rgba(") && s.ends_with(')') {
            let inner = &s[5..s.len() - 1];
            let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
            if parts.len() == 4 {
                let r = parts[0].parse().ok()?;
                let g = parts[1].parse().ok()?;
                let b = parts[2].parse().ok()?;
                let a = parts[3].parse().ok()?;
                return Some(Self::rgba(r, g, b, a));
            }
        }

        // rgb() format
        if s.starts_with("rgb(") && s.ends_with(')') {
            let inner = &s[4..s.len() - 1];
            let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
            if parts.len() == 3 {
                let r = parts[0].parse().ok()?;
                let g = parts[1].parse().ok()?;
                let b = parts[2].parse().ok()?;
                return Some(Self::rgb(r, g, b));
            }
        }

        // Named colors (basic set)
        match s.to_lowercase().as_str() {
            "white" => Some(Self::rgb(255, 255, 255)),
            "black" => Some(Self::rgb(0, 0, 0)),
            "red" => Some(Self::rgb(255, 0, 0)),
            "green" => Some(Self::rgb(0, 128, 0)),
            "blue" => Some(Self::rgb(0, 0, 255)),
            "yellow" => Some(Self::rgb(255, 255, 0)),
            "grey" | "gray" => Some(Self::rgb(128, 128, 128)),
            "lightgrey" | "lightgray" => Some(Self::rgb(211, 211, 211)),
            "darkgrey" | "darkgray" => Some(Self::rgb(169, 169, 169)),
            "navy" => Some(Self::rgb(0, 0, 128)),
            "purple" => Some(Self::rgb(128, 0, 128)),
            "orange" => Some(Self::rgb(255, 165, 0)),
            "pink" => Some(Self::rgb(255, 192, 203)),
            "cyan" => Some(Self::rgb(0, 255, 255)),
            "magenta" => Some(Self::rgb(255, 0, 255)),
            "transparent" => Some(Self::rgba(0, 0, 0, 0.0)),
            _ => None,
        }
    }

    /// Convert to hex string (#RRGGBB or #RRGGBBAA if has alpha)
    pub fn to_hex(&self) -> String {
        if (self.a - 1.0).abs() < 0.001 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            let a = (self.a * 255.0).round() as u8;
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, a)
        }
    }

    /// Convert to rgba() string
    pub fn to_rgba_string(&self) -> String {
        format!("rgba({}, {}, {}, {})", self.r, self.g, self.b, self.a)
    }

    /// Convert to hsl() string (matching mermaid.js output format)
    /// Uses high precision to match mermaid.js computed values
    pub fn to_hsl_string(&self) -> String {
        let (h, s, l) = self.to_hsl();
        // Mermaid.js outputs full precision HSL values
        // Format: hsl(h, s%, l%) with high precision decimals
        format!(
            "hsl({}, {}%, {}%)",
            h.round() as i32,
            format_hsl_pct(s),
            format_hsl_pct(l)
        )
    }

    /// Convert RGB to HSL (returns h: 0-360, s: 0-100, l: 0-100)
    pub fn to_hsl(&self) -> (f64, f64, f64) {
        let r = self.r as f64 / 255.0;
        let g = self.g as f64 / 255.0;
        let b = self.b as f64 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        // Lightness
        let l = (max + min) / 2.0;

        if delta < 0.00001 {
            // Achromatic (gray)
            return (0.0, 0.0, l * 100.0);
        }

        // Saturation
        let s = if l < 0.5 {
            delta / (max + min)
        } else {
            delta / (2.0 - max - min)
        };

        // Hue
        let h = if (max - r).abs() < 0.00001 {
            let mut h = (g - b) / delta;
            if g < b {
                h += 6.0;
            }
            h
        } else if (max - g).abs() < 0.00001 {
            (b - r) / delta + 2.0
        } else {
            (r - g) / delta + 4.0
        };

        (h * 60.0, s * 100.0, l * 100.0)
    }

    /// Create a color from HSL values (h: 0-360, s: 0-100, l: 0-100)
    pub fn from_hsl(h: f64, s: f64, l: f64) -> Self {
        let h = ((h % 360.0) + 360.0) % 360.0; // Normalize hue to 0-360
        let s = (s / 100.0).clamp(0.0, 1.0);
        let l = (l / 100.0).clamp(0.0, 1.0);

        if s < 0.00001 {
            // Achromatic
            let v = (l * 255.0).round() as u8;
            return Self::rgb(v, v, v);
        }

        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;

        let hue_to_rgb = |p: f64, q: f64, mut t: f64| -> f64 {
            if t < 0.0 {
                t += 1.0;
            }
            if t > 1.0 {
                t -= 1.0;
            }
            if t < 1.0 / 6.0 {
                return p + (q - p) * 6.0 * t;
            }
            if t < 0.5 {
                return q;
            }
            if t < 2.0 / 3.0 {
                return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
            }
            p
        };

        let h_norm = h / 360.0;
        let r = (hue_to_rgb(p, q, h_norm + 1.0 / 3.0) * 255.0).round() as u8;
        let g = (hue_to_rgb(p, q, h_norm) * 255.0).round() as u8;
        let b = (hue_to_rgb(p, q, h_norm - 1.0 / 3.0) * 255.0).round() as u8;

        Self::rgb(r, g, b)
    }

    /// Calculate relative luminance (0.0 - 1.0)
    /// Uses the formula from WCAG 2.0
    pub fn luminance(&self) -> f64 {
        let to_linear = |c: u8| {
            let c = c as f64 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };

        let r = to_linear(self.r);
        let g = to_linear(self.g);
        let b = to_linear(self.b);

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Check if the color is dark (luminance < 0.5)
    pub fn is_dark(&self) -> bool {
        self.luminance() < 0.5
    }
}

/// Format a percentage value for HSL output, matching mermaid.js precision
/// Uses full precision for non-integer values, integer for whole numbers
fn format_hsl_pct(value: f64) -> String {
    // Check if value is close to a whole number
    let rounded = value.round();
    if (value - rounded).abs() < 0.0001 {
        format!("{}", rounded as i32)
    } else {
        // Use full precision like mermaid.js does
        format!("{}", value)
    }
}

/// Adjust a color's HSL values
/// Amounts are in degrees (h) or percentage points (s, l)
pub fn adjust(color: &Color, h: f64, s: f64, l: f64) -> Color {
    let (ch, cs, cl) = color.to_hsl();
    let mut result = Color::from_hsl(ch + h, cs + s, cl + l);
    result.a = color.a;
    result
}

/// Adjust a color's HSL values and return the result as an HSL string directly.
/// This avoids the precision loss from converting HSL → RGB → HSL.
/// Output format matches mermaid.js: hsl(h, s%, l%) with specific precision.
pub fn adjust_to_hsl_string(color: &Color, h: f64, s: f64, l: f64) -> String {
    let (ch, cs, cl) = color.to_hsl();
    let new_h = ((ch + h) % 360.0 + 360.0) % 360.0; // Normalize hue
    let new_s = (cs + s).clamp(0.0, 100.0);
    let new_l = (cl + l).clamp(0.0, 100.0);

    // Format matching mermaid.js precision (10-digit precision for percentages)
    format!(
        "hsl({}, {}%, {}%)",
        new_h.round() as i32,
        format_mermaid_pct(new_s),
        format_mermaid_pct(new_l)
    )
}

/// Format percentage value matching mermaid.js output precision
fn format_mermaid_pct(value: f64) -> String {
    // Mermaid.js uses 10 significant digits after decimal
    // Check if value is close to a whole number
    let rounded = value.round();
    if (value - rounded).abs() < 0.0001 {
        format!("{}", rounded as i32)
    } else {
        // Match mermaid's precision: up to 10 decimal places
        format!("{:.10}", value)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Darken a color by reducing lightness
/// Amount is in percentage points (0-100)
pub fn darken(color: &Color, amount: f64) -> Color {
    adjust(color, 0.0, 0.0, -amount)
}

/// Lighten a color by increasing lightness
/// Amount is in percentage points (0-100)
pub fn lighten(color: &Color, amount: f64) -> Color {
    adjust(color, 0.0, 0.0, amount)
}

/// Invert a color (RGB inversion)
pub fn invert(color: &Color) -> Color {
    Color::rgba(255 - color.r, 255 - color.g, 255 - color.b, color.a)
}

/// Add or modify the alpha channel
pub fn with_alpha(color: &Color, alpha: f64) -> Color {
    Color::rgba(color.r, color.g, color.b, alpha)
}

/// Create a border color from a fill color
/// In dark mode: lighter and less saturated
/// In light mode: darker and less saturated
pub fn mk_border(color: &Color, dark_mode: bool) -> Color {
    if dark_mode {
        adjust(color, 0.0, -40.0, 10.0)
    } else {
        adjust(color, 0.0, -40.0, -10.0)
    }
}

/// Get a contrasting text color (black or white)
pub fn contrasting_text(background: &Color) -> Color {
    if background.is_dark() {
        Color::rgb(255, 255, 255)
    } else {
        Color::rgb(0, 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex() {
        let c = Color::from_hex("#ff0000").unwrap();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);

        let c = Color::from_hex("#ECECFF").unwrap();
        assert_eq!(c.r, 236);
        assert_eq!(c.g, 236);
        assert_eq!(c.b, 255);
    }

    #[test]
    fn test_parse_short_hex() {
        let c = Color::from_hex("#f00").unwrap();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    }

    #[test]
    fn test_to_hex() {
        let c = Color::rgb(255, 128, 64);
        assert_eq!(c.to_hex(), "#ff8040");
    }

    #[test]
    fn test_hsl_roundtrip() {
        let original = Color::rgb(100, 150, 200);
        let (h, s, l) = original.to_hsl();
        let roundtrip = Color::from_hsl(h, s, l);

        // Allow small rounding errors
        assert!((original.r as i16 - roundtrip.r as i16).abs() <= 1);
        assert!((original.g as i16 - roundtrip.g as i16).abs() <= 1);
        assert!((original.b as i16 - roundtrip.b as i16).abs() <= 1);
    }

    #[test]
    fn test_is_dark() {
        assert!(Color::rgb(0, 0, 0).is_dark());
        assert!(Color::rgb(50, 50, 50).is_dark());
        assert!(!Color::rgb(255, 255, 255).is_dark());
        assert!(!Color::rgb(200, 200, 200).is_dark());
    }

    #[test]
    fn test_darken() {
        let c = Color::from_hex("#8a90dd").unwrap();
        let darker = darken(&c, 10.0);
        let (_, _, l_orig) = c.to_hsl();
        let (_, _, l_dark) = darker.to_hsl();
        assert!(l_dark < l_orig);
    }

    #[test]
    fn test_lighten() {
        let c = Color::from_hex("#8a90dd").unwrap();
        let lighter = lighten(&c, 10.0);
        let (_, _, l_orig) = c.to_hsl();
        let (_, _, l_light) = lighter.to_hsl();
        assert!(l_light > l_orig);
    }

    #[test]
    fn test_invert() {
        let white = Color::rgb(255, 255, 255);
        let black = invert(&white);
        assert_eq!(black.r, 0);
        assert_eq!(black.g, 0);
        assert_eq!(black.b, 0);

        let c = Color::rgb(100, 150, 200);
        let inv = invert(&c);
        assert_eq!(inv.r, 155);
        assert_eq!(inv.g, 105);
        assert_eq!(inv.b, 55);
    }

    #[test]
    fn test_mk_border_light_mode() {
        let c = Color::from_hex("#ECECFF").unwrap();
        let border = mk_border(&c, false);
        let (_, s_border, l_border) = border.to_hsl();
        let (_, s_orig, l_orig) = c.to_hsl();

        // Should be less saturated and darker
        assert!(s_border < s_orig || s_orig < 1.0); // Already low saturation
        assert!(l_border < l_orig);
    }

    #[test]
    fn test_adjust_hue() {
        let red = Color::rgb(255, 0, 0);
        let (h, _, _) = red.to_hsl();
        assert!((h - 0.0).abs() < 1.0); // Red is at hue 0

        // Shift hue by 120 degrees (should give greenish)
        let shifted = adjust(&red, 120.0, 0.0, 0.0);
        let (h2, _, _) = shifted.to_hsl();
        assert!((h2 - 120.0).abs() < 1.0);
    }

    #[test]
    fn test_parse_rgba() {
        let c = Color::parse("rgba(255, 128, 64, 0.5)").unwrap();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 64);
        assert!((c.a - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_parse_named() {
        assert_eq!(Color::parse("white").unwrap(), Color::rgb(255, 255, 255));
        assert_eq!(Color::parse("black").unwrap(), Color::rgb(0, 0, 0));
        assert_eq!(Color::parse("red").unwrap(), Color::rgb(255, 0, 0));
    }

    #[test]
    fn test_contrasting_text() {
        let dark_bg = Color::rgb(50, 50, 50);
        let text = contrasting_text(&dark_bg);
        assert_eq!(text, Color::rgb(255, 255, 255));

        let light_bg = Color::rgb(200, 200, 200);
        let text = contrasting_text(&light_bg);
        assert_eq!(text, Color::rgb(0, 0, 0));
    }
}
