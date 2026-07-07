//! Error types for selkie

use thiserror::Error;

/// Main error type for selkie operations
#[derive(Error, Debug)]
pub enum MermaidError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Unknown diagram type: {0}")]
    UnknownDiagramType(String),

    #[error("Invalid value: {message}")]
    InvalidValue { message: String },

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Layout error: {0}")]
    LayoutError(String),

    #[error("Render error: {0}")]
    RenderError(String),
}

impl From<String> for MermaidError {
    fn from(s: String) -> Self {
        MermaidError::ParseError(s)
    }
}

impl From<Box<dyn std::error::Error>> for MermaidError {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        MermaidError::ParseError(e.to_string())
    }
}

impl From<selkie_layout::LayoutError> for MermaidError {
    fn from(error: selkie_layout::LayoutError) -> Self {
        MermaidError::LayoutError(error.to_string())
    }
}

/// Result type alias for mermaid operations
pub type Result<T> = std::result::Result<T, MermaidError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_layout_error_to_mermaid_error() {
        let error =
            MermaidError::from(selkie_layout::LayoutError::DuplicateNodeId("A".to_string()));

        match error {
            MermaidError::LayoutError(message) => {
                assert_eq!(message, "Duplicate node id: A");
            }
            other => panic!("expected layout error, got {other:?}"),
        }
    }
}
