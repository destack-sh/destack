use std::sync::Arc;

use destack_core::FxIndexMap as IndexMap;
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, SegmentView, Space};

/// Cumulative layout policies for one DIR module.
#[derive(Debug, Clone)]
pub struct RepresentationTable<'a> {
    /// The module id of the representation table.
    pub module_id: ModuleId,
    /// The ordered representation table segments.
    segments: SegmentView<'a, RepresentationSegment>,
}

impl RepresentationTable<'static> {
    /// Create a representation table from ordered segments.
    pub fn from_segments(segments: Vec<Arc<RepresentationSegment>>) -> Self {
        Self::from_view(SegmentView::from_segments(segments))
    }
}

impl<'a> RepresentationTable<'a> {
    /// Create a representation table from a segment view.
    pub fn from_view(segments: SegmentView<'a, RepresentationSegment>) -> Self {
        let first = segments
            .first()
            .unwrap_or_else(|| panic!("representation table needs at least one segment"));

        Self {
            module_id: first.module_id,
            segments,
        }
    }

    /// Return whether one nominal declaration permits copying by structure.
    pub fn copies(&self, symbol: GlobalSymbolId) -> Option<bool> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.copies(symbol))
    }

    /// Return the space one nominal declaration's instances live in.
    pub fn space(&self, symbol: GlobalSymbolId) -> Option<Space> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.space(symbol))
    }
}

/// The layout policies the check pass commits for one module's nominal declarations.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct RepresentationSegment {
    /// The module id of the representation segment.
    pub module_id: ModuleId,
    /// Whether each nominal declaration permits copying by structure.
    copies: IndexMap<GlobalSymbolId, bool>,
    /// The space each placed nominal declaration's instances live in.
    spaces: IndexMap<GlobalSymbolId, Space>,
}

impl RepresentationSegment {
    /// Create an empty representation segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            copies: IndexMap::default(),
            spaces: IndexMap::default(),
        }
    }

    /// Set whether one nominal declaration permits copying by structure.
    pub fn set_copies(&mut self, symbol: GlobalSymbolId, copies: bool) {
        self.copies.insert(symbol, copies);
    }

    /// Return whether one nominal declaration permits copying by structure.
    pub fn copies(&self, symbol: GlobalSymbolId) -> Option<bool> {
        self.copies.get(&symbol).copied()
    }

    /// Set the space one nominal declaration's instances live in.
    pub fn set_space(&mut self, symbol: GlobalSymbolId, space: Space) {
        self.spaces.insert(symbol, space);
    }

    /// Return the space one nominal declaration's instances live in.
    pub fn space(&self, symbol: GlobalSymbolId) -> Option<Space> {
        self.spaces.get(&symbol).copied()
    }

    /// Return whether the segment commits no policy.
    pub fn is_empty(&self) -> bool {
        self.copies.is_empty() && self.spaces.is_empty()
    }
}
