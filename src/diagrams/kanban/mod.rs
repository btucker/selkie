//! Kanban diagram support
//!
//! This module provides data structures for kanban board diagrams.
//! Kanban diagrams show work items organized in columns/sections.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{KanbanData, KanbanDb, KanbanNode, NodeShape, Priority};
