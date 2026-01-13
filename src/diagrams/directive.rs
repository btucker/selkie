//! Directive parsing for mermaid diagram configuration
//!
//! Parses `%%{init: ...}%%` directives that configure theme and other options.
//!
//! ## Supported syntax
//!
//! ```text
//! %%{init: {'theme': 'forest'}}%%
//! %%{init: {"theme": "dark", "themeVariables": {"primaryColor": "#ff0000"}}}%%
//! ```
//!
//! The directive must appear at the start of the diagram text.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Regex to match `%%{...}%%` directive blocks
/// Matches: %%{directive_name: json_content}%%
///
/// Uses `(?s)` for single-line mode (dot matches newline) and non-greedy `.*?`
/// to capture content up to the first `}%%`.
///
/// Groups:
/// - 1: Directive name (e.g., "init")
/// - 2: JSON content (everything between the name and closing }%%)
static DIRECTIVE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // (?s) enables dot-matches-newline mode
    // .*? is non-greedy so it stops at the first }%%
    Regex::new(r"(?s)%%\{\s*(\w+)\s*:\s*(.*?)\s*\}%%").unwrap()
});

/// Configuration extracted from diagram directives
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramConfig {
    /// Theme name (e.g., "default", "dark", "forest")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,

    /// Theme variable overrides
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub theme_variables: HashMap<String, String>,

    /// Custom CSS to inject after theme CSS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_css: Option<String>,

    /// Diagram-specific configuration (opaque JSON)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl DiagramConfig {
    /// Merge another config into this one (other takes precedence)
    pub fn merge(&mut self, other: &DiagramConfig) {
        if other.theme.is_some() {
            self.theme = other.theme.clone();
        }
        for (k, v) in &other.theme_variables {
            self.theme_variables.insert(k.clone(), v.clone());
        }
        if other.theme_css.is_some() {
            self.theme_css = other.theme_css.clone();
        }
        for (k, v) in &other.extra {
            self.extra.insert(k.clone(), v.clone());
        }
    }
}

/// A single parsed directive
#[derive(Debug, Clone)]
pub struct Directive {
    /// Directive type (e.g., "init", "wrap")
    pub directive_type: String,
    /// Parsed arguments (may be empty)
    pub args: Option<serde_json::Value>,
}

/// Detect and parse all directives from diagram text
///
/// Returns a vector of all directives found in the text.
pub fn detect_directives(text: &str) -> Vec<Directive> {
    let mut directives = Vec::new();

    for cap in DIRECTIVE_REGEX.captures_iter(text) {
        let directive_type = cap
            .get(1)
            .map(|m| m.as_str().to_lowercase())
            .unwrap_or_default();
        let json_content = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("");

        // Try to parse as JSON (handle both single and double quotes)
        let normalized_json = normalize_json(json_content);
        let args = serde_json::from_str(&normalized_json).ok();

        directives.push(Directive {
            directive_type,
            args,
        });
    }

    directives
}

/// Detect init directives and extract configuration
///
/// Looks for `%%{init: ...}%%` directives and parses the theme configuration.
/// Multiple init directives are merged (later ones override earlier ones).
pub fn detect_init(text: &str) -> Option<DiagramConfig> {
    let directives = detect_directives(text);

    let init_directives: Vec<_> = directives
        .into_iter()
        .filter(|d| d.directive_type == "init" || d.directive_type == "initialize")
        .collect();

    if init_directives.is_empty() {
        return None;
    }

    let mut config = DiagramConfig::default();

    for directive in init_directives {
        if let Some(args) = directive.args {
            if let Ok(parsed) = parse_init_args(&args) {
                let sanitized = sanitize_config(parsed);
                config.merge(&sanitized);
            }
        }
    }

    Some(config)
}

