//! Curated Dagre expert API.
//!
//! This module exposes stable graph construction and layout entrypoints without
//! making every internal algorithm phase part of the public API.

pub(crate) mod internal;

pub use internal::{Acyclicer, RankDir};

use crate::Point;

/// Configuration for the Dagre layout algorithm.
#[derive(Debug, Clone)]
pub struct DagreConfig {
    /// Direction of the layout: TB, BT, LR, RL.
    pub rankdir: RankDir,
    /// Separation between nodes on the same rank.
    pub nodesep: f64,
    /// Separation between edges.
    pub edgesep: f64,
    /// Separation between ranks.
    pub ranksep: f64,
    /// Horizontal margin around the graph.
    pub marginx: f64,
    /// Vertical margin around the graph.
    pub marginy: f64,
    /// Method for breaking cycles.
    pub acyclicer: Acyclicer,
    /// Method for assigning ranks.
    pub ranker: Ranker,
}

impl Default for DagreConfig {
    fn default() -> Self {
        Self {
            rankdir: RankDir::TB,
            nodesep: 50.0,
            edgesep: 20.0,
            ranksep: 50.0,
            marginx: 0.0,
            marginy: 0.0,
            acyclicer: Acyclicer::Greedy,
            ranker: Ranker::NetworkSimplex,
        }
    }
}

impl DagreConfig {
    fn to_internal(&self) -> internal::DagreConfig {
        internal::DagreConfig {
            rankdir: self.rankdir,
            nodesep: self.nodesep,
            edgesep: self.edgesep,
            ranksep: self.ranksep,
            marginx: self.marginx,
            marginy: self.marginy,
            acyclicer: self.acyclicer,
            ranker: match self.ranker {
                Ranker::NetworkSimplex => internal::Ranker::NetworkSimplex,
                Ranker::LongestPath => internal::Ranker::LongestPath,
            },
        }
    }
}

/// Stable public ranking algorithms supported by the expert API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ranker {
    /// Network simplex algorithm.
    NetworkSimplex,
    /// Longest-path rank assignment.
    LongestPath,
}

#[derive(Debug, Clone)]
pub struct DagreGraph {
    inner: internal::graph::DagreGraph,
}

pub type DagrePoint = Point;

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
            .map(|point| DagrePoint::new(point.x, point.y))
            .collect()
    }
}

pub fn layout(graph: &mut DagreGraph, config: &DagreConfig) {
    internal::layout(graph.inner_mut(), &config.to_internal());
}
