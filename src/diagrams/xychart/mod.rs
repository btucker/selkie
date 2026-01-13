//! XY Chart diagram support
//!
//! This module provides data structures for XY Chart diagrams.
//! XY charts show data on a 2D coordinate system with line and bar plots.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{
    AxisType, BandAxisData, ChartOrientation, DataPoint, LinearAxisData, Plot, PlotType, XAxisData,
    XYChartDb, YAxisData,
};
