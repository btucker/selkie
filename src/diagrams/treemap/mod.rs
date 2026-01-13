//! Treemap diagram support
//!
//! This module provides data structures for treemap diagrams.
//! Treemap diagrams show hierarchical data as nested rectangles.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{build_hierarchy, StyleClass, TreemapDb, TreemapNode};
