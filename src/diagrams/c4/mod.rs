//! C4 diagram support
//!
//! This module provides data structures for C4 diagrams.
//! C4 diagrams show software architecture using the C4 model.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{C4Boundary, C4Db, C4Element, C4Relationship, C4ShapeType};
