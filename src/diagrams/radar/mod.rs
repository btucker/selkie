//! Radar diagram support
//!
//! This module provides data structures for radar (spider/web) diagrams.
//! Radar diagrams show multivariate data plotted on axes radiating from a center point.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{Graticule, RadarAxis, RadarCurve, RadarDb, RadarEntry, RadarOptions};
