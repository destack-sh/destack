use std::sync::Arc;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalNodeIdAny, ModuleEdge, ModuleRelation, SegmentView};

/// Cumulative module import edges for one profile-scoped DIR module.
#[derive(Debug, Clone)]
pub struct ModuleTable<'a> {
    /// The module id of the module table.
    pub module_id: ModuleId,
    /// The ordered module table segments.
    segments: SegmentView<'a, ModuleSegment>,
}

impl ModuleTable<'static> {
    /// Create a module table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<ModuleSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a module table from one segment.
    pub fn from_segment(segment: Arc<ModuleSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> ModuleTable<'a> {
    /// Create a module table from a segment view.
    pub fn from_view(segments: SegmentView<'a, ModuleSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("module table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "module table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a module table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b ModuleSegment) -> ModuleTable<'b> {
        ModuleTable::from_view(self.segments.with_tail(tail))
    }

    /// Iterate visible module import edges.
    pub fn iter(&self) -> impl Iterator<Item = &ModuleEdge> + '_ {
        self.segments.iter().flat_map(|segment| segment.iter())
    }

    /// Return whether the table has no module import edges.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }

    /// Return the latest edge for one import source and relation.
    pub fn edge_for_source(
        &self,
        source: GlobalNodeIdAny,
        relation: ModuleRelation,
    ) -> Option<&ModuleEdge> {
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

    /// Return the latest resolved module for one import source and relation.
    pub fn target_for_source(
        &self,
        source: GlobalNodeIdAny,
        relation: ModuleRelation,
    ) -> Option<ModuleId> {
        self.edge_for_source(source, relation)
            .and_then(|edge| edge.target)
    }
}

/// Module import edges added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSegment {
    /// The module id of the segment.
    pub module_id: ModuleId,
    /// Locally resolved module import edges.
    pub edges: Vec<ModuleEdge>,
}

impl ModuleSegment {
    /// Create an empty module segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            edges: Vec::new(),
        }
    }

    /// Create a module segment from resolved edges.
    pub fn from_edges(module_id: ModuleId, edges: Vec<ModuleEdge>) -> Self {
        Self { module_id, edges }
    }

    /// Return locally resolved module import edges.
    #[inline]
    pub fn as_slice(&self) -> &[ModuleEdge] {
        &self.edges
    }

    /// Iterate resolved module import edges.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &ModuleEdge> + '_ {
        self.edges.iter()
    }

    /// Add a resolved module import edge.
    #[inline]
    pub fn push(&mut self, edge: ModuleEdge) {
        self.edges.push(edge);
    }

    /// Return whether this segment has no resolved module import edges.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

impl<'a> IntoIterator for &'a ModuleSegment {
    type Item = &'a ModuleEdge;
    type IntoIter = std::slice::Iter<'a, ModuleEdge>;

    fn into_iter(self) -> Self::IntoIter {
        self.edges.iter()
    }
}