/// Parse init directive arguments into DiagramConfig
fn parse_init_args(args: &serde_json::Value) -> Result<DiagramConfig, serde_json::Error> {
    // Always use manual extraction because serde's #[serde(flatten)] on `extra`
    // consumes keys like "themeCSS" before rename_all can match them to theme_css.
    let mut config = DiagramConfig::default();

    if let Some(obj) = args.as_object() {
        // Extract theme
        if let Some(theme) = obj.get("theme").and_then(|v| v.as_str()) {
            config.theme = Some(theme.to_string());
        }

        // Extract themeVariables
        if let Some(vars) = obj.get("themeVariables").and_then(|v| v.as_object()) {
            for (key, value) in vars {
                if let Some(s) = value.as_str() {
                    config.theme_variables.insert(key.clone(), s.to_string());
                }
            }
        }

        // Extract themeCSS
        if let Some(css) = obj.get("themeCSS").and_then(|v| v.as_str()) {
            config.theme_css = Some(css.to_string());
        }

        // Collect other keys into extra
        for (key, value) in obj {
            if key != "theme" && key != "themeVariables" && key != "themeCSS" {
                config.extra.insert(key.clone(), value.clone());
            }
        }
    }

    Ok(config)
}

/// Sanitize configuration to prevent XSS and injection attacks
///
/// Follows mermaid.js security patterns:
/// - Removes keys starting with "__" (prototype pollution)
/// - Validates theme variable values (only safe characters)
/// - Removes HTML tags and data URLs
fn sanitize_config(mut config: DiagramConfig) -> DiagramConfig {
    // Sanitize theme name
    if let Some(ref theme) = config.theme {
        if !is_safe_theme_name(theme) {
            config.theme = None;
        }
    }

    // Sanitize theme variables
    config
        .theme_variables
        .retain(|key, value| is_safe_key(key) && is_safe_css_value(value));

    // Sanitize extra config
    config.extra = config
        .extra
        .into_iter()
        .filter(|(key, _)| is_safe_key(key))
        .map(|(key, value)| (key, sanitize_value(value)))
        .collect();

    config
}

/// Check if a key is safe (no prototype pollution)
fn is_safe_key(key: &str) -> bool {
    !key.starts_with("__") && !key.contains("proto") && !key.contains("constr")
}

/// Check if a theme name is safe
fn is_safe_theme_name(theme: &str) -> bool {
    // Only allow alphanumeric and hyphens
    theme
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Check if a CSS value is safe for theme variables
/// Matches mermaid.js pattern: /^[\d "#%(),.;A-Za-z]+$/
fn is_safe_css_value(value: &str) -> bool {
    value.chars().all(|c| {
        c.is_alphanumeric()
            || c == ' '
            || c == '"'
            || c == '#'
            || c == '%'
            || c == '('
            || c == ')'
            || c == ','
            || c == '.'
            || c == ';'
            || c == '-'
            || c == '_'
    }) && !value.contains('<')
        && !value.contains('>')
        && !value.to_lowercase().contains("url(data:")
}

/// Sanitize a JSON value recursively
fn sanitize_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            if s.contains('<') || s.contains('>') || s.to_lowercase().contains("url(data:") {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(s)
            }
        }
        serde_json::Value::Object(obj) => {
            let sanitized: serde_json::Map<String, serde_json::Value> = obj
                .into_iter()
                .filter(|(k, _)| is_safe_key(k))
                .map(|(k, v)| (k, sanitize_value(v)))
                .collect();
            serde_json::Value::Object(sanitized)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(sanitize_value).collect())
        }
        other => other,
    }
}

/// Normalize JSON by converting single quotes to double quotes
/// This handles mermaid.js style JSON which often uses single quotes
fn normalize_json(json: &str) -> String {
    let mut result = String::with_capacity(json.len());
    let mut in_double_quote = false;

    for c in json.chars() {
        match c {
            '"' => {
                in_double_quote = !in_double_quote;
                result.push(c);
            }
            '\'' if !in_double_quote => {
                // Convert single quote to double quote when not inside a double-quoted string
                result.push('"');
            }
            _ => result.push(c),
        }
    }

    result
}

