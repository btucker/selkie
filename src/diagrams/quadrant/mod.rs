//! Quadrant chart diagram support
//!
//! This module provides data structures for quadrant chart diagrams.
//! Quadrant charts divide data into four quadrants based on two axes,
//! commonly used for prioritization matrices.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{ClassDef, PointStyle, QuadrantDb, QuadrantPoint};
