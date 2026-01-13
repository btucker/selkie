//! Requirement diagram support
//!
//! This module provides data structures for requirement diagrams.
//! Requirement diagrams show requirements, elements, and their relationships.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{
    Element, Relation, RelationshipType, Requirement, RequirementClass, RequirementDb,
    RequirementType, RiskLevel, VerifyType,
};