/// Remove directives from diagram text
///
/// Returns the text with all `%%{...}%%` directives stripped out.
pub fn remove_directives(text: &str) -> String {
    DIRECTIVE_REGEX.replace_all(text, "").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_init_with_theme() {
        let text = r#"%%{init: {"theme": "forest"}}%%
flowchart TD
    A --> B"#;

        let config = detect_init(text).expect("Should parse directive");
        assert_eq!(config.theme, Some("forest".to_string()));
    }

    #[test]
    fn test_detect_init_with_single_quotes() {
        let text = r#"%%{init: {'theme': 'dark'}}%%
flowchart TD
    A --> B"#;

        let config = detect_init(text).expect("Should parse directive");
        assert_eq!(config.theme, Some("dark".to_string()));
    }

    #[test]
    fn test_detect_init_with_theme_variables() {
        let text = r##"%%{init: {"theme": "default", "themeVariables": {"primaryColor": "#ff0000", "lineColor": "#00ff00"}}}%%
flowchart TD
    A --> B"##;

        let config = detect_init(text).expect("Should parse directive");
        assert_eq!(config.theme, Some("default".to_string()));
        assert_eq!(
            config.theme_variables.get("primaryColor"),
            Some(&"#ff0000".to_string())
        );
        assert_eq!(
            config.theme_variables.get("lineColor"),
            Some(&"#00ff00".to_string())
        );
    }

    #[test]
    fn test_sanitize_rejects_xss() {
        let text =
            r#"%%{init: {"themeVariables": {"primaryColor": "<script>alert(1)</script>"}}}%%"#;

        let config = detect_init(text).expect("Should parse directive");
        // XSS value should be filtered out
        assert!(config.theme_variables.get("primaryColor").is_none());
    }

    #[test]
    fn test_sanitize_rejects_prototype_pollution() {
        let text = r#"%%{init: {"__proto__": {"polluted": true}}}%%"#;

        let config = detect_init(text).expect("Should parse directive");
        // Prototype pollution keys should be filtered out
        assert!(config.extra.get("__proto__").is_none());
    }

    #[test]
    fn test_remove_directives() {
        let text = r#"%%{init: {"theme": "forest"}}%%
flowchart TD
    A --> B"#;

        let cleaned = remove_directives(text);
        assert!(!cleaned.contains("%%{"));
        assert!(cleaned.contains("flowchart TD"));
    }

    #[test]
    fn test_multiple_init_directives_merge() {
        let text = r##"%%{init: {"theme": "default"}}%%
%%{init: {"themeVariables": {"primaryColor": "#ff0000"}}}%%
flowchart TD
    A --> B"##;

        let config = detect_init(text).expect("Should parse directives");
        assert_eq!(config.theme, Some("default".to_string()));
        assert_eq!(
            config.theme_variables.get("primaryColor"),
            Some(&"#ff0000".to_string())
        );
    }

    #[test]
    fn test_no_directive_returns_none() {
        let text = r#"flowchart TD
    A --> B"#;

        assert!(detect_init(text).is_none());
    }

    #[test]
    fn test_theme_css_parsing() {
        let text = r#"%%{init: {"themeCSS": ".node rect { rx: 10; }"}}%%
flowchart TD
    A --> B"#;

        let config = detect_init(text).expect("Should parse directive");
        assert_eq!(config.theme_css, Some(".node rect { rx: 10; }".to_string()));
    }

    #[test]
    fn test_valid_css_values() {
        assert!(is_safe_css_value("ff0000"));
        assert!(is_safe_css_value("rgb(255, 0, 0)"));
        assert!(is_safe_css_value("transparent"));
        assert!(is_safe_css_value("1px solid black"));
        assert!(!is_safe_css_value("<script>"));
        assert!(!is_safe_css_value("url(data:text/html,<script>)"));
    }
}
