//! Compatibility facade for layout APIs.
//!
//! Core layout types and algorithms live in the `selkie-layout` crate. Selkie
//! keeps diagram adapter and size-estimation helpers here because they depend on
//! Selkie parser and renderer behavior.

mod adapter;
mod size;

pub use adapter::{NodeSizeConfig, SizeEstimator, ToLayoutGraph};
pub use selkie_layout::*;
pub use size::{create_size_estimator, CharacterSizeEstimator, FontdueSizeEstimator};
