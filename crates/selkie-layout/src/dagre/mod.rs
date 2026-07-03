//! Curated Dagre expert API.
//!
//! This module exposes stable graph construction and layout entrypoints without
//! making every internal algorithm phase part of the public API.

pub(crate) mod internal;

pub use internal::{Acyclicer, DagreConfig, RankDir, Ranker};

#[derive(Debug, Clone)]
pub struct DagreGraph {
    inner: internal::graph::DagreGraph,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DagrePoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct DagreNode<'a> {
    label: &'a internal::graph::NodeLabel,
}

#[derive(Debug, Clone)]
pub struct DagreEdge<'a> {
    label: &'a internal::graph::EdgeLabel,
}

impl DagreGraph {
    pub fn new() -> Self {
        Self {
            inner: internal::graph::DagreGraph::new(),
        }
    }

    pub fn set_node(&mut self, id: &str, width: f64, height: f64) {
        self.inner.set_node(
            id,
            internal::graph::NodeLabel {
                width,
                height,
                ..Default::default()
            },
        );
    }

    pub fn set_edge(&mut self, source: &str, target: &str) {
        self.inner
            .set_edge(source, target, internal::graph::EdgeLabel::default());
    }

    pub fn node(&self, id: &str) -> Option<DagreNode<'_>> {
        self.inner.node(id).map(|label| DagreNode { label })
    }

    pub fn edge(&self, source: &str, target: &str) -> Option<DagreEdge<'_>> {
        self.inner
            .edge(source, target)
            .map(|label| DagreEdge { label })
    }

    pub(crate) fn inner_mut(&mut self) -> &mut internal::graph::DagreGraph {
        &mut self.inner
    }
}

impl Default for DagreGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DagreNode<'_> {
    pub fn x(&self) -> Option<f64> {
        self.label.x
    }

    pub fn y(&self) -> Option<f64> {
        self.label.y
    }

    pub fn width(&self) -> f64 {
        self.label.width
    }

    pub fn height(&self) -> f64 {
        self.label.height
    }
}

impl DagreEdge<'_> {
    pub fn points(&self) -> Vec<DagrePoint> {
        self.label
            .points
            .iter()
            .map(|point| DagrePoint {
                x: point.x,
                y: point.y,
            })
            .collect()
    }
}

pub fn layout(graph: &mut DagreGraph, config: &DagreConfig) {
    internal::layout(graph.inner_mut(), config);
}
