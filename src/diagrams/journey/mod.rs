//! User journey diagram support
//!
//! This module provides data structures for user journey diagrams.
//! User journey diagrams show user experiences as a series of tasks
//! with satisfaction scores and actors involved.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{JourneyDb, JourneyTask};
