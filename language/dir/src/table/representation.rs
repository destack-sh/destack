use std::sync::Arc;

use destack_core::{FxIndexMap as IndexMap, FxIndexSet as IndexSet};
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, GlobalTypeId, SegmentView, Space};

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

    /// Return whether one nominal declaration derives Copy, holding when its stored values do.
    pub fn derives_copy(&self, symbol: GlobalSymbolId) -> Option<bool> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.derives_copy(symbol))
    }

    /// Return whether values of one visited type copy.
    pub fn copies(&self, ty: GlobalTypeId) -> Option<bool> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.copies(ty))
    }

    /// Return the space one nominal declaration's instances live in.
    pub fn space(&self, symbol: GlobalSymbolId) -> Option<Space> {
        self.segments
            .iter()
            .rev()
            .find_map(|segment| segment.space(symbol))
    }

    /// Return whether one class's constructor escapes `this`.
    pub fn escapes_this(&self, symbol: GlobalSymbolId) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.escapes_this(symbol))
    }
}

/// The layout policies the check pass commits for one module's nominal declarations.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct RepresentationSegment {
    /// The module id of the representation segment.
    pub module_id: ModuleId,
    /// Whether each nominal declaration derives Copy, holding when its stored values do.
    copy_derivations: IndexMap<GlobalSymbolId, bool>,
    /// Whether values of each visited type copy.
    copies: IndexMap<GlobalTypeId, bool>,
    /// The space each placed nominal declaration's instances live in.
    spaces: IndexMap<GlobalSymbolId, Space>,
    /// The classes whose constructors let `this` escape.
    this_escapes: IndexSet<GlobalSymbolId>,
}

impl RepresentationSegment {
    /// Create an empty representation segment.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            copy_derivations: IndexMap::default(),
            copies: IndexMap::default(),
            spaces: IndexMap::default(),
            this_escapes: IndexSet::default(),
        }
    }

    /// Set whether one nominal declaration derives Copy.
    pub fn set_derives_copy(&mut self, symbol: GlobalSymbolId, derives_copy: bool) {
        self.copy_derivations.insert(symbol, derives_copy);
    }

    /// Return whether one nominal declaration derives Copy.
    pub fn derives_copy(&self, symbol: GlobalSymbolId) -> Option<bool> {
        self.copy_derivations.get(&symbol).copied()
    }

    /// Set whether values of one visited type copy.
    pub fn set_copies(&mut self, ty: GlobalTypeId, copies: bool) {
        self.copies.insert(ty, copies);
    }

    /// Return whether values of one visited type copy.
    pub fn copies(&self, ty: GlobalTypeId) -> Option<bool> {
        self.copies.get(&ty).copied()
    }

    /// Set the space one nominal declaration's instances live in.
    pub fn set_space(&mut self, symbol: GlobalSymbolId, space: Space) {
        self.spaces.insert(symbol, space);
    }

    /// Return the space one nominal declaration's instances live in.
    pub fn space(&self, symbol: GlobalSymbolId) -> Option<Space> {
        self.spaces.get(&symbol).copied()
    }

    /// Record that one class's constructor escapes `this`.
    pub fn set_escapes_this(&mut self, symbol: GlobalSymbolId) {
        self.this_escapes.insert(symbol);
    }

    /// Return whether one class's constructor escapes `this`.
    pub fn escapes_this(&self, symbol: GlobalSymbolId) -> bool {
        self.this_escapes.contains(&symbol)
    }

    /// Return whether the segment commits no policy.
    pub fn is_empty(&self) -> bool {
        self.copy_derivations.is_empty()
            && self.copies.is_empty()
            && self.spaces.is_empty()
            && self.this_escapes.is_empty()
    }
}
