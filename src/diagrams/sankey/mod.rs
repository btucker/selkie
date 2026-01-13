//! Sankey diagram support
//!
//! This module provides data structures for Sankey diagrams.
//! Sankey diagrams show flow/movement between nodes with weighted connections.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{GraphLink, GraphNode, SankeyDb, SankeyGraph, SankeyLink, SankeyNode};
