//! Block diagram support
//!
//! This module provides data structures for block diagrams.
//! Block diagrams show blocks/nodes and their connections in a grid layout.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{Block, BlockDb, BlockType, ClassDef, Edge};
