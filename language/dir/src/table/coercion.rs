use std::sync::Arc;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{CastOrigin, GlobalNodeIdAny, GlobalTypeId, SegmentView};

/// Cumulative checked coercions for one DIR module.
#[derive(Debug, Clone)]
pub struct CoercionTable<'a> {
    /// The module id of the coercion table.
    pub module_id: ModuleId,
    /// The ordered coercion table segments.
    segments: SegmentView<'a, CoercionSegment>,
}

impl CoercionTable<'static> {
    /// Create a coercion table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<CoercionSegment>>) -> Self {
        let segments = SegmentView::from_segments(segments);

        Self::from_view(segments)
    }

    /// Create a coercion table from one segment.
    pub fn from_segment(segment: Arc<CoercionSegment>) -> Self {
        Self::from_segments(vec![segment])
    }
}

impl<'a> CoercionTable<'a> {
    /// Create a coercion table from a segment view.
    pub fn from_view(segments: SegmentView<'a, CoercionSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("coercion table needs at least one segment"));
        let module_id = first.module_id;

        // require a single module owner
        for segment in segments.iter() {
            assert_eq!(
                segment.module_id, module_id,
                "coercion table segment belongs to a different module"
            );
        }

        Self {
            module_id,
            segments,
        }
    }

    /// Create a coercion table by appending a borrowed tail segment.
    pub fn with_tail<'b>(&'b self, tail: &'b CoercionSegment) -> CoercionTable<'b> {
        CoercionTable::from_view(self.segments.with_tail(tail))
    }

    /// Get the effective coercion for one node.
    pub fn coercion(&self, node_id: GlobalNodeIdAny) -> Option<Coercion> {
        for segment in self.segments.iter().rev() {
            if let Some(coercion) = segment.coercion(node_id) {
                return Some(coercion);
            }
        }

        None
    }

    /// Iterate visible coercions.
    pub fn coercions(&self) -> impl Iterator<Item = (GlobalNodeIdAny, Coercion)> + '_ {
        self.segments
            .iter()
            .enumerate()
            .flat_map(move |(segment_index, segment)| {
                segment
                    .coercions
                    .iter()
                    .filter_map(move |(node_id, coercion)| {
                        let is_shadowed = self
                            .segments
                            .iter()
                            .skip(segment_index + 1)
                            .any(|segment| segment.coercions.contains_key(node_id));

                        (!is_shadowed).then_some((*node_id, *coercion))
                    })
            })
    }

    /// Return whether this table has no coercions.
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.is_empty())
    }
}

/// One type coercion attached to a value node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coercion {
    /// The source type before coercion.
    pub source: GlobalTypeId,
    /// The target type after coercion.
    pub target: GlobalTypeId,
    /// How the coercion entered DIR.
    pub origin: CastOrigin,
}

impl Coercion {
    /// Create one coercion.
    pub fn new(source: GlobalTypeId, target: GlobalTypeId, origin: CastOrigin) -> Self {
        Self {
            source,
            target,
            origin,
        }
    }
}

/// Coercions added by one DIR phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoercionSegment {
    /// The module id of the coercion segment.
    pub module_id: ModuleId,
    /// Coercions keyed by the value node being coerced.
    pub(crate) coercions: IndexMap<GlobalNodeIdAny, Coercion>,
}

impl CoercionSegment {
    /// Create an empty coercion segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            coercions: IndexMap::new(),
        }
    }

    /// Bind the coercion for one value node.
    pub fn bind_coercion(
        &mut self,
        node_id: GlobalNodeIdAny,
        coercion: Coercion,
    ) -> Option<Coercion> {
        self.coercions.insert(node_id, coercion)
    }

    /// Get the coercion for one value node.
    pub fn coercion(&self, node_id: GlobalNodeIdAny) -> Option<Coercion> {
        self.coercions.get(&node_id).copied()
    }

    /// Iterate coercions in insertion order.
    pub fn coercions(&self) -> impl Iterator<Item = (GlobalNodeIdAny, Coercion)> + '_ {
        self.coercions
            .iter()
            .map(|(node_id, coercion)| (*node_id, *coercion))
    }

    /// Return whether this segment has no coercions.
    pub fn is_empty(&self) -> bool {
        self.coercions.is_empty()
    }
}
