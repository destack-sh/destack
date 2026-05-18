use std::sync::Arc;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{DependencyEdge, DependencyRelation, GlobalNodeIdAny, SegmentView};

/// Cumulative module dependency edges for one profile-scoped DIR module.
#[derive(Debug, Clone)]
pub struct DependencyTable<'a> {
    /// The module id of the dependency table.
    pub module_id: ModuleId,
    /// The ordered dependency table segments.
    segments: SegmentView<'a, DependencySegment>,
}

impl DependencyTable<'static> {
    /// Create a dependency table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<DependencySegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a dependency table from one segment.
    pub fn from_segment(segment: Arc<DependencySegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> DependencyTable<'a> {
    /// Create a dependency table from a segment view.
    pub fn from_view(segments: SegmentView<'a, DependencySegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("dependency table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "dependency table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a dependency table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b DependencySegment) -> DependencyTable<'b> {
        DependencyTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate visible dependency edges.
    pub fn iter(&self) -> impl Iterator<Item = &DependencyEdge> + '_ {
        self.segments.iter().flat_map(|segment| segment.iter())
    }

    /// Return whether the table has no dependency edges.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }

    /// Return the latest edge for one dependency source and relation.
    pub fn edge_for_source(
        &self,
        source: GlobalNodeIdAny,
        relation: DependencyRelation,
    ) -> Option<&DependencyEdge> {
        // search later table segments first
        for segment in self.segments.iter().rev() {
            for edge in segment.edges.iter().rev() {
                if edge.source == source && edge.relation == relation {
                    return Some(edge);
                }
            }
        }

        None
    }

    /// Return the latest resolved module for one dependency source and relation.
    pub fn target_for_source(
        &self,
        source: GlobalNodeIdAny,
        relation: DependencyRelation,
    ) -> Option<ModuleId> {
        self.edge_for_source(source, relation)
            .and_then(|edge| edge.target)
    }
}

/// Module dependency edges added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySegment {
    /// The module id of the dependency segment.
    pub module_id: ModuleId,
    /// Locally resolved dependency edges.
    pub edges: Vec<DependencyEdge>,
}

impl DependencySegment {
    /// Create an empty dependency segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            edges: Vec::new(),
        }
    }

    /// Create a dependency segment from resolved edges.
    pub fn from_edges(module_id: ModuleId, edges: Vec<DependencyEdge>) -> Self {
        Self { module_id, edges }
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
