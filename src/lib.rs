//! selkie - A Rust port of mermaid.js
//!
//! This library provides parsing, layout, and rendering for mermaid diagram syntax.
//! It supports multiple diagram types including flowcharts, sequence diagrams,
//! pie charts, and more.

pub mod common;
pub mod config;
pub mod diagrams;
pub mod error;
#[cfg(feature = "eval")]
pub mod eval;
pub mod layout;
pub mod render;

#[cfg(feature = "kitty")]
pub mod kitty;
#[cfg(feature = "wasm")]
pub mod wasm;

pub use config::Config;
pub use error::{MermaidError, Result};
pub use render::{
    render, render_ascii, render_text, render_text_ascii, render_with_config, RenderConfig, Theme,
};

/// Parse a mermaid diagram and return a diagram representation
pub fn parse(input: &str) -> Result<diagrams::Diagram> {
    let diagram_type = diagrams::detect_type(input)?;
    diagrams::parse(diagram_type, input)
}
