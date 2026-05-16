use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{DependencyEdge, DependencyRelation, DependencyTarget, GlobalNodeIdAny};

/// Cumulative module dependency edges for one profile-scoped DIR module.
#[derive(Debug, Clone, Default)]
pub struct DependencyTable {
    /// The ordered dependency table segments.
    segments: Vec<Arc<DependencySegment>>,
}

impl DependencyTable {
    /// Create a dependency table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<DependencySegment>>) -> Self {
        Self { segments }
    }

    /// Create a dependency table from one segment.
    pub fn from_segment(segment: Arc<DependencySegment>) -> Self {
        Self::from_segments(vec![segment])
    }

    /// Iterate visible dependency edges.
    pub fn iter(&self) -> impl Iterator<Item = &DependencyEdge> + '_ {
        self.segments.iter().flat_map(|segment| segment.iter())
    }

    /// Return whether the table has no dependency edges.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }

    /// Return the latest target for one dependency source and relation.
    pub fn target_for_source(
        &self,
        source: GlobalNodeIdAny,
        relation: DependencyRelation,
    ) -> Option<DependencyTarget> {
        // search later table segments first
        for segment in self.segments.iter().rev() {
            for edge in segment.edges.iter().rev() {
                if edge.source == source && edge.relation == relation {
                    return Some(edge.target);
                }
            }
        }

        None
    }
}

/// Module dependency edges added by one DIR phase.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencySegment {
    /// Locally resolved dependency edges.
    pub edges: Vec<DependencyEdge>,
}

impl DependencySegment {
    /// Create an empty dependency segment.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a dependency segment from resolved edges.
    pub fn from_edges(edges: Vec<DependencyEdge>) -> Self {
        Self { edges }
    }

    /// Return locally resolved dependency edges.
    #[inline]
    pub fn as_slice(&self) -> &[DependencyEdge] {
        &self.edges
    }

    /// Iterate resolved dependency edges.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &DependencyEdge> + '_ {
        self.edges.iter()
    }

    /// Add a resolved dependency edge.
    #[inline]
    pub fn push(&mut self, edge: DependencyEdge) {
        self.edges.push(edge);
    }

    /// Return whether this table has no resolved dependency edges.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

impl<'a> IntoIterator for &'a DependencySegment {
    type Item = &'a DependencyEdge;
    type IntoIter = std::slice::Iter<'a, DependencyEdge>;

    fn into_iter(self) -> Self::IntoIter {
        self.edges.iter()
    }
}
