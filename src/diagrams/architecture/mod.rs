//! Architecture diagram support
//!
//! This module provides data structures for architecture diagrams.
//! Architecture diagrams show software architecture with services, groups, and connections.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{
    get_direction_alignment, ArchitectureAlignment, ArchitectureDb, ArchitectureDirection,
    ArchitectureEdge, ArchitectureError, ArchitectureGroup, ArchitectureJunction, ArchitectureNode,
    ArchitectureService, DirectionPair, RegistryEntry,
};
