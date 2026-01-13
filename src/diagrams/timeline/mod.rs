//! Timeline diagram support
//!
//! This module provides data structures for timeline diagrams.
//! Timeline diagrams show events organized by time periods or sections.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{TimelineDb, TimelineTask};
