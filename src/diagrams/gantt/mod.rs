//! Gantt diagram support
//!
//! This module provides parsing and data structures for Gantt chart diagrams.
//! Gantt charts show project schedules with tasks, durations, and dependencies.

pub mod parser;
mod types;

pub use parser::parse;
pub use types::{DisplayMode, Duration, DurationUnit, GanttDb, Task, TaskFlags, WeekendStart};
